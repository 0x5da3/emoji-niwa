// Dev-only WebSocket load harness for the emoji-niwa relay server.
//
// The server is a dumb relay: a client sends {t:'snap',d:<opaque string>} and
// the server stores the room's latest `d` and fans it out to every OTHER
// connection (server/src/main.rs:442/463). We make `d` itself a small JSON
// envelope carrying {rid,sid,seq,ts} so receivers can measure fan-out latency
// and detect dropped snapshots (broadcast channel cap = 64 → Lagged).
//
// Assumes: the server is already running AND the target rooms are seeded
// (see seed-rooms.mjs). Standalone: `node tests/load/ws-load.mjs`.
// Orchestrated end-to-end: `node tests/load/run.mjs`.
//
// Knobs (env): WS_BASE ORIGIN ROOMS PEERS ROOM_PREFIX SNAP_MS SNAP_BYTES
//              DURATION_MS RAMP_MS STABILIZE_MS DRAIN_MS SERVER_PID

import { WebSocket } from 'ws';
import { readFileSync } from 'node:fs';

const env = (k, d) => process.env[k] ?? d;
const int = (k, d) => parseInt(env(k, String(d)), 10);

const WS_BASE = env('WS_BASE', 'ws://127.0.0.1:8080');
const ORIGIN = env('ORIGIN', 'http://localhost:8000');
const ROOMS = int('ROOMS', 1);
const PEERS = int('PEERS', 8);
const ROOM_PREFIX = env('ROOM_PREFIX', 'load-');
const SNAP_MS = int('SNAP_MS', 320); // matches the app's send debounce
const SNAP_BYTES = int('SNAP_BYTES', 4096);
const DURATION_MS = int('DURATION_MS', 15000);
const RAMP_MS = int('RAMP_MS', 5);
const STABILIZE_MS = int('STABILIZE_MS', 1200);
const DRAIN_MS = int('DRAIN_MS', 700);
const SERVER_PID = env('SERVER_PID', '');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const maxOf = (a) => a.reduce((m, x) => (x > m ? x : m), 0);
// quantiles off a single pre-sorted array (avoids re-sorting 100k+ samples)
const q = (sorted, p) =>
  sorted.length ? sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))] : 0;

// Pre-size the snap envelope to ~SNAP_BYTES.
function makeSnap(rid, sid, seq) {
  const base = { _lt: 1, rid, sid, seq, ts: Date.now(), pad: '' };
  const padLen = Math.max(0, SNAP_BYTES - JSON.stringify(base).length);
  base.pad = 'A'.repeat(padLen);
  base.ts = Date.now();
  return JSON.stringify(base);
}

// ── server resource sampling via /proc (Linux) ──────────────────────────────
let cpuStart = null;
let rssPeakKb = 0;
const clk = 100; // USER_HZ: 100 on effectively all Linux (getconf CLK_TCK)
function readProc() {
  if (!SERVER_PID) return null;
  try {
    const stat = readFileSync(`/proc/${SERVER_PID}/stat`, 'utf8');
    const rp = stat.lastIndexOf(')');
    const f = stat.slice(rp + 2).split(' ');
    const utime = +f[11];
    const stime = +f[12]; // fields 14/15 (1-based) → indices 11/12 after comm
    const status = readFileSync(`/proc/${SERVER_PID}/status`, 'utf8');
    const m = status.match(/VmRSS:\s+(\d+)\s+kB/);
    const rssKb = m ? +m[1] : 0;
    if (rssKb > rssPeakKb) rssPeakKb = rssKb;
    return { ticks: utime + stime, rssKb };
  } catch {
    return null;
  }
}

// ── per-connection peer ─────────────────────────────────────────────────────
class Peer {
  constructor(roomIdx, peerIdx) {
    this.roomIdx = roomIdx;
    this.peerIdx = peerIdx;
    this.roomId = `${ROOM_PREFIX}${roomIdx}`;
    this.ws = null;
    this.connected = false;
    this.failed = false;
    this.role = null; // last {t:'role',owner}
    this.maxPeers = 0; // max {t:'peers',n} seen
    this.sentSeq = 0;
    this.recv = 0;
    this.latencies = []; // ms, only for snaps sent within the measure window
    this.lastSeqFrom = new Map(); // `${rid}:${sid}` -> last seq (gap = server drop)
    this.gaps = 0;
    this.sendersSeen = new Set();
  }
  open() {
    return new Promise((resolve) => {
      let ws;
      try {
        ws = new WebSocket(`${WS_BASE}/room/${this.roomId}`, { origin: ORIGIN });
      } catch (e) {
        this.failed = true;
        return resolve();
      }
      this.ws = ws;
      const to = setTimeout(() => {
        if (!this.connected) {
          this.failed = true;
          try { ws.terminate(); } catch {}
          resolve();
        }
      }, 15000);
      ws.on('open', () => {
        this.connected = true;
        clearTimeout(to);
        resolve();
      });
      ws.on('message', (buf) => this.onMsg(buf));
      ws.on('error', () => {
        if (!this.connected) {
          this.failed = true;
          clearTimeout(to);
          resolve();
        }
      });
      ws.on('close', () => {
        this.connected = false;
      });
    });
  }
  onMsg(buf) {
    let m;
    try { m = JSON.parse(buf.toString()); } catch { return; }
    if (m.t === 'peers') {
      if (m.n > this.maxPeers) this.maxPeers = m.n;
    } else if (m.t === 'role') {
      this.role = !!m.owner;
    } else if (m.t === 'snap' && typeof m.d === 'string') {
      let p;
      try { p = JSON.parse(m.d); } catch { return; }
      if (p._lt !== 1) return;
      this.recv++;
      this.sendersSeen.add(`${p.rid}:${p.sid}`);
      const key = `${p.rid}:${p.sid}`;
      const last = this.lastSeqFrom.get(key);
      if (last != null && p.seq > last + 1) this.gaps += p.seq - last - 1;
      if (last == null || p.seq > last) this.lastSeqFrom.set(key, p.seq);
      if (p.ts >= this._windowStart) this.latencies.push(Date.now() - p.ts);
    }
  }
  startSending() {
    this._windowStart = Date.now();
    this._timer = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        const d = makeSnap(this.roomId, this.peerIdx, ++this.sentSeq);
        try { this.ws.send(JSON.stringify({ t: 'snap', d })); } catch {}
      }
    }, SNAP_MS);
  }
  stopSending() {
    clearInterval(this._timer);
  }
  close() {
    try { this.ws && this.ws.close(); } catch {}
  }
}

async function main() {
  console.log(
    `[load] ${ROOMS} room(s) x ${PEERS} peer(s) = ${ROOMS * PEERS} conns | ` +
      `snap ${SNAP_BYTES}B every ${SNAP_MS}ms | ${DURATION_MS}ms | ${WS_BASE} (origin ${ORIGIN})`,
  );

  const peers = [];
  for (let r = 0; r < ROOMS; r++)
    for (let p = 0; p < PEERS; p++) peers.push(new Peer(r, p));

  // Ramp connections.
  const t0 = Date.now();
  for (const pr of peers) {
    pr.open();
    if (RAMP_MS > 0) await sleep(RAMP_MS);
  }
  // Wait until everyone is open or failed.
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline && peers.some((p) => !p.connected && !p.failed))
    await sleep(50);
  const ok = peers.filter((p) => p.connected).length;
  const failed = peers.length - ok;
  console.log(`[load] connected ${ok}/${peers.length} (failed ${failed}) in ${Date.now() - t0}ms`);

  await sleep(STABILIZE_MS); // let peers/role settle after the join storm

  const baseProc = readProc();
  cpuStart = baseProc ? baseProc.ticks : null;
  const sampler = setInterval(readProc, 500);

  for (const p of peers) p.startSending();
  await sleep(DURATION_MS);
  for (const p of peers) p.stopSending();
  await sleep(DRAIN_MS); // collect in-flight fan-out
  clearInterval(sampler);
  const endProc = readProc();

  // ── health assertions (true N-player play) ────────────────────────────────
  const fails = [];
  if (failed > 0) fails.push(`${failed} connection(s) failed`);
  for (let r = 0; r < ROOMS; r++) {
    const rp = peers.filter((p) => p.roomIdx === r && p.connected);
    if (rp.length !== PEERS) {
      fails.push(`room ${r}: ${rp.length}/${PEERS} connected`);
      continue;
    }
    if (!rp.every((p) => p.maxPeers === PEERS))
      fails.push(`room ${r}: not all peers saw peers=${PEERS} (max ${maxOf(rp.map((p) => p.maxPeers))})`);
    const owners = rp.filter((p) => p.role === true).length;
    if (owners !== 1) fails.push(`room ${r}: expected exactly 1 owner, got ${owners}`);
    const sawAll = rp.every((p) => p.sendersSeen.size >= PEERS - 1);
    if (!sawAll)
      fails.push(`room ${r}: some peer did not receive all ${PEERS - 1} other senders`);
  }

  // ── metrics ───────────────────────────────────────────────────────────────
  const allLat = peers.flatMap((p) => p.latencies).sort((a, b) => a - b);
  const sent = peers.reduce((a, p) => a + p.sentSeq, 0);
  const recv = peers.reduce((a, p) => a + p.recv, 0);
  const gaps = peers.reduce((a, p) => a + p.gaps, 0);
  const secs = DURATION_MS / 1000;

  console.log('\n──────── results ────────');
  console.log(`connections      : ${ok} ok / ${failed} failed`);
  console.log(`snaps sent       : ${sent}  (${(sent / secs).toFixed(0)}/s)`);
  console.log(`snaps received   : ${recv}  (${(recv / secs).toFixed(0)}/s)`);
  console.log(
    `fan-out latency  : p50 ${q(allLat, 50)}ms  p90 ${q(allLat, 90)}ms  ` +
      `p99 ${q(allLat, 99)}ms  max ${maxOf(allLat)}ms  (n=${allLat.length})`,
  );
  console.log(
    `dropped snaps    : ${gaps}  (seq gaps = broadcast Lagged / cap ${64}); ` +
      `${recv ? ((100 * gaps) / (recv + gaps)).toFixed(2) : '0'}% loss`,
  );
  if (baseProc && endProc) {
    const cpuPct = (100 * ((endProc.ticks - cpuStart) / clk)) / secs;
    console.log(
      `server CPU       : ${cpuPct.toFixed(1)}% (1 core = 100%) over the window`,
    );
    console.log(`server RSS peak  : ${(rssPeakKb / 1024).toFixed(1)} MB`);
  } else if (SERVER_PID) {
    console.log('server CPU/RSS   : (could not read /proc — wrong SERVER_PID?)');
  }
  console.log('health           : ' + (fails.length ? 'FAIL' : 'PASS'));
  if (fails.length) for (const f of fails) console.log('  - ' + f);
  console.log('─────────────────────────');

  for (const p of peers) p.close();
  await sleep(200);
  process.exit(fails.length ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(2);
});
