# emoji-niwa multiplayer server (optional)

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
| GET    | `/room/{id}`     | – (WS)      | join/co-edit (must already exist)         |

Wire: C→S `{"t":"snap","d":<encoded>}` · S→C `{"t":"snap","d"}`
`{"t":"role","owner":bool}` `{"t":"peers","n":int}`.

## Run locally

```bash
cd server
GH_CLIENT_ID=xxx GH_CLIENT_SECRET=yyy \
APP_ORIGIN=http://localhost:8000 PUBLIC_BASE=http://localhost:8080 \
DATA_PATH=./data/state.json cargo run
```
Serve the client (`python3 -m http.server 8000` in repo root) and temporarily
set `MP_HTTP`/`MP_WS` in `index.html` to `http://localhost:8080` /
`ws://localhost:8080`.

## Deploy (Fly.io + Cloudflare front)

1. Create a **GitHub OAuth App**; Authorization callback URL =
   `https://<api-domain>/auth/callback`.
2. `fly launch` (uses `Dockerfile`/`fly.toml`); create the volume:
   `fly volumes create niwa_data --size 1 --region nrt`.
3. Set secrets:
   ```bash
   fly secrets set GH_CLIENT_ID=... GH_CLIENT_SECRET=... \
     APP_ORIGIN=https://0x5da3.github.io/emoji-niwa \
     PUBLIC_BASE=https://<api-domain>
   ```
4. `fly deploy`.
5. In Cloudflare, point/proxy `<api-domain>` at the Fly app (orange-cloud;
   WebSockets enabled).
6. In `../index.html` set `MP_HTTP='https://<api-domain>'` and
   `MP_WS='wss://<api-domain>'`, commit.

## Env

`GH_CLIENT_ID`, `GH_CLIENT_SECRET`, `APP_ORIGIN` (browser app origin, for the
post-login redirect + CORS), `PUBLIC_BASE` (this server's external base, for
the OAuth `redirect_uri`), `BIND_ADDR` (default `0.0.0.0:8080`), `DATA_PATH`
(default `./data/state.json`).
