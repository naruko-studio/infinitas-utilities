# infinitas-utilities

beatmania IIDX INFINITAS 的直播／遊玩輔助工具（Windows amd64）。

> 開發中（P0 初始化）。目前還不能實際使用，功能和進度見 [docs/PLAN.md](docs/PLAN.md)。

## 預計功能

- 打鍵計數（當日累計、1P／2P／DP）
- 結算和選曲畫面辨識、成績資料庫、成績檢視器
- 結算圖片自動儲存（預設只在刷新紀錄時存）
- 給 OBS 瀏覽器來源用的 HTML 畫面（今日紀錄、統計、收據…）
- OBS 錄影控制：每首歌一個檔，結束後依曲名、難度、燈號自動改名
- 遊戲畫面疊加、手機 HTTP 伺服器、RANDOM 配置辨識
- 介面語言：繁體中文、日本語、English

## 運作方式

只透過**擷取畫面加上影像處理**取得資訊，不讀取或修改遊戲的記憶體和檔案，也不做逆向工程。

畫面來源是 OBS 的 Spout2 濾鏡，需要：

1. OBS 安裝 Spout2 外掛。
2. 在擷取 `bm2dx.exe` 的遊戲擷取來源加上 **Spout Filter**，放在濾鏡的最上面（或只放這一個濾鏡）。預設送出名稱是 `Spout_OBS_Filter`。
3. 遊戲內打開「判定數即時顯示」（Start → E2）。

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
```

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
- [Tauri](https://tauri.app/)、[Svelte](https://svelte.dev/)。

辨識用的遮罩和查找表，都是用本專案的 `tools/resgen` 從自己標註的截圖產生。

## 聲明

本專案與 KONAMI 無關，也未受其認可。beatmania IIDX 和 INFINITAS 是 KONAMI 的商標。

## 授權

[MIT](LICENSE)。第三方的授權和出處見 [NOTICE](NOTICE)。
