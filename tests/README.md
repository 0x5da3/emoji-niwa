# tests/ — dev-only Playwright regression suite

These tests are **development tooling only**. The shipped product is still a
single zero-dependency `index.html`; nothing here is bundled, imported, or
served. `node_modules/` and reports are git-ignored.

## Setup (first time)

```bash
npm install
npx playwright install --with-deps chromium webkit
```

## Run

```bash
npm test            # all specs, Chromium + Mobile WebKit
npm run test:ui     # interactive UI mode
npm run test:report # open the last HTML report
```

The config starts `python3 -m http.server 8000` automatically (and reuses an
already-running one locally).

## What is covered

Tests drive the real UI and assert against the `emoji-niwa-save` localStorage
JSON, the DOM, and the absence of console/page errors (canvas emoji rendering
is OS-dependent, so it is only smoke-checked, never pixel-compared).

- `smoke` — boots clean, canvas non-blank, 🎲 weather-random default, offline
  safety of the share/room buttons.
- `i18n` — JA⇄EN switch updates & persists, 🎲 survives relabel.
- `store_scatter` — exactly one 3×3 🏪 with a clear footprint; count cap.
- `persistence` — reload restores; 全リセット clears; 新規マップ resizes.
- `modes_undo` — mode buttons toggle the palettes; undo path.
- `share_view` — URL share → viewing mode autosave isolation →
  「これを自分のものに」 / 「自分の箱庭に戻る」 (regressions #26–#28).
- `time` — time presets / pause persist into the save.

## Out of scope

Multiplayer rooms, GitHub OAuth, and the Rust `server/` need a backend and are
**not** exercised here beyond "the UI exists and offline stays unbroken".
Backend integration testing is a separate effort.
