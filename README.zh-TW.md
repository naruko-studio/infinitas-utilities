<div align="center">

# infinitas-utilities

[English](README.md) | **繁體中文** | [日本語](README.ja.md)

beatmania IIDX INFINITAS 的直播／遊玩輔助工具（Windows x64）。

[![CI](https://github.com/naruko-studio/infinitas-utilities/actions/workflows/ci.yml/badge.svg?branch=development)](https://github.com/naruko-studio/infinitas-utilities/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/naruko-studio/infinitas-utilities?include_prereleases&sort=semver)](https://github.com/naruko-studio/infinitas-utilities/releases)
[![Downloads](https://img.shields.io/github/downloads/naruko-studio/infinitas-utilities/total)](https://github.com/naruko-studio/infinitas-utilities/releases)
[![License: MIT](https://img.shields.io/github/license/naruko-studio/infinitas-utilities)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D6?logo=windows)](#)

[![Rust](https://img.shields.io/badge/Rust-2024-000000?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte)](https://svelte.dev/)
[![AI-assisted](https://img.shields.io/badge/AI--assisted-Claude%20Code-D97757?logo=claude)](#ai-輔助開發)

[![Stars](https://img.shields.io/github/stars/naruko-studio/infinitas-utilities?style=flat)](https://github.com/naruko-studio/infinitas-utilities/stargazers)
[![Issues](https://img.shields.io/github/issues/naruko-studio/infinitas-utilities)](https://github.com/naruko-studio/infinitas-utilities/issues)
[![Contributors](https://img.shields.io/github/contributors/naruko-studio/infinitas-utilities)](https://github.com/naruko-studio/infinitas-utilities/graphs/contributors)
[![Last commit](https://img.shields.io/github/last-commit/naruko-studio/infinitas-utilities/development)](https://github.com/naruko-studio/infinitas-utilities/commits/development)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](https://github.com/naruko-studio/infinitas-utilities/pulls)

</div>

> [!NOTE]
> 開發中（P2 OBS 和手機頁面），目前還不能實際使用。功能範圍和進度見 [docs/PLAN.md](docs/PLAN.md)。

## 預計功能

- 打鍵計數：今日累計，支援 1P／2P／DP；在日本時間 05:00（每日維護開始）換日
- 結算和選曲畫面辨識、成績資料庫、成績檢視器
- 結算圖片自動儲存（預設只在刷新紀錄時存）
- 給 OBS 瀏覽器來源用的 HTML 畫面（今日紀錄、統計、收據…），手機也能在區網內查看
- OBS 錄影控制：每首歌一個檔，結束後依曲名、難度、燈號自動改名
- 遊戲畫面疊加、RANDOM 配置辨識
- 介面語言：English、繁體中文、日本語

## 運作方式

只透過**擷取畫面加上影像處理**取得資訊，不讀取或修改遊戲的記憶體和檔案，也不做逆向工程。

畫面來源是 OBS 的 Spout2 濾鏡，需要：

1. OBS 安裝 Spout2 外掛。
2. 在擷取 `bm2dx.exe` 的遊戲擷取來源加上 **Spout Filter**，放在濾鏡的最上面（或只放這一個濾鏡）。預設送出名稱是 `Spout_OBS_Filter`。
3. 在遊戲的 ADVANCED GAME OPTION SETTING 把「判定の数リアルタイム表示」設為 ON。

目前只支援 1920×1080。其他解析度歡迎協作者發 PR（見 PLAN 4.1.1）。

### OBS 和手機

App 會在 `http://localhost:8767` 提供網頁（主畫面會列出網址，並附複製按鈕）：

| 頁面 | 內容 | OBS 來源建議大小 |
| --- | --- | --- |
| `/obs/counter.html` | 今日打鍵數和遊玩次數（OBS 瀏覽器來源） | 400 × 100 |
| `/obs/judge.html` | 這首歌的判定數和比例條；加 `?scope=today` 改顯示今日合計 | 300 × 270 |
| `/` | 手機頁：今日合計、這首歌、今天每首的紀錄、最近幾天 | — |

OBS 頁背景透明，字型、顏色、大小可以在瀏覽器來源的「自訂 CSS」用 CSS 變數（`--iu-size`、`--iu-color`…）調整。要用手機看，請在主畫面勾選「允許區網內的手機連線」，再用手機打開畫面上顯示的網址。頁面只能查看、沒有密碼，請只在家裡的網路使用。上表的大小是預設樣式下的建議值，改了 `--iu-size` 就等比例調整。OBS 頁的文字跟著電腦的語言，也可以在網址加 `?lang=en`、`zh-TW` 或 `ja` 指定；手機頁右上角可以選語言。

## 開發

需要 Rust（stable，MSVC）、Node.js LTS、pnpm。

```powershell
cd app
pnpm install
pnpm tauri dev                 # 啟動 App

# 在 repo 根目錄
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo resources               # 檢查標註 → 產生辨識資源 → 截圖回歸測試
```

### 資料存放位置

| 檔案 | 位置 |
| --- | --- |
| 設定 `config.json`、資料庫 `data.db` | `%APPDATA%\studio.naruko.infinitas-utilities\` |
| log | `%LOCALAPPDATA%\studio.naruko.infinitas-utilities\logs\` |

`data.db` 是 SQLite（WAL 模式）。檢視時可以用 [HeidiSQL](https://www.heidisql.com/)、[DataGrip](https://www.jetbrains.com/datagrip/)（非商業用途免費）、[Beekeeper Studio Community Edition](https://www.beekeeperstudio.io/) 或 [DbGate](https://dbgate.org/)。App 執行中也可以開，但請不要修改資料。

### 截圖回歸測試

遊戲截圖的著作權屬於 Konami，**不放進 repo**。測試會從
`<圖片資料夾>\INFINITAS Screenshots\dev-capture\<寬>x<高>\<畫面>\` 讀取截圖；
找不到資料夾時會略過（CI 就是這種情況）。

- `IU_FIXTURES_DIR`：改用其他資料夾。
- `IU_REQUIRE_FIXTURES=1`：找不到截圖時視為失敗，避免在本機不小心略過。

## 銘謝

本專案參考了以下專案，在此致謝：

- [dj-kata/inf_daken_counter_obsw](https://github.com/dj-kata/inf_daken_counter_obsw)（Apache-2.0）：功能範圍和整體設計的主要參考。
- [kaktuswald/inf-notebook](https://github.com/kaktuswald/inf-notebook)：參考結算、選曲畫面辨識的做法。**只參考演算法**，沒有使用、轉檔或複製它的程式碼和辨識資源。
- [yuki-RAI01/RandomLaneAnalysis_OCR](https://github.com/yuki-RAI01/RandomLaneAnalysis_OCR)：參考 RANDOM 配置辨識的做法。**只參考演算法**，沒有使用它的程式碼和資源。
- [Spout2](https://github.com/leadedge/Spout2)（BSD-2-Clause）：擷取畫面。
- [BEMANIWiki](https://bemaniwiki.com/?beatmania+IIDX+INFINITAS/%E5%85%A8%E6%9B%B2%E3%83%AA%E3%82%B9%E3%83%88)：曲目主表（`resources/songs.json`：曲名、演出者、各譜面的等級和 NOTES 數）的資料來源。
- [Tauri](https://tauri.app/)、[Svelte](https://svelte.dev/)。

辨識用的遮罩和查找表，都是用本專案的 `tools/resgen` 從自己標註的截圖產生。

## AI 輔助開發

本專案使用 AI 輔助開發：大部分程式碼和文件由 [Claude Code](https://claude.com/claude-code)（Anthropic）撰寫，維護者負責決定方向、審查修改，並在實機上驗證。有 AI 參與的 commit 都會附上 `Co-Authored-By: Claude`。

## 聲明

本專案與 KONAMI 無關，也未受其認可。beatmania IIDX 和 INFINITAS 是 KONAMI 的商標。

## 授權

[MIT](LICENSE)。第三方的授權和出處見 [NOTICE](NOTICE)。
