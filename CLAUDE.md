# CLAUDE.md — emoji-niwa (絵文字庭)

ブラウザだけで動く箱庭サンドボックスゲーム。Canvas で絵文字を並べて庭をつくる。

## 絶対に守る制約（最重要）

- **単一ファイル**: HTML / CSS / JS はすべて `index.html` に内包する。新しいソースファイルに分割しない。
- **依存ゼロ・ビルドなし**: フレームワーク・npm パッケージ・ビルドツール・CDN 読み込みを一切追加しない。「インストール不要・依存ゼロ・単一 HTML」は README で謳う製品の売り（README.md / README.ja.md L9・L57）。

迷ったら「依存を増やさず素の Web API で実装する」を選ぶ。

**サーバー例外（多人数のみ）**: クライアント（`index.html`）は上記を厳守＝単一ファイル・
依存ゼロのまま。例外として **任意・オプトインの多人数機能**用に Rust/Actix Web の
バックエンドを `server/`（別管理・別デプロイ）に置くことを許可する。`server/` は
Cargo の通常依存可。クライアントは `MP_HTTP`/`MP_WS` 未設定なら多人数 UI を無効化し、
オフライン/`#w=` 共有は非会員でも従来どおり動く（既定はオフライン）。`server/` の
依存は `index.html` には一切持ち込まない。

## アーキテクチャ

- ロジックは `index.html` 内の単一インライン `<script>`（現状およそ 406〜4357 行）。
- 描画: Canvas 2D（アイソメトリック投影）＋自前の Perlin ノイズ地形生成。
- 効果音: Web Audio API でリアルタイム合成（音声ファイルは持たない）。
- 永続化: localStorage。保存キーは `emoji-niwa-save`（`index.html` 4104 行付近）。オートセーブ＋手動セーブ。

## 多言語（日英）規約

- UI 文字列は `data-i18n` / `data-i18n-title` 属性で JA/EN を切り替える（`<html lang="ja">`、選択は localStorage に保存）。
- **文言を追加・変更したら日本語と英語の両方を必ず同期する。** 翻訳対象は表示テキストのみ。

## ドキュメント同期

機能を追加・変更したら `README.md`（英語）と `README.ja.md`（日本語）の両方を更新する。

## ローカル実行 / プレビュー

```bash
python3 -m http.server 8000   # → http://localhost:8000
```

`index.html` を直接ブラウザで開いてもよい。iOS Safari で日本語が文字化けする場合は charset=utf-8 を付けて配信する。

## 編集後の検証

`index.html` を編集したら、まず以下で JS の構文を確認する（依存ゼロ・Web セッションでも常用）:

```bash
node .claude/scripts/check-syntax.mjs
```

挙動の確認はブラウザで目視（配置・天気・時間・セーブ／ロード・日英切替など）。

## テスト（dev 専用・任意）

`tests/` に Playwright のブラウザ回帰テストがある。**これは開発ツールであり出荷物ではない**
— アプリ本体は依存ゼロの単一 `index.html` のまま。`package.json` / `node_modules`
はテスト用かつ git 無視で、`index.html` には一切持ち込まない（上記「絶対に守る制約」を満たす）。

```bash
npm install && npx playwright install --with-deps chromium webkit   # 初回のみ
npm test            # Chromium + Mobile WebKit で全 spec
```

検証はセーブ JSON（`emoji-niwa-save`）・DOM・コンソールエラー無しが主軸（絵文字 canvas は
OS 依存のため厳密比較しない）。多人数 / OAuth / Rust `server/` はバックエンド必須のため
スコープ外（UI 存在＋オフライン非破壊のみ）。詳細は `tests/README.md`。
構文チェック（上記）は依存ゼロの常用クイック確認として引き続き必須。

## コミット規約

既存履歴に倣い、**先頭に内容を表す絵文字を付けた日本語のコミットメッセージ**にする。

例: `🏪 人が入店して人数表示＋ランダム配置で🏪1個保証＋入店音`、`🍽️ 動物・人が近くの食べ物を食べて消費する機能`

## ファイル構成

```
emoji-niwa/
├── index.html            # アプリ本体（HTML + CSS + JS すべて）
├── README.md             # 英語
├── README.ja.md          # 日本語
├── assets/screenshot.jpg # README 用画像
├── server/               # 任意・多人数用 Rust/Actix バックエンド（別管理）
├── tests/                # dev 専用 Playwright（出荷物ではない）
├── playwright.config.ts  # dev 専用
├── package.json          # dev 専用（テストのみ・git 無視の node_modules）
└── .claude/              # Claude Code 用のフック・スクリプト・設定
```

大きなバイナリを増やさない（GitHub Pages 直配信のため軽量に保つ）。`node_modules/` と
Playwright のレポートは `.gitignore` 済み＝配信物には含めない。
