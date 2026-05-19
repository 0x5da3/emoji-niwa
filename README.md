# 🌿 emoji-niwa (絵文字庭)

🌐 **English** | [日本語](README.ja.md)

<p align="center"><img src="assets/screenshot.jpg" alt="emoji-niwa — a night garden with aurora, moon and a meteor" width="420"></p>

Place emoji to build your own little garden — a sandbox that runs entirely in the browser.

Generate terrain, arrange emoji, change the weather and time of day, and watch fireworks or auroras at night. No install, zero dependencies, all in a single HTML file.

**🎮 Demo:** https://0x5da3.github.io/emoji-niwa/

## ✨ Features

- **2D / 3D view** — Switch between a top-down flat view and an isometric 3D view with height
- **Place emoji** — 10 categories (nature, animals, buildings, sky, food, vehicles, people, seasonal events, …). Animals wander the garden, sleep at night (💤), and nocturnal creatures (🦉🦊🦝🦔🐍🐈‍⬛) roam after dark. Aquatic animals (🐟🐠🐡🦞🦀🐢) stay in water — if placed on land they swim toward the nearest water cell and vanish after ~30s if they can't reach any
- **Terrain paint** — Grass / sand / water / snow / rock plus custom colors; raise and lower tile height
- **Procedural terrain** — 22 presets in 5 categories (Green / Mountain & Snow / Wilds / Water / Built — e.g. grassland, forest, Amazon, snowy peaks, canyon, mesa, savanna, seaside, atoll, castle) via Perlin noise, plus solid fill and random scatter
- **Auto-place emoji on generated terrain** — generated terrain is automatically dressed with type-matching emoji (grass → trees & flowers, water → fish & ducks, sand → cactus, snow → snowman & deer, rock → rocks & conifers). On by default, with an opt-out toggle and a density slider in the 🎲 Generate panel. Wandering animals start from the spawn cell on the receiver side (positions are not drifted across the share link). Note: the rock terrain pool is intentionally generic since rock and city roads share the same type. Auto-placed emoji are **never enumerated in the share link** — the receiver regenerates them from the same seed, so URLs stay ~150 chars even at high density
- **Time & sky** — 24-hour time slider; sky, sun, moon and stars change with day/night (time can be paused)
- **Weather** — Clear / rain / snow / thunderstorm / hail / sandstorm / cherry blossoms / autumn leaves, plus a random-rotation mode
- **Effects** — Night: fireworks & aurora / Day: bubbles & morning mist. Tap the moon for a meteor shower; tap the sun while raining for a rainbow
- **Sound** — 9 cute placement sounds synthesized with the Web Audio API, plus fireworks/bubble sounds and feedback cues for terrain painting/erasing/height, map generation, solid fill, reset-all, and meteor shower / rainbow / thunder. Also a procedural ambient **BGM that adapts to weather and time of day** (generative, no audio files; off by default, independent of the SFX mute). Mute, an SFX volume slider, and a separate BGM on/off + volume live in the 🔈 panel
- **Language switch** — Toggle Japanese / English from the settings menu (your choice is saved)
- **Saving** — Autosave to localStorage (configurable interval) plus manual save; your garden survives reloads
- **GIF export** — the “📸” button (below 🔈) first shows the **exact square area that will be captured** (anchored to the top of the screen, everything outside dimmed); while it's shown you can pan / zoom the garden to frame the shot, then press OK to record a ~3-second clip of that region **as it looks on screen** (no UI in the frame), exported as a self-encoded GIF via the share sheet or a download (resolution selectable right on the capture frame — Standard 320 / High 720 / Max 1080, default High 720 — for sharper Live Photos). The post-save hint is per-device: on iOS, convert it to a Live Photo with an Apple Shortcut / compatible app (done on-device — browsers can't create Live Photos directly); on Android, use it in a live-wallpaper / motion-photo app. If you cancel the share sheet, nothing is saved and no hint is shown
- **Share link** — the “🔗” button in the left button column encodes your world into a share link (a static snapshot, no server) — choose the native share sheet or just copy the link. Opening one starts a read-but-editable visiting mode that never touches the viewer's own garden or autosave (with “Back to my garden” / “Make this mine”). Distinct from Multiplayer below: this is a one-way snapshot, not live co-editing. When the world is a freshly-generated map with no manual edits, only the recipe (seed + preset + auto-emoji density) is encoded — auto-placed emoji are regenerated deterministically on open, keeping URLs ~150 chars even at high density. Manual edits switch back to a full snapshot for exact round-trip
- **Multiplayer (optional)** — “👥” issues an invite link for real-time co-editing with friends. **Issuing requires a GitHub-login member**; anyone with the invite link can join. While in a room the “👥” button shows live occupancy (e.g. 2/8), a chat box (3 lines above the palette, collapsible/expandable) lets you talk, joins are announced in chat, new joiners see the recent chat history (last ~100 messages, replayed on join), and your offline garden / autosave is never touched. The room owner can set how long an empty room is kept (1-30 days, default 7). Needs the optional backend (Rust/Actix, see `server/`). If unconfigured, offline play and `🔗` share links work as before
- **Helpers** — Zoom, minimap, fullscreen, undo, new map (5×5–50×50), a one-tap collapsible left button column, and an app version + plain-language “what’s new” list in the ⚙️ settings menu
- **In-app help** — a "📖" button opens a How-to-Play panel: the new-player flow plus a quick reference for every control and button (JA/EN, follows the language toggle)
- **Diagonal pad cursor** — a 4-direction diagonal pad (↖↗↙↘) moves an active-cell cursor along the isometric grid axes; the center ⚪ places the selected emoji there (also tap-a-cell, keyboard q/e/z/c or numpad 7/9/1/3, press-and-hold; toggle in ⚙️ settings)

## 🕹 Controls

| Action | Effect |
|---|---|
| Tap / click | Place the selected emoji (or paint/erase terrain) |
| Drag / two fingers | Pan the view |
| ＋ / − buttons | Zoom in / out |
| 🗺 button | Toggle minimap |
| 2D / 3D toggle | Switch view mode |
| ↩ Undo | Undo the last action |
| Tap 🌙 | Meteor shower |
| Tap ☀️ (while raining) | Rain stops and a rainbow appears |

⚙️ Settings, 💾 Save and 🔈 Sound are at the top-left; time, weather, generate and effects menus are in the top bar.

## 🚀 Run locally

No build step. Clone the repo and open `index.html` in a browser.

```bash
git clone https://github.com/0x5da3/emoji-niwa.git
cd emoji-niwa
# open index.html directly, or serve it
python3 -m http.server 8000   # → http://localhost:8000
```

> If Japanese text looks garbled in iOS Safari, serve it with `charset=utf-8` or open the GitHub Pages URL (which sends the charset).

## 🛠 Tech

- **No framework / library / build** — a single self-contained `index.html`
- Rendering with **Canvas 2D** (isometric projection, hand-rolled Perlin-noise terrain)
- Sounds synthesized in real time with the **Web Audio API** (no audio files)
- State persistence via **localStorage**; UI is bilingual (display text only)
- Touch, pinch and two-finger pan supported on mobile

## 📁 Structure

```
emoji-niwa/
├── index.html      # The whole app (HTML + CSS + JS)
├── server/         # optional multiplayer backend (Rust/Actix, deployed separately — see server/README.md)
├── assets/
│   └── screenshot.jpg  # image used in the README
├── README.md       # English (this file)
└── README.ja.md    # Japanese
```

## 🌐 Deploy (GitHub Pages)

Only a static `index.html` at the repo root, so no build is needed.

1. Repository **Settings → Pages**
2. **Source**: `Deploy from a branch`
3. **Branch**: `main` / `/ (root)` → **Save**

It goes live at `https://0x5da3.github.io/emoji-niwa/` within a few minutes, and auto-updates on every push to `main`.
