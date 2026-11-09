<div align="center">

# infinitas-utilities

**English** | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md)

A streaming and play companion for beatmania IIDX INFINITAS (Windows x64).

[![CI](https://github.com/naruko-studio/infinitas-utilities/actions/workflows/ci.yml/badge.svg?branch=development)](https://github.com/naruko-studio/infinitas-utilities/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/naruko-studio/infinitas-utilities?include_prereleases&sort=semver)](https://github.com/naruko-studio/infinitas-utilities/releases)
[![Downloads](https://img.shields.io/github/downloads/naruko-studio/infinitas-utilities/total)](https://github.com/naruko-studio/infinitas-utilities/releases)
[![License: MIT](https://img.shields.io/github/license/naruko-studio/infinitas-utilities)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D6?logo=windows)](#)

[![Rust](https://img.shields.io/badge/Rust-2024-000000?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte)](https://svelte.dev/)
[![AI-assisted](https://img.shields.io/badge/AI--assisted-Claude%20Code-D97757?logo=claude)](#ai-assistance)

[![Stars](https://img.shields.io/github/stars/naruko-studio/infinitas-utilities?style=flat)](https://github.com/naruko-studio/infinitas-utilities/stargazers)
[![Issues](https://img.shields.io/github/issues/naruko-studio/infinitas-utilities)](https://github.com/naruko-studio/infinitas-utilities/issues)
[![Contributors](https://img.shields.io/github/contributors/naruko-studio/infinitas-utilities)](https://github.com/naruko-studio/infinitas-utilities/graphs/contributors)
[![Last commit](https://img.shields.io/github/last-commit/naruko-studio/infinitas-utilities/development)](https://github.com/naruko-studio/infinitas-utilities/commits/development)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](https://github.com/naruko-studio/infinitas-utilities/pulls)

</div>

> [!NOTE]
> Work in progress (P2: OBS and phone pages). Not usable yet. See [docs/PLAN.md](docs/PLAN.md) (Traditional Chinese) for scope and progress.

## Features (planned)

- Keystroke counter: today's totals across 1P / 2P / DP; the day rolls over at 05:00 JST, when daily maintenance starts
- Result and music-select screen recognition, score database and score viewer
- Automatic result screenshots (by default only when a record is broken)
- HTML pages for OBS browser sources (today's plays, stats, receipt…), also viewable from a phone on the LAN
- OBS recording control: one file per song, renamed by title, difficulty and clear lamp
- In-game overlay, RANDOM lane recognition
- UI languages: English, 繁體中文, 日本語

## How it works

Everything comes from **screen capture and image processing**. The game's memory and files are never read or modified, and nothing is reverse engineered.

Frames come from OBS through Spout2:

1. Install the Spout2 plugin for OBS.
2. Add a **Spout Filter** to the Game Capture source of `bm2dx.exe`, at the top of its filter list (or as its only filter). The default sender name is `Spout_OBS_Filter`.
3. In the game's ADVANCED GAME OPTION SETTING, set 「判定の数リアルタイム表示」 (live judge count) to ON.

Only 1920×1080 is supported for now. Other resolutions are open for contributions (see PLAN 4.1.1).

### OBS and phone

The app serves pages on `http://localhost:8767` (the main screen lists the URLs with copy buttons):

| Page | Shows | OBS source size |
| --- | --- | --- |
| `/obs/counter.html` | Today's keystrokes and plays (OBS browser source) | 400 × 100 |
| `/obs/judge.html` | The current song's judge counts and ratio bar; `?scope=today` for today's totals | 300 × 270 |
| `/` | Phone page: today, the current song, today's plays, recent days | — |

OBS pages have a transparent background; fonts, colors and size can be changed with CSS variables (`--iu-size`, `--iu-color`…) in the browser source's Custom CSS. To use the phone page, turn on "Allow phones on the LAN to connect" on the main screen and open the address it shows. The pages are read-only and have no password, so only do this on a home network. The sizes above fit the default styling; scale them with `--iu-size`. Labels follow the PC's language, or add `?lang=en`, `zh-TW` or `ja`. The phone page has its own language menu.

## Development

Requires Rust (stable, MSVC), Node.js LTS and pnpm.

```powershell
cd app
pnpm install
pnpm tauri dev                 # run the app

# from the repository root
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo resources               # labels → recognition resources → screenshot tests
```

### Where data is stored

| File | Location |
| --- | --- |
| Settings `config.json`, database `data.db` | `%APPDATA%\studio.naruko.infinitas-utilities\` |
| Logs | `%LOCALAPPDATA%\studio.naruko.infinitas-utilities\logs\` |

`data.db` is SQLite (WAL mode). To browse it, use [HeidiSQL](https://www.heidisql.com/), [DataGrip](https://www.jetbrains.com/datagrip/) (free for non-commercial use), [Beekeeper Studio Community Edition](https://www.beekeeperstudio.io/) or [DbGate](https://dbgate.org/). It can be opened while the app runs, but please don't modify it.

### Screenshot regression tests

Game screenshots are © Konami and are **not in this repository**. Tests read them from
`<Pictures>\INFINITAS Screenshots\dev-capture\<width>x<height>\<screen>\`
and are skipped when that folder is missing (as on CI).

- `IU_FIXTURES_DIR`: use another folder.
- `IU_REQUIRE_FIXTURES=1`: fail instead of skipping, so they are not skipped by accident locally.

## Acknowledgements

This project builds on ideas from:

- [dj-kata/inf_daken_counter_obsw](https://github.com/dj-kata/inf_daken_counter_obsw) (Apache-2.0): the main reference for features and overall design.
- [kaktuswald/inf-notebook](https://github.com/kaktuswald/inf-notebook): approach to result and music-select recognition. **Algorithms only**; none of its code or recognition resources are used, converted or copied.
- [yuki-RAI01/RandomLaneAnalysis_OCR](https://github.com/yuki-RAI01/RandomLaneAnalysis_OCR): approach to RANDOM lane recognition. **Algorithms only**; none of its code or resources are used.
- [Spout2](https://github.com/leadedge/Spout2) (BSD-2-Clause): frame capture.
- [BEMANIWiki](https://bemaniwiki.com/?beatmania+IIDX+INFINITAS/%E5%85%A8%E6%9B%B2%E3%83%AA%E3%82%B9%E3%83%88): source of the song list (`resources/songs.json`: titles, artists, chart levels and note counts).
- [Tauri](https://tauri.app/), [Svelte](https://svelte.dev/).

All recognition masks and lookup tables are generated by this project's `tools/resgen` from our own labelled screenshots.

## AI assistance

This project is developed with the help of AI: [Claude Code](https://claude.com/claude-code) (Anthropic) writes much of the code and documentation. The maintainer sets the direction, reviews the changes and verifies them on real hardware. Commits written with AI help carry a `Co-Authored-By: Claude` trailer.

## Disclaimer

This project is not affiliated with or endorsed by KONAMI. beatmania IIDX and INFINITAS are trademarks of KONAMI.

## License

[MIT](LICENSE). Third-party licenses and attributions are in [NOTICE](NOTICE).
