// Dev-only: seed server/data/state.json with N relay rooms so the WS load
// harness can join without going through GitHub OAuth. The server's
// load_state() restores these on boot (server/src/main.rs:200).
//
//   node tests/load/seed-rooms.mjs <count> [outPath] [roomPrefix]
//
// PersistRoom shape: { id, snap: null, creator_uid }  (server/src/main.rs:153)

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

const count = parseInt(process.argv[2] || '1', 10);
const out = process.argv[3] || 'server/data/state.json';
const prefix = process.argv[4] || 'load-';

if (!Number.isInteger(count) || count < 1) {
  console.error('usage: node tests/load/seed-rooms.mjs <count> [outPath] [roomPrefix]');
  process.exit(2);
}

const rooms = Array.from({ length: count }, (_, i) => ({
  id: `${prefix}${i}`,
  snap: null,
  creator_uid: 'load',
}));

mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, JSON.stringify({ rooms, sessions: [], members: [] }));
console.log(`seeded ${count} room(s) "${prefix}0..${count - 1}" -> ${out}`);
