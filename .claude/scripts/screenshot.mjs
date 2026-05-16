// UI 変更の目視確認用スクリーンショット（依存ゼロのまま：Playwright は dev のみ・リポジトリには含めない）
//
// 既定で 3 環境を撮影する:
//   iphone-air : 420x912   (iPhone Air 相当のポートレート)
//   ipad       : 820x1180  (iPad 相当のポートレート)
//   fullhd     : 1920x1080 (フル HD の Web ブラウザ)
//
// 使い方:
//   node .claude/scripts/screenshot.mjs                 → 3環境を /tmp に出力
//   node .claude/scripts/screenshot.mjs base.png        → base-<env>.png に出力
//   node .claude/scripts/screenshot.mjs base.png <url>  → 撮影対象URLを指定
//   SHOT_VP=390,844 node .claude/scripts/screenshot.mjs → 単一カスタム viewport のみ
//
// Chromium 未導入なら自動で `npx playwright install chromium` してから撮影する
// （新しいコンテナでも手動操作不要・撮影が必要なときだけ初回数分かかる）。
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const indexHtml = resolve(here, '../../index.html');

const baseArg = process.argv[2] || `/tmp/emoji-niwa-${Date.now()}.png`;
const url = process.argv[3] || `file://${indexHtml}`;
const waitMs = process.env.SHOT_WAIT || '4000';

const PRESETS = process.env.SHOT_VP
  ? [{ name: 'custom', vp: process.env.SHOT_VP }]
  : [
      { name: 'iphone-air', vp: '420,912' },
      { name: 'ipad',       vp: '820,1180' },
      { name: 'fullhd',     vp: '1920,1080' },
    ];

const outFor = (name) =>
  PRESETS.length === 1 ? baseArg : baseArg.replace(/\.png$/i, '') + `-${name}.png`;

function shoot(vp, out) {
  return spawnSync(
    'npx',
    ['--yes', 'playwright', 'screenshot',
      `--viewport-size=${vp}`, `--wait-for-timeout=${waitMs}`, url, out],
    { stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8' }
  );
}
const needsBrowser = (log) =>
  /Executable doesn't exist|playwright install|Please run the following/i.test(log);

let browserReady = false;
const done = [];
for (const p of PRESETS) {
  const out = outFor(p.name);
  let r = shoot(p.vp, out);
  let log = (r.stdout || '') + (r.stderr || '');

  if (r.status !== 0 && needsBrowser(log) && !browserReady) {
    console.error('screenshot: Chromium 未導入 → 自動導入します（初回のみ数分）…');
    const ins = spawnSync('npx', ['--yes', 'playwright', 'install', 'chromium'],
      { stdio: 'inherit' });
    if (ins.status !== 0) {
      console.error('screenshot: Chromium の自動導入に失敗しました。');
      console.error('  手動で再試行: npx --yes playwright install chromium');
      process.exit(1);
    }
    browserReady = true;
    r = shoot(p.vp, out);
    log = (r.stdout || '') + (r.stderr || '');
  }

  if (r.status !== 0) {
    console.error(`screenshot: 失敗 (${p.name} ${p.vp})\n` + log.trim());
    process.exit(1);
  }
  browserReady = true;
  done.push(`${p.name.padEnd(10)} ${p.vp.padEnd(10)} → ${out}`);
}

console.log('screenshot: OK');
for (const d of done) console.log('  ' + d);
