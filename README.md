# 🌿 emoji-niwa (絵文字庭)

🌐 **English** | [日本語](README.ja.md)

<p align="center"><img src="assets/screenshot.jpg" alt="emoji-niwa — a night garden with aurora, moon and a meteor" width="420"></p>

Place emoji to build your own little garden — a sandbox that runs entirely in the browser.

Generate terrain, arrange emoji, change the weather and time of day, and watch fireworks or auroras at night. No install, zero dependencies, all in a single HTML file.

**🎮 Demo:** https://0x5da3.github.io/emoji-niwa/

## ✨ Features

- **2D / 3D view** — Switch between a top-down flat view and an isometric 3D view with height
- **Place emoji** — 10 categories (nature, animals, buildings, sky, food, vehicles, people, seasonal events, …). Animals wander the garden, sleep at night (💤), and nocturnal creatures (🦉🦊🦝🦔🐍🐈‍⬛) roam after dark
- **Terrain paint** — Grass / sand / water / snow / rock plus custom colors; raise and lower tile height
- **Procedural terrain** — 10 presets (grassland / island / mountains / desert / snowfield / archipelago / forest / volcano / atoll / valley) via Perlin noise, solid fill, and random scatter
- **Time & sky** — 24-hour time slider; sky, sun, moon and stars change with day/night (time can be paused)
- **Weather** — Clear / rain / snow / thunderstorm / hail / sandstorm / cherry blossoms / autumn leaves, plus a random-rotation mode
- **Effects** — Night: fireworks & aurora / Day: bubbles & morning mist. Tap the moon for a meteor shower; tap the sun while raining for a rainbow
- **Sound** — 9 cute placement sounds synthesized with the Web Audio API, plus fireworks/bubble sounds (mutable)
- **Language switch** — Toggle Japanese / English from the settings menu (your choice is saved)
- **Saving** — Autosave to localStorage (configurable interval) plus manual save; your garden survives reloads
- **Share via URL** — the “🔗” button in the left button column encodes your world into a URL (no server). Opening one starts a read-but-editable visiting mode that never touches the viewer's own garden or autosave (with “Back to my garden” / “Make this mine”)
- **Multiplayer (optional, shared play)** — “👥” issues an invite URL for real-time co-editing with friends. **Issuing requires a GitHub-login member**; anyone with the invite URL can join. While in a room your offline garden / autosave is never touched. Needs the optional backend (Rust/Actix, see `server/`). If unconfigured, offline play and `🔗` sharing work as before
- **Helpers** — Zoom, minimap, fullscreen, undo, new map (5×5–50×50)

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
