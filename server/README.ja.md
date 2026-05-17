# emoji-niwa 多人数サーバー（任意）

🌐 [English](README.md) | **日本語**

emoji-niwa の**オプトイン**リアルタイム共同編集（URLルーム）を担う Rust + Actix
Web バックエンド。ブラウザアプリ（`../index.html`）は単一・依存ゼロのまま。会員が
ルームを発行／誰かが参加（`#r=<id>`）したときだけ本サービスに接続する。オフライン
プレイと `#w=` スナップショット共有は本サービスに一切触れない。

- 会員は **GitHub OAuth**（state CSRF・confidential クライアント）でログイン。
- **ルーム発行は会員のみ**（`POST /room/new`）。参加 `/room/{id}`（WebSocket）は
  招待リンクを持っていれば誰でも可。
- ダム中継：ルーム毎に最新のコンパクトなワールドスナップショット（クライアントの
  `encodeWorld` 文字列。サーバーからは不透明）を保持し全員へ再配信。last-writer-wins。
- 状態はインプロセス保持。15 秒毎＋終了時に `DATA_PATH` へスナップショット保存し、
  起動時に復元。単一インスタンス（友達規模）。水平スケールには共有ストアが必要
  （本スコープ外）。

## エンドポイント

| Method | Path             | 認証        | 用途                                       |
|--------|------------------|-------------|--------------------------------------------|
| GET    | `/healthz`       | –           | 死活確認                                   |
| GET    | `/auth/login`    | –           | GitHub OAuth へリダイレクト                |
| GET    | `/auth/callback` | –           | OAuth コールバック → `#auth=<token>` 返却  |
| GET    | `/auth/me`       | Bearer      | 現在の会員情報                             |
| POST   | `/auth/logout`   | Bearer      | セッション失効                             |
| POST   | `/room/new`      | Bearer      | **会員限定** — ルーム id 発行              |
| GET    | `/room/{id}`     | –（WS）     | 参加／共同編集（既存ルームのみ）           |

ワイヤ: C→S `{"t":"snap","d":<encoded>}` · S→C `{"t":"snap","d"}`
`{"t":"role","owner":bool}` `{"t":"peers","n":int}`。

## ローカル実行

```bash
cd server
GH_CLIENT_ID=xxx GH_CLIENT_SECRET=yyy \
APP_ORIGIN=http://localhost:8000 PUBLIC_BASE=http://localhost:8080 \
DATA_PATH=./data/state.json cargo run
```
クライアントを配信（リポジトリ直下で `python3 -m http.server 8000`）し、`index.html`
の `MP_HTTP`/`MP_WS` を一時的に `http://localhost:8080` / `ws://localhost:8080` に設定。

## デプロイ（Fly.io ＋ 前段 Cloudflare）

1. **GitHub OAuth App** を作成。Authorization callback URL =
   `https://<api-domain>/auth/callback`。
2. `fly launch`（`Dockerfile`/`fly.toml` を使用）。ボリューム作成：
   `fly volumes create niwa_data --size 1 --region nrt`。
3. シークレット設定：
   ```bash
   fly secrets set GH_CLIENT_ID=... GH_CLIENT_SECRET=... \
     APP_ORIGIN=https://0x5da3.github.io/emoji-niwa \
     PUBLIC_BASE=https://<api-domain>
   ```
4. `fly deploy`。
5. Cloudflare で `<api-domain>` を Fly アプリにプロキシ（orange-cloud、
   WebSocket 有効）。
6. `../index.html` の `MP_HTTP='https://<api-domain>'` と
   `MP_WS='wss://<api-domain>'` を設定してコミット。

## 環境変数

`GH_CLIENT_ID`、`GH_CLIENT_SECRET`、`APP_ORIGIN`（ブラウザアプリのオリジン。
ログイン後リダイレクト＋CORS 用）、`PUBLIC_BASE`（本サーバーの外部ベース URL。
OAuth `redirect_uri` 用）、`BIND_ADDR`（既定 `0.0.0.0:8080`）、`DATA_PATH`
（既定 `./data/state.json`）。

セキュリティ設定:

- `DEV` — **ローカル開発時のみ** `1`/`true` を設定。未設定（本番）では
  許可オリジンは `APP_ORIGIN` のみ（CORS **および** WebSocket ハンドシェイク）。
  dev では加えて `http://localhost` / `http://127.0.0.1`（任意ポート）を許可
  （ホスト完全一致。緩い前方一致はしない）。
- `MAX_SNAP_BYTES` — 受理するワールドスナップショットの最大バイト数
  （既定 `262144` = 256 KB）。超過した `snap` は破棄（保存／中継しない）、
  著しい超過（2 倍超）は接続を切断。メモリ／帯域と再配信増幅を防ぐ。env で
  引き上げ可。正当なワールドが恒常的に超えるならチャンク分割へ移行。
- `MAX_ROOM_PEERS` — 1 ルームの同時接続上限（既定 `8`）。上限超の参加は
  `409` で拒否（ルーム自体は継続）。
- `MAX_ROOMS_PER_MEMBER` — 会員あたりの保持ルーム数上限（既定 `3`。`0` で
  無制限）。上限超の `POST /room/new` は `429`。空ルームは 6 時間アイドルで
  GC され、枠は再び空く。
- `SNAP_RATE` / `SNAP_BURST` — 接続ごとの `snap` レート制限。`SNAP_RATE`/秒
  （既定 `6.0`）で補充し最大 `SNAP_BURST` トークン（既定 `12.0`）のトークン
  バケツ。アプリは約 3〜4/秒＋バーストなので十分余裕。超過 snap は黙って破棄
  （保存・中継せず、接続も切らない）。

永続化は dirty フラグでゲートし、ブロッキングプールスレッドで実行する。
アイドル時は変更があるまでディスク I/O **ゼロ**、15 秒フラッシュも非同期
ランタイムを止めない。
