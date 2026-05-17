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
| POST   | `/room/{id}/ttl` | Bearer      | **オーナー限定** — 空き部屋の保持日数を設定（1〜30） |
| GET    | `/room/{id}`     | –（WS）     | 参加／共同編集（既存ルームのみ）           |

ワイヤ: C→S `{"t":"snap","d":<encoded>}` `{"t":"hello","name":str}`
`{"t":"chat","text":str}` · S→C `{"t":"snap","d"}`
`{"t":"role","owner":bool,"ttlDays":int}` `{"t":"peers","n":int,"cap":int,"names":[str]}`
`{"t":"chat","name":str,"text":str}`
`{"t":"chatlog","items":[{"name":str,"text":str,"ts":int}]}` `{"t":"full"}`。
チャットはライブ中継に加え、直近のバックログ（約100件・メモリ内）を後入り
参加者へ参加時に再生（`ts`=エポックms。サーバ再起動は跨いで保持しない）。

## ローカル実行

```bash
cd server
GH_CLIENT_ID=xxx GH_CLIENT_SECRET=yyy \
APP_URL=http://localhost:8000 PUBLIC_BASE=http://localhost:8080 \
DEV=1 DATA_PATH=./data/state.json cargo run
```
クライアントを配信（リポジトリ直下で `python3 -m http.server 8000`）し、`index.html`
の `MP_HTTP`/`MP_WS` を一時的に `http://localhost:8080` / `ws://localhost:8080` に設定。

## デプロイ A — GitHub Actions（ローカル CLI 不要・iOS だけで可）

`.github/workflows/fly-deploy.yml` が `server/` を `flyctl deploy --remote-only`
でデプロイ。一度きりの準備のあとは push か手動実行だけ（スマホのブラウザで完結）。

1. **GitHub OAuth App** — Authorization callback URL =
   `<PUBLIC_BASE>/auth/callback`（例 `https://<app>.fly.dev/auth/callback`）。
2. **Fly アプリ＋ボリュームを一度だけ作成**（ワークフローは*デプロイのみ*で
   作成はしない）。任意のシェル／Fly ダッシュボードで：
   `fly apps create <app>` → `fly volumes create niwa_data --size 1 --region nrt`
   （ボリューム名・リージョンは `fly.toml` と一致）。
3. **Fly app secrets**（Fly ダッシュボード → アプリ → Secrets、または
   `fly secrets set`）：`GH_CLIENT_ID`、`GH_CLIENT_SECRET`、
   `APP_URL=https://0x5da3.github.io/emoji-niwa`、
   `PUBLIC_BASE=https://<app>.fly.dev`。
4. **GitHub repo secret**（Settings → Secrets and variables → Actions）：
   `FLY_API_TOKEN` — **App スコープ Deploy トークン**（有効期限は `90日` 推奨。
   Org トークンは初回 `fly apps create` のときだけ短期で使い捨て）。
5. `server/**` の変更を `main` に push、または Actions タブの
   *Deploy server to Fly.io* を手動 Run。
6. `../index.html` の `MP_HTTP='https://<app>.fly.dev'` と
   `MP_WS='wss://<app>.fly.dev'` を設定してコミット。

## デプロイ B — ローカル CLI（代替）

```bash
cd server
fly launch --no-deploy        # fly.toml/Dockerfile からアプリ作成
fly volumes create niwa_data --size 1 --region nrt
fly secrets set GH_CLIENT_ID=... GH_CLIENT_SECRET=... \
  APP_URL=https://0x5da3.github.io/emoji-niwa PUBLIC_BASE=https://<app>.fly.dev
fly deploy
```
任意：独自ドメインを Cloudflare 前段に置き（orange-cloud・WS 有効）、それを
`PUBLIC_BASE`/`MP_*`/OAuth callback に使用。`*.fly.dev` 単体（既に HTTPS）でも
Cloudflare 無しで動く。

## 環境変数

`GH_CLIENT_ID`、`GH_CLIENT_SECRET`、`APP_URL`（ブラウザアプリの**パス込み**
フル URL。例 `https://0x5da3.github.io/emoji-niwa`。ログイン後リダイレクトに使用し、
その origin `scheme://host` を CORS/WS 照合用に導出。`APP_ORIGIN` は非推奨の
別名として受理）、`PUBLIC_BASE`（本サーバーの外部ベース URL。OAuth callback の
ホストと一致必須。`redirect_uri` 用）、`BIND_ADDR`（既定 `0.0.0.0:8080`）、
`DATA_PATH`（既定 `./data/state.json`）。

セキュリティ設定:

- `DEV` — **ローカル開発時のみ** `1`/`true` を設定。未設定（本番）では
  許可オリジンは `APP_URL` から導出した origin のみ（CORS **および** WebSocket
  ハンドシェイク）。dev では加えて `http://localhost` / `http://127.0.0.1`
  （任意ポート）を許可（ホスト完全一致。緩い前方一致はしない）。
- `MAX_SNAP_BYTES` — 受理するワールドスナップショットの最大バイト数
  （既定 `262144` = 256 KB）。超過した `snap` は破棄（保存／中継しない）、
  著しい超過（2 倍超）は接続を切断。メモリ／帯域と再配信増幅を防ぐ。env で
  引き上げ可。正当なワールドが恒常的に超えるならチャンク分割へ移行。
- `ALLOWED_LOGINS` — ログインを許可する GitHub ユーザー名のカンマ区切り
  （大文字小文字無視）。空／未設定なら任意の GitHub アカウントがログイン可。
  OAuth コールバック（セッション未発行）と認証付き全リクエスト（非許可
  ユーザーの既存セッションも無効化）の両方で強制。例 `ALLOWED_LOGINS=0x5da3`。
- `MAX_ROOM_PEERS` — 1ルームの最大同時参加人数（作成者含む。既定 `8`、
  最小 `1`）。上限超の参加は `{"t":"full"}` を受け取りソケットが閉じる
  （クライアントは満員トースト表示・自分の世界に留まる・再接続しない）。
- `ROOM_TTL_DAYS` — 空き部屋のデフォルト保持日数（既定 `7`、`1`〜`30` に
  クランプ）。部屋ごとにオーナーが `POST /room/{id}/ttl` で上書き可。GC は
  接続0の状態が保持日数を超えた部屋だけを破棄する。
