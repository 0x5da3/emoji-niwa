// UI 変更の目視確認用スクリーンショット（依存ゼロのまま：Playwright は dev のみ・リポジトリには含めない）
// 使い方: node .claude/scripts/screenshot.mjs [out.png] [url]
//   out.png 省略時 → /tmp/emoji-niwa-<timestamp>.png
//   url     省略時 → リポジトリの index.html を file:// で開く
// Chromium 未導入なら自動で `npx playwright install chromium` してから撮影する
// （新しいコンテナでも手動操作不要・撮影が必要なときだけ初回数分かかる）。
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const indexHtml = resolve(here, '../../index.html');
const out = process.argv[2] || `/tmp/emoji-niwa-${Date.now()}.png`;
const url = process.argv[3] || `file://${indexHtml}`;
const vp = process.env.SHOT_VP || '1100,720';
const waitMs = process.env.SHOT_WAIT || '4000';

function shoot() {
  return spawnSync(
    'npx',
    ['--yes', 'playwright', 'screenshot',
      `--viewport-size=${vp}`, `--wait-for-timeout=${waitMs}`, url, out],
    { stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8' }
  );
}
const needsBrowser = (log) =>
  /Executable doesn't exist|playwright install|Please run the following/i.test(log);

let r = shoot();
let log = (r.stdout || '') + (r.stderr || '');

if (r.status !== 0 && needsBrowser(log)) {
  console.error('screenshot: Chromium 未導入 → 自動導入します（初回のみ数分）…');
  const ins = spawnSync('npx', ['--yes', 'playwright', 'install', 'chromium'],
    { stdio: 'inherit' });
  if (ins.status !== 0) {
    console.error('screenshot: Chromium の自動導入に失敗しました。');
    console.error('  手動で再試行: npx --yes playwright install chromium');
    process.exit(1);
  }
  r = shoot();
  log = (r.stdout || '') + (r.stderr || '');
}

if (r.status === 0) {
  console.log(`screenshot: OK → ${out}  (viewport ${vp})`);
  process.exit(0);
}
console.error('screenshot: 失敗\n' + log.trim());
process.exit(1);
