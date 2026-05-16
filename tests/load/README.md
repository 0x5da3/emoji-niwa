# tests/load/ — dev-only WebSocket load harness (multiplayer server)

Stress / soak test for the `server/` relay. **Dev tooling only** — the shipped
product is still the single zero-dependency `index.html`; this never ships and
its `ws` dependency lives in the git-ignored `node_modules`.

The server is a *dumb relay*: a client sends `{t:'snap',d:<opaque string>}`,
the server keeps the room's latest `d` and fans it out to every **other**
connection (`server/src/main.rs:442`). A real browser adds nothing for server
load, so we drive raw `ws` clients instead and make `d` a tiny JSON envelope
(`{rid,sid,seq,ts,pad}`) so receivers can measure fan-out latency and detect
dropped snapshots.

## Run (end-to-end, builds nothing)

```bash
# one-time: build the server
cargo build --release --manifest-path server/Cargo.toml

# seed rooms → start server (DEV=1, no OAuth) → load → teardown
npm run load                                  # default 1×8 (8-player health)
ROOMS=50 PEERS=8 DURATION_MS=15000 npm run load   # ~400 conns, medium
ROOMS=200 PEERS=8 SNAP_MS=120 npm run load        # push toward the limit
```

`run.mjs` seeds `server/data/state.json` (git-ignored) so clients can join
without GitHub OAuth, starts the release binary with `DEV=1`, waits for
`/healthz`, runs the harness with `SERVER_PID` set (for `/proc` CPU/RSS
sampling), then `SIGTERM`s the server.

Standalone (server already up + rooms seeded): `node tests/load/ws-load.mjs`.

### Knobs (env)

`ROOMS` `PEERS` `ROOM_PREFIX` `SNAP_MS` (send interval; app uses ~320ms)
`SNAP_BYTES` (envelope size, ~4 KB default) `DURATION_MS` `RAMP_MS`
`STABILIZE_MS` `DRAIN_MS` `WS_BASE` `ORIGIN` `PORT`.

## What it asserts (true N-player play) & measures

Health (non-zero exit on FAIL): every peer connects; every peer in a room saw
`peers == PEERS`; exactly one `role:owner` per room; every peer received snaps
from all `PEERS-1` others. Metrics: send/recv rate, fan-out latency
p50/p90/p99/max, dropped-snap count (sequence gaps = broadcast `Lagged`),
server CPU% and peak RSS.

## Baseline results (this environment, 1 vCPU class)

| scenario            | conns | sent/s | recv/s | p99    | drops | server CPU | RSS    | health |
|---------------------|-------|--------|--------|--------|-------|------------|--------|--------|
| 8-player health 1×8 | 8     | 24     | 168    | 44 ms  | 0     | 0.8 %      | 7.5 MB | PASS   |
| medium 50×8         | 400   | 1227   | 8587   | 48 ms  | 0     | 18.5 %     | 35 MB  | PASS   |

## Server-specific bottlenecks to watch when scaling further

- **Broadcast cap 64** (`BCAST_CAP`, `main.rs:36`): a receiver >64 messages
  behind gets `Lagged` and silently misses snapshots (last-write-wins, so
  functionally tolerable but a clear overload signal). Shows up as
  "dropped snaps".
- **O(P²) fan-out**: each receiver task re-serializes its own JSON per
  snapshot (`main.rs:466`). CPU is the first thing to climb with peers/room
  and snap rate — the main scaling limit.
- **15 s persistence tick** (`main.rs:565`): `save_state` walks every room,
  locks each snap mutex and writes the file synchronously → periodic p99
  spike under many/large rooms.
- **`ulimit -n`** bounds client-side concurrency (4096 here). Raise it and
  ephemeral-port range before attempting limit-exploration runs.

Out of scope: GitHub OAuth flow and disk-restore correctness (the harness
deliberately bypasses auth by pre-seeding rooms).
