#!/usr/bin/env node
// Zero-dependency syntax check for the inline JS in index.html.
// Extracts every non-src <script> block and runs `node --check` on it.
// Exit 0 = syntax OK, exit 1 = syntax error (Node diagnostic is printed).

import { readFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const htmlPath = process.argv[2]
  ? resolve(process.argv[2])
  : join(repoRoot, 'index.html');

const html = readFileSync(htmlPath, 'utf8');

// Match <script ...>...</script>, capture the opening-tag attributes and body.
const scriptRe = /<script\b([^>]*)>([\s\S]*?)<\/script>/gi;
const blocks = [];
let m;
while ((m = scriptRe.exec(html)) !== null) {
  const attrs = m[1];
  if (/\bsrc\s*=/.test(attrs)) continue; // external script, nothing to check
  blocks.push(m[2]);
}

if (blocks.length === 0) {
  console.error(`check-syntax: no inline <script> blocks found in ${htmlPath}`);
  process.exit(1);
}

const combined = blocks.join('\n;\n');
const tmpFile = join(
  tmpdir(),
  `emoji-niwa-syntax-${process.pid}-${Date.now()}.js`,
);

let exitCode = 0;
try {
  writeFileSync(tmpFile, combined, 'utf8');
  execFileSync(process.execPath, ['--check', tmpFile], { stdio: 'inherit' });
  console.log(
    `check-syntax: OK — ${blocks.length} inline <script> block(s) in ${htmlPath} parse cleanly.`,
  );
} catch (err) {
  // node --check already printed the diagnostic (with line number) to stderr.
  exitCode = typeof err.status === 'number' ? err.status : 1;
  console.error('check-syntax: FAILED — inline JS has a syntax error (see above).');
} finally {
  try {
    unlinkSync(tmpFile);
  } catch {
    /* temp file may not exist; ignore */
  }
}

process.exit(exitCode);
