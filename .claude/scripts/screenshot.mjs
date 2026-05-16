// UI 変更の目視確認用スクリーンショット（依存ゼロのまま：Playwright は dev のみ・リポジトリには含めない）
// 使い方: node .claude/scripts/screenshot.mjs [out.png] [url]
//   out.png 省略時 → /tmp/emoji-niwa-<timestamp>.png
//   url     省略時 → リポジトリの index.html を file:// で開く
// 事前準備（コンテナ初回のみ）: npx --yes playwright install chromium
// Playwright/ブラウザ未導入なら導入手順を表示して終了。
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const indexHtml = resolve(here, '../../index.html');
const out = process.argv[2] || `/tmp/emoji-niwa-${Date.now()}.png`;
const url = process.argv[3] || `file://${indexHtml}`;
const vp = process.env.SHOT_VP || '1100,720';
const waitMs = process.env.SHOT_WAIT || '4000';

const r = spawnSync(
  'npx',
  ['--yes', 'playwright', 'screenshot',
    `--viewport-size=${vp}`, `--wait-for-timeout=${waitMs}`, url, out],
  { stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8' }
);

const log = (r.stdout || '') + (r.stderr || '');
if (r.status === 0) {
  console.log(`screenshot: OK → ${out}  (viewport ${vp})`);
  process.exit(0);
}
if (/Executable doesn't exist|playwright install|not found/i.test(log)) {
  console.error('screenshot: Playwright かブラウザが未導入です。');
  console.error('  先に実行: npx --yes playwright install chromium');
} else {
  console.error('screenshot: 失敗\n' + log.trim());
}
process.exit(1);
