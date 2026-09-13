# HifzSDK

A Rust SDK that wires the **Sabqi → Manzil** daily memorization routine into any
Swift Quran app — without ever embedding the mushaf in a `.swift` file.

## What it does

- Loads the whole mushaf from `data/quran.base.json` (~1.7 MB, 114 surahs,
  6,236 verses).
- Chunks it into **daily Sabqi lessons** (10 verses/day, never breaking a surah
  across a daily lesson) → **624 days**.
- Builds the **Sabqi → Manzil** plan:
  - `sabqi_new`      — today's new lesson
  - `sabqi_review`   — the previous 5 lessons
  - `manzil_review`  — the rolling ~2-week window before the sabqi window
- Exports `data/quran.json` = whole mushaf + `meta` + `hifz_plan`.
- Serves a little **wiring studio** (`http://127.0.0.1:8787`).

## The wiring studio (what your neighbour's Xcode app gets)

1. Open a terminal in this folder and run `npm run dev` (builds the Rust SDK,
   boots the server, opens the browser).
2. Point the studio at your neighbour's Swift app folder.
3. The studio finds `hifzsdk.swift` in it.
   - Empty file → fills it with the complete `HifzSDK` Swift wrapper that loads
     `quran.json` from the app bundle (the mushaf stays in JSON, never in Swift).
   - File with existing code → splices a one-line `HifzSDK` header onto line 1
     and ships `quran.json` beside it, leaving the neighbour's code untouched.
4. `quran.json` (whole mushaf + plan) is copied next to `hifzsdk.swift` so the
   Swift app can bundle it as a resource.

## Run

```bash
npm run dev          # build + server + open browser
cargo run --build-data   # just regenerate data/quran.json
```

## Layout

```
data/quran.base.json   original mushaf (identity data, never edited)
data/quran.json        generated: mushaf + meta + hifz_plan
src/                   Rust: parse → chunk → plan + axum web server
web/index.html         wiring studio UI
package.json           npm run dev → cargo build + run + open browser
```

## License

MIT — see [LICENSE](LICENSE).
