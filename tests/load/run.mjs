// Dev-only end-to-end orchestrator: seed rooms → start the release server
// (DEV=1, no OAuth needed since rooms are pre-seeded) → wait for /healthz →
// run the WS load harness with SERVER_PID set → SIGTERM the server.
//
//   ROOMS=50 PEERS=8 DURATION_MS=20000 node tests/load/run.mjs
//
// All ws-load.mjs knobs pass straight through via env.

import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';

const env = (k, d) => process.env[k] ?? d;
const ROOMS = parseInt(env('ROOMS', '1'), 10);
const ROOM_PREFIX = env('ROOM_PREFIX', 'load-');
const PORT = parseInt(env('PORT', '8080'), 10);
const DATA_PATH = env('DATA_PATH', 'server/data/state.json');
const APP_ORIGIN = env('ORIGIN', 'http://localhost:8000');
const BIN = 'server/target/release/emoji-niwa-server';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

if (!existsSync(BIN)) {
  console.error(`missing ${BIN} — build first: cargo build --release --manifest-path server/Cargo.toml`);
  process.exit(2);
}

// 1. Seed rooms.
const seed = spawnSync(
  process.execPath,
  ['tests/load/seed-rooms.mjs', String(ROOMS), DATA_PATH, ROOM_PREFIX],
  { stdio: 'inherit' },
);
if (seed.status !== 0) process.exit(2);

// 2. Start the server.
const server = spawn(BIN, [], {
  stdio: ['ignore', 'inherit', 'inherit'],
  env: {
    ...process.env,
    DEV: '1',
    BIND_ADDR: `127.0.0.1:${PORT}`,
    DATA_PATH,
    APP_ORIGIN,
    PUBLIC_BASE: `http://127.0.0.1:${PORT}`,
    GH_CLIENT_ID: '',
    GH_CLIENT_SECRET: '',
  },
});
let serverExited = false;
server.on('exit', () => (serverExited = true));

const stopServer = () => {
  if (!serverExited) try { server.kill('SIGTERM'); } catch {}
};
process.on('SIGINT', () => { stopServer(); process.exit(130); });

// 3. Wait for /healthz.
const healthDeadline = Date.now() + 15000;
let healthy = false;
while (Date.now() < healthDeadline && !serverExited) {
  try {
    const res = await fetch(`http://127.0.0.1:${PORT}/healthz`);
    if (res.ok) { healthy = true; break; }
  } catch {}
  await sleep(250);
}
if (!healthy) {
  console.error('server did not become healthy');
  stopServer();
  process.exit(2);
}

// 4. Run the load harness.
const load = spawn(
  process.execPath,
  ['tests/load/ws-load.mjs'],
  {
    stdio: 'inherit',
    env: { ...process.env, SERVER_PID: String(server.pid), PORT: String(PORT) },
  },
);
const code = await new Promise((r) => load.on('exit', r));

// 5. Teardown (SIGTERM → server save_state + exit, server/src/main.rs:580).
stopServer();
await sleep(400);
process.exit(code ?? 0);
