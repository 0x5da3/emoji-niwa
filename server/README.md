# emoji-niwa multiplayer server (optional)

🌐 **English** | [日本語](README.ja.md)

Rust + Actix Web backend powering **opt-in** real-time room co-editing for
emoji-niwa. The browser app (`../index.html`) stays a single zero-dependency
file; this service is only contacted when a member issues / someone joins a
room (`#r=<id>`). Offline play and `#w=` snapshot sharing never touch it.

- Members log in with **GitHub OAuth** (state CSRF, confidential client).
- **Only members can issue a room** (`POST /room/new`). Joining `/room/{id}`
  (WebSocket) is open to anyone with the invite link.
- Dumb relay: keeps each room's latest compact world snapshot (the client's
  `encodeWorld` string, opaque here) and fans it out; last-writer-wins.
- In-process state, snapshotted to `DATA_PATH` every 15s and on shutdown,
  restored on boot. Single instance (friend scale); horizontal scaling would
  need a shared store (out of scope).

## Endpoints

| Method | Path             | Auth        | Purpose                                  |
|--------|------------------|-------------|------------------------------------------|
| GET    | `/healthz`       | –           | liveness                                 |
| GET    | `/auth/login`    | –           | redirect to GitHub OAuth                  |
| GET    | `/auth/callback` | –           | OAuth callback → `#auth=<token>` redirect |
| GET    | `/auth/me`       | Bearer      | current member                           |
| POST   | `/auth/logout`   | Bearer      | revoke session                           |
| POST   | `/room/new`      | Bearer      | **member-only** — issue a room id         |
| POST   | `/room/{id}/ttl` | Bearer      | **owner-only** — set empty-room retention (days, 1-30) |
| GET    | `/room/{id}`     | – (WS)      | join/co-edit (must already exist)         |

Wire: C→S `{"t":"snap","d":<encoded>}` `{"t":"hello","name":str}`
`{"t":"chat","text":str}` · S→C `{"t":"snap","d"}`
`{"t":"role","owner":bool,"ttlDays":int}` `{"t":"peers","n":int,"cap":int,"names":[str]}`
`{"t":"chat","name":str,"text":str}`
`{"t":"chatlog","items":[{"name":str,"text":str,"ts":int}]}` `{"t":"full"}`.
Chat is relayed live, and a bounded recent backlog (~100, in-memory) is
replayed to a late joiner on join (`ts` = epoch ms; not persisted across
server restarts).

## Run locally

```bash
cd server
GH_CLIENT_ID=xxx GH_CLIENT_SECRET=yyy \
APP_URL=http://localhost:8000 PUBLIC_BASE=http://localhost:8080 \
DEV=1 DATA_PATH=./data/state.json cargo run
```
Serve the client (`python3 -m http.server 8000` in repo root) and temporarily
set `MP_HTTP`/`MP_WS` in `index.html` to `http://localhost:8080` /
`ws://localhost:8080`.

## Deploy A — GitHub Actions (no local CLI; works from iOS)

`.github/workflows/fly-deploy.yml` deploys `server/` with
`flyctl deploy --remote-only`. One-time setup, then push (or run the
workflow) — all doable from a phone browser.

1. **GitHub OAuth App** — Authorization callback URL =
   `<PUBLIC_BASE>/auth/callback` (e.g. `https://<app>.fly.dev/auth/callback`).
2. **Create the Fly app + volume once** (the workflow only *deploys*; it does
   not create them). Easiest from any shell / Fly dashboard:
   `fly apps create <app>` then
   `fly volumes create niwa_data --size 1 --region nrt`
   (volume name + region must match `fly.toml`).
3. **Fly app secrets** (Fly dashboard → app → Secrets, or `fly secrets set`):
   `GH_CLIENT_ID`, `GH_CLIENT_SECRET`,
   `APP_URL=https://0x5da3.github.io/emoji-niwa`,
   `PUBLIC_BASE=https://<app>.fly.dev`.
4. **GitHub repo secret** (Settings → Secrets and variables → Actions):
   `FLY_API_TOKEN` — an **app-scoped Deploy token** (`~90d` expiry
   recommended; an org token, short-lived, is only needed for the very
   first `fly apps create`).
5. Push a change under `server/**` to `main`, or run the workflow manually
   (Actions tab → *Deploy server to Fly.io* → Run).
6. In `../index.html` set `MP_HTTP='https://<app>.fly.dev'` and
   `MP_WS='wss://<app>.fly.dev'`, commit.

## Deploy B — local CLI (alternative)

```bash
cd server
fly launch --no-deploy        # creates the app from fly.toml/Dockerfile
fly volumes create niwa_data --size 1 --region nrt
fly secrets set GH_CLIENT_ID=... GH_CLIENT_SECRET=... \
  APP_URL=https://0x5da3.github.io/emoji-niwa PUBLIC_BASE=https://<app>.fly.dev
fly deploy
```
Optional: put a custom domain in front via Cloudflare (orange-cloud, WS
enabled) and use it for `PUBLIC_BASE`/`MP_*`/the OAuth callback. `*.fly.dev`
alone (HTTPS already) works without Cloudflare.

## Env

`GH_CLIENT_ID`, `GH_CLIENT_SECRET`, `APP_URL` (full browser app URL **incl.
path**, e.g. `https://0x5da3.github.io/emoji-niwa` — used for the post-login
redirect, and its origin `scheme://host` is derived for CORS/WS matching;
`APP_ORIGIN` is accepted as a deprecated alias), `PUBLIC_BASE` (this server's
external base, must equal the OAuth callback host, for `redirect_uri`),
`BIND_ADDR` (default `0.0.0.0:8080`), `DATA_PATH` (default
`./data/state.json`).

Security knobs:

- `DEV` — set `1`/`true` **only for local dev**. When unset (production),
  only the app origin derived from `APP_URL` is allowed (CORS **and** the
  WebSocket handshake). In dev it additionally allows `http://localhost` /
  `http://127.0.0.1` on any port (exact host match — no loose prefix).
- `MAX_SNAP_BYTES` — max accepted world-snapshot size in bytes
  (default `262144` = 256 KB). Oversize `snap` messages are dropped (not
  stored/relayed); grossly oversize (> 2×) drops the connection. Guards
  memory/bandwidth and broadcast amplification. Raise via env, or move to
  chunked snapshots if a legitimate world ever exceeds it.
- `ALLOWED_LOGINS` — comma-separated GitHub usernames allowed to sign in
  (case-insensitive). Empty/unset = any GitHub account may log in. Enforced
  both at OAuth callback (no session issued) and on every authenticated
  request (existing sessions for non-allowed users stop working). e.g.
  `ALLOWED_LOGINS=0x5da3`.
- `MAX_ROOM_PEERS` — max concurrent participants per room incl. the creator
  (default `8`, min `1`). A join beyond the cap gets a `{"t":"full"}`
  message then the socket is closed (client shows a “room full” toast and
  stays on its own world; no reconnect).
- `ROOM_TTL_DAYS` — default retention for empty rooms in days (default `7`,
  clamped to `1`..`30`). Per-room overridable by the room owner via
  `POST /room/{id}/ttl`; the GC drops a room only after it has been empty
  (no connections) for longer than its retention.
