# infinitas-utilities 實作計畫

Repo：`naruko-studio/infinitas-utilities`

beatmania IIDX INFINITAS 的直播／遊玩輔助工具（Windows amd64）。
功能參考 [dj-kata/inf_daken_counter_obsw](https://github.com/dj-kata/inf_daken_counter_obsw)（Python + PySide6），本專案以 **Rust + Tauri v2** 重新實作。

做法一律是「擷取畫面再做影像處理」，不讀取或修改遊戲記憶體，也不做逆向工程。

## 1. 範圍

### 要做

| # | 功能 | 參考專案對應 |
| --- | --- | --- |
| F1 | 打鍵計數：讀取判定的即時顯示，算出當日 PG/GR/GD/BD/PR 合計和打鍵數；自動判斷 SP/DP/1P/2P、DBx 選項 | `screen_reader.py`、`notes_counter.pyw` |
| F2 | 畫面狀態判斷（選曲／遊玩／結算／選項／其他），只在遊玩中計數 | `screen_reader.is_*` |
| F3 | 抓遊戲畫面：**只用 Spout2**（OBS 的 Spout2 外掛濾鏡送出畫面）。**解析度依 Spout 畫面大小自動判斷**，再選對應的辨識資源組 | `spout_capture.py` |
| F4 | 辨識曲名和結算、選曲畫面；成績 DB（遊玩紀錄＋最佳成績）、F7 從選曲畫面登錄、從資料夾 PNG 匯入 | `result.py`、`result_database.py`、inf-notebook `recog.py` |
| F5 | 結算圖片儲存：自動（**預設：刷新紀錄時才存**，另有每次存／不存）、F6 手動；馬賽克、裁切、寫入統計 | `image_save_settings.py`、`result_stats_writer.py` |
| F6 | 成績檢視器：依模式／等級列出最佳成績、單曲遊玩紀錄、刪除誤登錄 | `score_viewer.py` |
| F7 | OBS HTML 畫面＋WebSocket 資料伺服器（port 8767），可用 CSS 變數自訂 | `websocket_server.py`、`template/*.html` |
| F8 | OBS 錄影控制：連上 OBS WebSocket 後，依畫面切換的時機切換場景、開始／停止錄影。**預設每首歌一個檔**（遊玩開始時開始錄，結算畫面結束時停止；中途離開、沒有進結算的話，回到選曲畫面時停止）。停止後用 `StopRecord` 回傳的路徑，依可自訂的格式改檔名（預設格式見 5.2）。不做來源顯示／隱藏和自動截圖 | `obs_control.py`、`obs_websocket_manager.py` |
| F9 | 遊戲畫面疊加：滑鼠可穿透、不會被擷取拍到（`WDA_EXCLUDEFROMCAPTURE`）的置頂視窗 | `game_overlay.py` |
| F10 | RANDOM 配置辨識（SP、Spout 1080p）：先用正規／MIRROR 登錄譜面，再推算 RANDOM／R-RANDOM 配置 | `random_lane_analysis.py`、`random_lane_templates.py` |
| F11 | 手機 HTTP 伺服器（port 8787）：瀏覽成績、遊玩紀錄，選結算圖片拼接 | `mobile_http_server.py` |
| F12 | 單手難度表（SP☆11/12；資料包含在曲目主表裡） | `misc/update_katate_difficulty.py` |
| F13 | 多語系（繁中／日／英）＋自動更新 | `ui_jp.py`/`ui_en.py`、`update.py` |

### 不做

- VOICEVOX 語音提示
- 對手比較、inf_score.csv 共享、BPI CSV 輸出、挑戰模式
- 發推／今日成果推文
- 匯入 v2 的 `alllog.pkl`、匯入參考專案 v3 的 `playlog.infdc` 和 `config.json`（舊資料只能從結算圖片資料夾匯入）
- OBS 的來源顯示／隱藏、OBS 端自動截圖
- GDI、DXGI、OBS WebSocket 抓畫面（只用 Spout2）
- 段位認定（暫時不做，之後再加）。段位的結算畫面不記錄，也不觸發錄影規則，以免誤判。遊玩畫面如果和一般遊玩一樣能辨識出判定面板，打鍵數照算（鍵確實按了）；能不能辨識，等之後有段位截圖再確認

> 因為不做 BPI，HTML 畫面和統計圖裡的 BPI 欄位、BPI 篩選（`--bpi-min`、`bpi_only`）都會拿掉。

## 2. 架構

```
infinitas-utilities/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ iu-core/                # 共用型別：PlayMode、Judge、Result、Difficulty、Lamp…
│  ├─ iu-capture/             # Spout2 接收（FFI）、畫面大小偵測；保留 trait，方便測試時用截圖檔替代
│  ├─ iu-recog/               # 畫面狀態、判定數字、結算／選曲辨識、選項辨識
│  ├─ iu-random-lane/         # RANDOM 配置登錄和推算
│  ├─ iu-db/                  # SQLite（rusqlite）、migration、匯入
│  ├─ iu-imaging/             # 結算圖片加工（image、imageproc、ab_glyph）
│  ├─ iu-obs/                 # OBS WebSocket 客戶端和錄影規則（obws）
│  └─ iu-server/              # axum：一個 port（8767）提供 OBS 頁、手機頁、WebSocket、唯讀 API
├─ app/                       # Tauri v2
│  ├─ src-tauri/              # 主迴圈、狀態機、command、托盤、熱鍵、疊加視窗
│  └─ ui/                     # Svelte 5 + TS（設定畫面、主畫面、成績檢視器）
├─ views/                     # 給 OBS 用的 HTML/CSS/純 JS（透過 WS 收資料）
├─ resources/                 # 自己產生的辨識資源（JSON，有版本號）和曲目主表 songs.json
├─ .github/workflows/         # CI、發佈
└─ tools/
   └─ resgen/                 # 從標註資料產生資源的 CLI
```

### 使用前提和 Spout 設定

使用者的環境（2026-11-06 確認）：OBS 33.0.1；場景「IIDX INFINITAS」→ 遊戲擷取來源「INFINITAS」（模式：擷取特定視窗 `[bm2dx.exe]`）→ 效果濾鏡只有 **Spout Filter**，送出名稱 `Spout_OBS_Filter`。這和參考專案建議的設定相同，App 預設照這個設定。

- **Spout 送出名稱**：預設 `Spout_OBS_Filter`，可以在設定裡改。最好列出目前所有的 Spout 送出端讓使用者用選的，不用手打。
- **濾鏡順序**：Spout Filter 送出的是「這個來源經過它上面那些濾鏡之後」的畫面，和場景裡的縮放、位置無關。如果在它上面加了色彩校正、裁切等濾鏡，辨識就會壞掉。README 要寫明：**Spout Filter 放在最上面，或者是唯一的濾鏡**。
- **送出的必須是遊戲畫面本身**：不是整個場景或直播版面，解析度要和遊戲一致。
- **遊戲內設定**：打鍵計數需要在 ADVANCED GAME OPTION SETTING 把「判定の数リアルタイム表示」設為 ON。偵測到遊玩畫面卻讀不到判定時，主畫面要提示這一點。
- **斷線重連**（照參考專案的做法）：接收出錯時，2 秒後重試；連線中但 5 秒沒有新影格，就重建接收端；OBS 沒開時進入等待狀態，在主畫面顯示原因。
- **HDR**：Windows HDR 打開時，擷取到的顏色可能會變。像素比對驗證時要確認這一點，必要的話要求使用者用 SDR。

### 執行模型

- **擷取執行緒**：照設定的 FPS（預設約 30）抓畫面，交給辨識執行緒。
- **辨識執行緒**：判斷畫面狀態，再依狀態做對應辨識，送出 `Event`（`PlayStart`、`JudgeUpdate`、`ResultConfirmed`…）。
- **事件匯流排**（`tokio::sync::broadcast`）：DB 寫入、OBS 錄影規則、WS 推送、疊加視窗、圖片儲存都只聽事件，彼此不直接呼叫。
- **前端**：只透過 Tauri command 和 event 溝通，不碰辨識邏輯。推送頻率要節流（判定約 10Hz）。

### 關鍵技術選擇

| 項目 | 選擇 | 理由／備註 |
| --- | --- | --- |
| Win32 API | `windows` crate | 找遊戲視窗（疊加視窗要跟著它）、`SetWindowDisplayAffinity`、鍵盤 hook |
| Spout2 | **自己用 Win32 共享記憶體 + D3D11 實作接收端**（`iu-capture`） | 唯一的抓畫面方式。讀 `SpoutSenderNames` 和送出端資訊，用 `OpenSharedResource` 打開共享貼圖再複製到 staging 貼圖讀出。不用附 `SpoutLibrary.dll`，也不用透過 FFI 呼叫 C++ vtable；只參考 Spout2（BSD-2）的協定，沒有複製程式碼。實測（OBS 33 的 Spout Filter）：1920×1080 BGRA，讀一張約 3.5 ms；**這個送出端沒有畫面計數 semaphore**，所以「5 秒沒新影格」要改用畫面內容雜湊判斷。前提是 OBS 有開、裝了 Spout2 外掛，而且遊戲擷取來源加了 Spout 濾鏡；收不到畫面時要在主畫面明確提示 |
| OBS | `obws` | 支援 OBS WebSocket v5；用到 `SetCurrentProgramScene`、`StartRecord`、`StopRecord`，以及錄影狀態事件 |
| 影像處理 | `image` + 自寫的 ndarray 運算 | 參考專案大多是遮罩比對和像素計數，用不到 OpenCV；只有 RANDOM 的範本比對可能要 `imageproc` |
| DB | SQLite（`rusqlite`，bundled） | |
| 全域熱鍵 | `WH_KEYBOARD_LL` 低階鍵盤 hook | F6/F7 要在遊戲有焦點時也能用，而且**不能吃掉按鍵**；`RegisterHotKey` 會吃掉按鍵，不適用 |
| 疊加視窗 | Tauri 透明視窗 + `set_ignore_cursor_events` + `WDA_EXCLUDEFROMCAPTURE` | 跟著遊戲視窗的位置和大小 |
| 前端 | 主程式用 **Svelte 5 + TypeScript + Vite**；OBS 畫面用**純 JS** | 效能優先：Svelte 編譯後沒有 virtual DOM、runtime 很小；OBS 瀏覽器來源越輕越好 |
| 多語系 | 前端用 i18n 字典（zh-TW / ja / en），後端訊息用 key | 依系統語言自動選擇，可以手動切換 |
| 自動更新 | `tauri-plugin-updater` + GitHub Releases（`naruko-studio/infinitas-utilities`） | updater 一定要有一把簽章金鑰（minisign 格式、免費，和 GPG、Authenticode 都無關），用來驗證下載的更新檔沒被竄改 |
| 打包 | Tauri bundler（NSIS） | **exe 不做 Authenticode 簽章**（費用太高）。使用者第一次執行會跳 SmartScreen 警告，README 要寫怎麼略過。之後如果有贊助或低價方案再加（例如 SignPath 的 OSS 免費方案、Azure Trusted Signing） |
| 授權 | **MIT** | `LICENSE` 用 MIT；`NOTICE` 列出參考的 Apache-2.0 程式碼和 Spout2（BSD-2） |

## 3. 開發階段

每個階段都要能單獨跑起來驗證。

| 階段 | 內容 | 完成條件 |
| --- | --- | --- |
| **P0 初始化** | workspace、Tauri 骨架、設定檔（`config.json` + serde，有預設值）、tracing log、i18n 架構、CI（`cargo fmt/clippy/test`）、樣本截圖回歸測試框架 | 空白視窗能啟動；`cargo test` 通過 |
| **P1 打鍵計數 MVP**（F1、F2、F3） | Spout2 接收、解析度偵測、開發者擷取模式、Spout 和 F12 截圖的像素比對（見 4.2）、畫面狀態錨點、判定數字表（自己建）、1P/2P/DP 判斷、當日累計、主畫面顯示 | 標註截圖的判定值 100% 正確；實機遊玩時能正確累計 |
| **P2 HTML 畫面基礎**（F7，含 P4／P8 的共用部分） | 事件匯流排（引擎 → UI／伺服器，補上 P1 欠的架構）；`iu-server`（axum）：WS 推送格式、靜態 HTML、唯讀查詢 API；打鍵數和圖表畫面；`iu-db` 歷史查詢（每日合計、當天每首紀錄）；**手機頁**（今日合計、這首歌、今天每首紀錄、最近幾天的打鍵數）；**區網存取**：預設只聽 localhost，設定打開後才對區網開放，主畫面顯示手機要連的網址（2026-11-09 使用者要求提前） | OBS 瀏覽器來源能即時更新；手機在區網內能即時看到計數和今天的紀錄 |
| **P3a 資源工具鏈** | 標註畫面（App 內的開發者畫面）、`resgen all`（`cargo resources`）、標註格式和一致性檢查、fixtures 回歸測試、曲目主表（本機工具產生） | 一個指令就能從標註資料產生資源並跑完測試 |
| **P3b 結算／選曲辨識**（F4、F5） | 數字、燈號、DJ LEVEL、難度、曲名 trie、選曲畫面；邊玩邊學的未知佇列；擴充 SQLite schema（`plays` 表已在 P1 建立）；F6/F7 熱鍵；圖片儲存和加工 | 已標註曲目的辨識正確率 100%；未知曲目能進佇列並學起來 |
| **P4 檢視與紀錄畫面**（F6、F7） | （伺服器、查詢 API、打鍵數的歷史已在 P2 完成）成績檢視器、`today_result` / `history_cursong` / `today_stats` / `today_receipt`（拿掉 BPI 區塊）/ `whole_layout` 畫面 | |
| **P5 其他解析度**（F3） | 為 1080p 以外的解析度收集截圖、產生資源組；偵測到不支援的解析度時提示使用者（已在 P1 完成）。**開放協作者 PR**（含自動縮放的提案，見 4.1.1） | 各支援解析度都通過回歸測試 |
| **P6 OBS 錄影控制**（F8） | WS 連線設定、「觸發時機 → 動作（切換場景／開始錄影／停止錄影）」規則的編輯和執行（預設套用每首歌一個檔的規則）、錄影檔改名、斷線重連 | 每首歌都能正確開始和停止錄影，檔名和結算內容一致 |
| **P7 遊戲畫面疊加**（F9） | 疊加視窗、跟隨遊戲視窗、不被擷取拍到 | OBS 錄影裡看不到疊加視窗 |
| **P8 手機伺服器**（F11） | （手機頁、區網存取已在 P2 完成）成績瀏覽、選網卡、暫存結算圖、拼接圖片 | |
| **P9 RANDOM 配置**（F10） | 範本、GET READY 偵測、登錄和推算、`random_lane.html`、在疊加視窗和紀錄裡顯示車道編號 | |
| **P10 收尾**（F12、F13） | 單手難度表、補齊三種語言的翻譯、**主程式、OBS 頁、手機頁的 UI/UX 調整**（2026-11-09 決定：開發期間以功能為主，Release 前統一調整）、自動更新、安裝檔 | |

## 4. 辨識方式：參考做法，資源自己做

**方針**：只參考 inf-notebook 和 inf_daken_counter_obsw 的演算法，用 Rust 自己實作。遮罩、查找表、座標都從我們自己收集的截圖產生。**不讀取、不轉檔、不附帶** inf-notebook 的 `.res` 資源。

### 4.1 參考的演算法（原理整理）

| 對象 | 參考專案的做法 | 我們的實作 |
| --- | --- | --- |
| 畫面狀態 | 比對固定位置的像素或小區塊是否等於特定值 | 每種畫面定義幾個「錨點區塊」，取 hash 比對 |
| 判定數字（遊玩中） | 切出 11×11 的數字格，B 通道 >100 二值化後加總，用加總值查表；6 和 9 加總相同，再看 (8,0) 區分 | 同樣的原理；查找表從自己的截圖產生 |
| 分數、BP、notes 數字 | 依位數切開，比對遮罩值後二值化，用位元圖查表 | 同上，字型固定，少量截圖就能建表 |
| 燈號、DJ LEVEL、難度、模式 | 取區塊的主色，或計算遮罩命中數，再查表 | 同上 |
| **曲名** | 依色系（灰／藍／紅）過濾後選像素最多的色系 → 套遮罩 → 每一列取最常出現的亮度做二值化 → 每 4 bit 轉成 hex，`列號+hex` 當 key，**逐列走 trie**，走到葉節點就是曲名 | 同樣的特徵抽取和 trie；trie 用自己標註的資料產生 |
| 選曲畫面 | 曲名（INFINITAS／LEGGENDARIA／AC 收錄三種字型）、等級、燈號、DJ LEVEL、分數，用同樣方式各自建表 | 同上 |
| RANDOM 配置 | RandomLaneAnalysis_OCR：範本比對加上 5040 種排列逐步縮小候選 | 只參考原理自己寫；範本也自己產生，不要求使用者另外裝原工具 |

### 4.1.1 解析度自動判斷

查找表是逐像素比對的，畫面一縮放就對不上。所以**不縮放成 1080p 後再辨識**，而是每種解析度各自一套資源組（`resources/<寬>x<高>/`）：

- 啟動和畫面大小改變時，讀取 Spout 畫面的寬高，載入對應的資源組。
- 第一版只做 1920×1080。其他解析度（例如 1280×720、2560×1440）等收集到該解析度的截圖後再加（P5）。
- 偵測到沒有資源組的解析度時，主畫面明確提示，不硬套。
- **自動縮放（2026-11-09 討論，暫緩）**：版面固定，座標可以等比算出。可行的做法是只把要比對的小區塊縮放回 1080p，再用 1080p 資源辨識。4K（×2）應該很穩，2K（×4/3）應該可行，HD（×2/3）的數字只剩約 7 px，門檻可能要另外調，SD 的數字只剩約 5 px，而且 4:3 會有黑邊，不保證能用。這個做法和「不縮放影像」的原則衝突，**本專案先不做，開放協作者發 PR**。PR 要附各解析度的實機截圖回歸測試，並一併修改 CLAUDE.md 的原則。

### 4.2 資源產生流程

1. **收集截圖**：App 加上「開發者擷取模式」，在各畫面自動存 1920×1080 原始截圖（PNG），並附上當時的畫面狀態。
   - 遊玩中的畫面沒辦法手動截圖，所以 P1 **第一個交付的就是這個模式**：先只做 Spout 接收加上每隔 N 影格存 PNG（還不需要辨識），使用者照常遊玩就能收集到資料。畫面狀態辨識做好之前，先用時間間隔存，之後再人工分類。
   - 使用者手上現有的結算截圖，如果是無損的 PNG 而且是 1920×1080，可以直接拿來建結算畫面的答案表。JPG 或縮放過的圖只能用來找版面位置。
   - **遊戲內 F12 截圖**：存在 `<遊戲安裝資料夾>\screenshots\`（使用者的環境是 `C:\Games\beatmania IIDX INFINITAS\screenshots`），檔名是 `iidx_infinitas_YYYYMMDDhhmmssSSS.png`，格式是 1920×1080 RGBA 無損 PNG。目前有 34 張，全部是 1P SP，2P 和 DP 之後補。annotator 和 F4 的「從資料夾匯入」都可以直接選這個資料夾（路徑可以在設定裡改）。
   - **⚠ 要先驗證**：答案表如果用 F12 截圖做，App 實際收到的 Spout 畫面必須和它**逐像素一致**。OBS 那一段可能會改到像素，例如色彩空間轉換、縮放、HDR。P1 要先在同一個結算畫面同時取得 F12 截圖和 Spout 影格，做像素比對：
     - 完全一致：F12 截圖可以直接當資料用。
     - 不一致：答案表只用 Spout 影格做（或在比對前先做轉換），F12 截圖只拿來找版面；README 也要寫出 OBS 這邊需要的設定。
     - **結果（2026-11-07，已驗證）：一致。** 選曲畫面：連續 2 秒的 Spout 影格裡始終不變的像素（約 61 萬個，排除動態背景和半透明面板）和同一畫面的 F12 截圖比對，**99.973% 完全相同**，145 個差 1，19 個差更多（應是當下畫面的小變化），平均色差約 0（R +0.0007、G 0、B −0.0002），沒有系統性色偏。F12 截圖可以直接當資料用。辨識仍然用門檻判斷，不做「完全相等」比對，以容忍 ±1 的誤差。
     - 工具：`iu-capture` 的範例 `static_diff`（只比靜態像素）、`spout_vs_f12`（等新的 F12 截圖再比）。程式送出的 F12 遊戲不接受，要使用者實際按。
2. **標註工具**（App 內的開發者畫面，打開開發者擷取模式後出現「標註」分頁；原本規劃的 `tools/annotator` 改成這樣，2026-11-09 使用者決定）：對截圖標上正確答案（曲名、難度、分數…）。
   - **半自動標註**：結算畫面的數字之間有固定關係，可以用來自動檢查。字型表建好之後，數字類的欄位由工具先填，人只要確認曲名就好。
     - `EX SCORE = PGREAT×2 + GREAT`（4 個樣本全部成立）
     - `MISS COUNT = BAD + POOR`（例：14+84 = 98）
     - **`NOTES = PG + GR + GD + CB`（COMBO BREAK）**（使用者提案）。CB 是 BAD 加上「打到音符的 POOR」，不含空 POOR，所以可以用等號。4 個樣本全部成立，**FAILED 也成立**（落掉之後剩下的音符也算進 POOR／CB）：
       - SP 1P F-COMBO：193+138+89+0 = 420 ✓
       - SP 2P FAILED：122+101+108+89 = 420 ✓
       - DP 2P A-CLEAR：85+77+226+37 = 425 ✓
       - DP 1P FAILED：71+88+198+68 = 425 ✓
     - 上面兩個 FAILED 都**沒有 DEAD POINT 欄**，是打到最後但血條不夠的 FAILED，剩下的音符都算進 CB。
     - **關門（結算畫面出現 DEAD POINT 欄）時，這條規則不成立**：判定只算到停下來的那一刻。HARD／EX-HARD 血條歸零，或自己中途退出，都會出現 DEAD POINT。
       - **關門時改用 `PG+GR+GD+CB = DEAD POINT 的 NOTE` 檢查（已確認）**：
         - EXH 中途斷：14+8+14+6 = 42 = NOTE 0042 ✓（MEASURE 0012；EX SCORE 14×2+8 = 36 ✓）
         - HARD 中途斷：0+0+0+14 = 14 = NOTE 0014 ✓（MEASURE 0005）
         - 一開始就斷：0 = NOTE 0000 ✓（MEASURE 0001）
       - DEAD POINT 欄的位置：1P 在圖表欄的右邊（約 x 565–775、y 280–355）。
     - 用途：結算時用來交叉檢查辨識結果；曲目主表沒有這首歌的 NOTES 時，也可以從結算畫面反推，自動補進去。遊玩中的判定面板也有 CB，所以打鍵計數也能用同一條規則檢查。
   - 結算畫面的版型目錄見 4.5。
3. **產生資源** `tools/resgen`（Rust CLI）：從標註資料產生遮罩、查找表、曲名 trie，輸出 `resources/<寬>x<高>/*.json`，附上版本號。**格式用 JSON 而不是原本規劃的 msgpack**：目前的資源很小（play.json 約 5 KB），JSON 在 git diff 裡看得出改了什麼，出問題也容易查；之後曲名 trie 等資源太大時，再個別改成 msgpack。
4. **回歸測試**：同一批標註資料（`dev-capture\`，見 6.2）也是回歸測試的資料。產生資源後馬上全部重新辨識一次，確認結果正確。

### 4.3 曲名的難題和對策

曲名 trie 要對每一首歌都有標註樣本，總數約 2000 個譜面規模。單靠開發端收集不完，所以加上**使用者端邊玩邊學**：

- 辨識不出曲名時，把特徵 key 和截圖存進「未知佇列」。
- 主畫面提示使用者從曲目清單選出正確曲名（可以用等級和 notes 數縮小候選）。
- 選好後寫進本機的學習表 `learned_titles`，下次就能直接辨識。
- 學習表可以匯出和匯入，之後能合併進官方資源，跟著更新一起發佈。

### 4.4 曲目主表

**資料來源：BEMANIWiki 的全曲リスト和総ノーツ数リスト**，用本機工具 `tools/songlist` 轉成 `resources/songs.json`（格式見 5.1）。**這個工具只放在使用者的電腦上，不進 repo**（2026-11-09 使用者決定）：它是獨立的 Cargo workspace，repo 的 `Cargo.lock` 不含它的依賴。只取事實資料，NOTICE 和 README 註明出處。曲單更新時在 `tools/songlist` 裡跑 `cargo run --release -- fetch`。

### 4.5 版型目錄（從實際截圖整理，2026-11-07）

樣本：`C:\Games\beatmania IIDX INFINITAS\screenshots` 裡的 Shooting Star [N4]，共 8 張，涵蓋 SP 1P／SP 2P／DP（資訊欄在左、在右）的遊玩畫面和結算畫面。座標以 1920×1080 為準，是目測的大約值，正確值由 `resgen` 從標註資料定出。

**遊玩畫面：判定面板**（打鍵計數用；`PG/GR/GD/BD/PR/CB` 加上 FAST/SLOW）

| 版型 | 面板位置（約） | 面板裡的排列 |
| --- | --- | --- |
| SP 2P | x 640–915、y 940–1065（下方，BPM 右邊） | 偏移值和 FAST/SLOW 在左，判定數字在右 |
| DP，資訊欄在右 | x 1600–1875、y 865–1000（右下） | JUDGE 和判定數字在左，FAST/SLOW 在右 |
| DP，資訊欄在左 | x 50–325、y 865–1000（左下） | 和 SP 2P 同一種排法 |
| SP 1P | x 1005–1280、y 940–1065（下方，BPM 左邊） | 和「DP，資訊欄在右」同一種排法 |
| SP 1P，圖表和 BGA 互換（圖表在最右邊） | x 590–865、y 940–1065（BPM 左邊，整組往左移了約 415px） | 同上 |

- 判定面板有兩種左右相反的排法，數字欄的位置要分開定義。
- **曲子剛開始時可能還沒出現判定面板**（DP 樣本裡同一首歌有一張沒有面板的）。判斷「遊玩中」不能只靠判定面板，要另外用 BPM 框、GROOVE GAUGE 等固定元素當錨點。
- SP 的版面另有 GRAPH INFORMATION（YOU／PACEMAKER 分數、與目標的差距），之後可以考慮拿來即時顯示分數，第一版不做。
- SP 的 1P／2P 版面左右相反：1P 是車道在左、圖表在中間、BGA 在右；2P 是 BGA 在左、圖表在中間、車道在右。
- **SP 的 GRAPH INFORMATION 和 BGA 可以在遊戲設定裡互換位置**（使用者提醒）。兩者的分辨方式：**有柱狀圖的是 GRAPH INFORMATION，沒有的是 BGA**。GRAPH INFORMATION 上方有橘色標題列，下方有「現在のスコア／自己ベスト」按鈕，這些都可以當錨點。
- **所以判定面板不寫死座標**，改用「版面偵測」：
  1. 進入遊玩畫面時，在畫面下方那一條（約 y 860–1070）搜尋判定面板的 `JUDGE` 字樣和面板外框，找出面板位置，以及它是哪一種排法（左右相反兩種）。
  2. 同時找出車道、GRAPH INFORMATION、BGA 各在哪裡，記成這首歌的「版型」。
  3. 版型找到後就固定使用，直到離開遊玩畫面，不用每格重找。
  4. 上表的座標當成第一批候選位置，先比對這些位置，比對不到才整條搜尋。
- **2P 基本上就是 1P 的左右鏡像**（使用者確認）。座標表可以只標 1P，2P 用 `x' = 寬 − x − w` 推出來，再用 2P 樣本驗證。
- 參考專案（inf_daken_counter_obsw）的做法：寫死 8 種模式（SP 1P／2P × 圖表在左／右／**不顯示圖表**、DP 左／右）的 `JUDGE` 字樣座標，每格逐一比對 average hash 判斷是不是遊玩畫面、是哪種模式。「不顯示圖表」指 GRAPH INFORMATION 整塊不顯示（和 BGA 開關無關）；使用者在遊戲裡沒找到這種設定，**先不做**，之後真的遇到再加。
- **BGA 開關不影響版面**（使用者確認，附他人直播截圖）：關掉 BGA 只是把 BGA 區換成一張靜態圖（INFINITAS 標誌），框線和其他區塊的位置不變。BGA 區的內容不管開或關都不能當錨點。
- **P1 實作結果（2026-11-07，11-09 補上 sp-2p-graph-outer）**：`resgen play` 從 22 張標註截圖算出的錨點（`resources/1920x1080/play.json`）：

  | 版型 | `JUDGE` 字樣左上 | 第一個數字格 | 樣本數 |
  | --- | --- | --- | --- |
  | sp-1p-graph-inner | (1014, 942) | (1049, 969) | 15 |
  | sp-1p-graph-outer | (597, 942) | (632, 969) | 2 |
  | sp-2p-graph-inner | (834, 942) | (852, 969) | 1 |
  | sp-2p-graph-outer | (1251, 942) | (1269, 969) | 1 |
  | dp-info-right | (1608, 871) | (1643, 898) | 1 |
  | dp-info-left | (244, 871) | (262, 898) | 2 |

  sp-2p-graph-outer 的字樣位置和 sp-1p-graph-outer 左右對稱（1920 − 597 − 73 ≈ 1251）。

  - 數字格 11×11，橫向間距 14、縱向 16，4 欄 × 6 列，靠右對齊。
  - **SP 2P 和 DP 資訊欄在左時，面板內部也左右相反**（`JUDGE` 在右上、LATE／EARLY 在左），所以數字格相對字樣的偏移不同（+18 而不是 +35）。上面「2P 用鏡像推座標」只適用整塊面板的位置，面板內部要另外定義。
  - 判斷遊玩畫面：`JUDGE` 字樣（73×17，二值化）和候選位置的距離。遊玩畫面最大 8，其他 46 張截圖最小 485，門檻 123。整條搜尋只在 `resgen` 用；執行時只比對已知錨點。
  - 數字：0–9 的遮罩用多數決產生。同一數字最大差 5 像素，最像的 6／8 差 13，門檻 5。
  - 回歸測試：22 張遊玩畫面的版型和判定值全對，46 張其他畫面都沒被誤判。**遮罩和測試用的是同一批截圖**，沒看過的畫面要靠實際遊玩和開發者擷取模式補驗證。
  - **換日**：「今日」是遊戲日，在日本時間 05:00 換日（每日定期維護 05:00–07:00 開始的時間，每月第三個星期二會提早到 02:00 開始，但都不會晚於 05:00），所以一個遊玩時段不會被切成兩天，也不受電腦時區影響。設定檔 `day_start_jst` 可以改換日時間，預設 `05:00:00`。`data.db` 的 `plays.game_day` 也是遊戲日。
  - **一首歌的邊界**：只有判定數歸零才算新的一首（包括前一首只有 POOR 就原地重開）。重開時如果剛好沒讀到歸零的畫面，讀數會比目前少；連續 10 格都比目前少、而且彼此一致，就判斷為重開，單一格讀錯則丟棄。遊玩中切換圖表和 BGA 的位置（使用者實際這樣做過）只會更新版型，不會把一首切成兩筆。打完後「這首歌」繼續顯示上一首，直到下一首開始。
  - **儲存**：`iu-db` crate 管理 `data.db`（SQLite，WAL）。每打完一首寫一筆到 `plays`，欄位有遊戲日、開始和結束時間（UTC）、版型、PG–CB。全部判定都是 0 的不寫入。「今日」的合計和遊玩次數都是查詢當天的資料算出來的，不另外存，所以過去每一天的資料都會留著。schema 版本存在 `PRAGMA user_version`，開啟時依序升級；遇到比 App 新的版本就拒絕開啟。原本的 `today.json` 已經廢除（測試期間的資料不轉移）。
  - 實機驗證（2026-11-08，開發者擷取模式）：SP 1P 打了 28 首。有截到結算畫面的 5 首，判定值和結算畫面完全一致；另 1 首用 EX SCORE 對得上選曲畫面的分數。中途重來會正確拆成兩首，讀數沒有任何一筆被丟棄。SP 2P 圖表在內側、DP 資訊欄在右也都在實機上認出來（放著不打）。2026-11-09：DP 資訊欄在左實機打了 1 首，版型和判定值都認對（EX SCORE 換算和畫面上的分數一致）；SP 2P 圖表在外側的樣本也補上了。6 種版型都已經有實機樣本。
- **P2 實作結果（2026-11-09）**：
  - 事件匯流排：`iu-core::event`（`tokio::sync::broadcast`），事件是 `Status`（快照，最多 10 Hz）和 `PlayFinished`。引擎發布，App 主畫面（`engine://status`、`engine://play`）和網頁伺服器都只聽事件。`data.db` 由引擎在發布 `PlayFinished` 前寫入，這樣聽到事件的一方去查 API 時，這一首一定已經在資料庫裡。
  - `iu-server`（axum）：一個 port（預設 8767）提供全部功能。`/` 是手機頁，`/obs/counter.html` 是今日打鍵數，`/obs/judge.html` 是這首歌的判定和比例條（`?scope=today` 改顯示今日合計）。另有 `/ws`（連上時先送一次快照，之後轉送事件）和唯讀 API `/api/status`、`/api/today`、`/api/days`。網頁用純 JS，嵌在執行檔裡；OBS 頁背景透明，可以用 CSS 變數自訂。設定檔 `server.port` 可改 port（舊的 `ws_port` 仍可讀），`server.lan_access` 控制區網存取，預設只聽 127.0.0.1。
  - 實機驗證（2026-11-09）：OBS 瀏覽器來源會即時更新，手機在區網內能即時看到資料。UI/UX 留到 Release 前調整（P10）。
- **P3a 實作結果（2026-11-09，尚未 commit）**：
  - 標註格式：`iu-core::label`。畫面種類就是資料夾名稱（`play`／`result`／`select`／`option`／`other`），JSON 裡只放那種畫面的欄位，所以 P1 的 play 標註照樣能讀。結算畫面的欄位：side、chart、level、songId（對應曲目主表）、title、notes、lamp（和最佳 lamp）、DJ LEVEL、SCORE（和最佳）、MISS COUNT（和最佳；「—」記為 null）、PG–PR、COMBO BREAK、FAST／SLOW、DEAD POINT（NOTE、MEASURE）。
  - 一致性檢查（`label::check`，照 4.2）：SCORE = PG×2+GR、MISS COUNT = BD+PR、PG+GR+GD+CB = NOTES（關門時改為等於 DEAD POINT 的 NOTE）、CB ≥ BD、FULL COMBO 時 CB = 0、選曲畫面 NO PLAY 時沒有 DJ LEVEL。
  - 標註畫面：左邊是截圖清單，來源有 dev-capture 的各資料夾和遊戲的 F12 資料夾（設定檔 `capture.game_screenshot_dir`），可以篩選待標註、全部、各分類。中間顯示截圖，右邊是表單：遊玩畫面用辨識結果預填；曲名從曲目主表選，選了就自動帶入等級和 NOTES；數字不一致時即時列出問題。儲存時會把截圖歸檔到 `dev-capture\<寬>x<高>\<畫面種類>\`：F12 截圖用複製的，遊戲資料夾不會被改；dev-capture 裡的截圖用移動的。「丟棄」會把截圖移到 `discarded\`，不會刪除。只能讀寫這兩個根目錄底下的 PNG。
  - `cargo resources`（= `resgen all`）：先檢查所有標註（解析和一致性），再產生全部資源，最後用 `IU_REQUIRE_FIXTURES=1` 跑截圖回歸測試。
  - 曲目主表：見 4.4、5.1。
- **P3b 進度（2026-11-09，進行中）**：
  - 標註：自己標了 22 張結算截圖（SP 1P／2P、DP 1P／2P、關門 3 張、輔助選項 3 張、有角色的背景 1 張）。和曲目主表交叉檢查等級和 NOTES，全部一致。
  - 一致性規則修正：`PG+GR+GD+CB` 可能**少於** NOTES，但不會超過。例如：A-SCR 這類自動皿的輔助選項，皿的音符不判定（FLOWER：559／575）；CN 提早放開會丟掉尾端判定，卻不算 COMBO BREAK（蠍火 BlackY Remix：519／521）。所以規則改成「不可超過 NOTES」；關門時仍要等於 DEAD POINT 的 NOTE。標註新增 `useOption`（USE OPTION 欄，例：A-SCR、LEGACY）。
  - 結算畫面數字辨識（`iu-recog::result`、`resgen result`）：用 `CLEAR TYPE` 字樣判斷是不是結算畫面，以及是哪一側（1P 在 (41, 421)，2P 在 +1350）。每個數字都是向右對齊、固定間距的數字格。字型有 6 種，每種的同一數字差距都是 0（逐像素完全相同）：

    | 欄位 | 字型（寬×高，間距） | 二值化 |
    | --- | --- | --- |
    | PG–PR | judge 23×15，28 | 每個通道 > 200（`Bright`） |
    | COMBO BREAK | combo 23×15，28（背景是條紋，另一套遮罩） | `Bright` |
    | FAST／SLOW | small 14×12，17 | `Bright` |
    | 這次的 SCORE、MISS COUNT | big-cyan 30×20，36 | 綠藍 > 128（`Cyan`） |
    | 最佳 SCORE、MISS COUNT | big 30×20，36 | `Bright` |
    | NOTES（下方曲名欄） | notes 17×15，21（前置的 0 比較暗，用 `Bright` 排除） | `Bright` |

    - 2P 不是單純平移：大部分欄位 +1349～+1350，FAST／SLOW +1344。所以 2P 的位置由 `resgen` 在 ±8 px 內搜尋。
    - 用 `Bright` 而不是 128 的原因：有角色的結算背景是亮的，128 會把背景也當成數字。
    - NOTES 的樣本裡沒有 9，暫時用 6 旋轉 180° 代替（七段式字型），之後補樣本。
    - 「—」（MISS COUNT、最佳成績）讀成 null。
  - 結果：22 張的版型和所有數字全部讀對；其他畫面離 `CLEAR TYPE` 遮罩至少 687，結算畫面最多 76。標註畫面打開結算截圖時，會用辨識結果預填所有數字。
  - **文字欄位（2026-11-09 晚）**：燈號（這次和最佳）、DJ LEVEL（這次和最佳）、難度和等級、SP／DP 標誌、DEAD POINT 欄是否出現、USE OPTION 文字。做法是固定區域整塊比對，每個類別（例：`failed`、`AA`、`N4`）用標註多數決產生一張遮罩：

    | 欄位組 | 區域（寬×高） | 二值化 | 類別（目前樣本） | 同類最大差／最近的兩類 |
    | --- | --- | --- | --- | --- |
    | 燈號（這次和最佳共用） | 149×18 | 任一通道 > 200（`Vivid`，燈號字是單色） | 8 種燈號 + 「—」 | 7／177 |
    | DJ LEVEL（共用） | 80×34 | `Vivid` | A–F + 「—」（還沒有 AA、AAA） | 0／40 |
    | 難度和等級 | 150×19（下方曲名欄） | `Vivid` | 只有 N4、N5、N6 | 14／36 |
    | SP／DP | 72×43 | `Bright` | sp、dp | 9／1246 |
    | DEAD POINT 欄 | 136×14（標題字） | `Bright` | 有／沒有 | 0／381 |
    | USE OPTION | 125×19（選項文字） | `Bright` | A-SCR、LEGACY、沒有 | 0／533 |

    - 2P 位置在 1P +1350 附近 ±40 px 搜尋（USE OPTION 只有 +1324）。DEAD POINT 欄目前沒有 2P 樣本，2P 的位置未經驗證。
    - 沒看過的類別讀成「不明」（None），不會硬套。門檻是同類最大差和最近兩類距離的中間值；之後收集到新類別（HYPER、ANOTHER、AA…）時，要確認不會被誤認成相近的類別。
    - DEAD POINT 的 NOTE 不讀，直接用 PG+GR+GD+CB 算（4.2 的規則）。MEASURE 的數字樣本不夠（只有 0、1、2、5），還沒做。
    - 標註新增 `bestDjLevel`。一致性檢查新增：有 DEAD POINT 一定是 FAILED；ASSIST CLEAR 一定有 USE OPTION；最佳成績的燈號、DJ LEVEL、SCORE 要嘛全是「—」，要嘛全有值。
    - 22 張結算截圖的所有數字和文字欄位都讀對。標註畫面會用這些結果預填。
  - 還沒做：DEAD POINT 的 MEASURE、難度和等級的其他類別（要收集樣本）、引擎串接（結算紀錄寫進 DB）、選曲畫面、曲名。
- 每種組合（SP 1P／2P × 圖表在中間／在外側、DP 資訊欄左／右）都要各收一組樣本。互換樣本（SP 1P）**證實互換後判定面板會跟著 BPM 框移動**，寫死座標確實行不通。
- **會蓋住畫面的浮動元素**（樣本：曲子開頭）：
  - `HI-SPEED 操作方法` 說明框（蓋在車道上）
  - HI-SPEED 面板**跟著車道走**：出現在車道旁邊（靠畫面中間的那一側），蓋住那個位置上的東西。圖表在中間時蓋住 GRAPH INFORMATION 的柱狀圖，互換後就蓋住 BGA（使用者說明）。依 HI-SPEED 設定有三種，位置和外框（上下橘色條、右側灰色直排標題）相同，只有內容不同：

    | 設定 | 直排標題 | 刻度 | 左下 HI-SPEED 框 |
    | --- | --- | --- | --- |
    | FLOATING | （FLOATING HI-SPEED） | 10.0～0.5，另有 `CURRENT HI-SPEED 01.42` | 實際倍率 |
    | CLASSIC | `CLASSIC HI-SPEED CHANGE` | 4.00～1.00（11 格，選中的是金框） | 和選中的值相同（例 1.50） |
    | 一般 | `HI-SPEED CHANGE` | 11～1（選中的是金框） | **和選中的段數不同**（例：選 4，框顯示 1.47） |

    辨識不需要讀這個面板；要知道倍率時讀左下的 HI-SPEED 框就好（三種設定都會顯示實際倍率）。
  - 擋板資訊：SUDDEN+／LIFT 旁的 `SUB-LENGTH`、`NOTES TIME`（綠色數字）
  - 所以辨識 GRAPH INFORMATION 時，要用橘色標題列和「現在のスコア／自己ベスト」按鈕當錨點，**不能只靠柱狀圖**；判定面板和 BPM 框不會被這些蓋住。
  - RANDOM 配置辨識（F10）的偵測帶在判定線附近。開 LIFT 時判定線會往上移，偵測帶可能要跟著擋板位置調整，P9 要確認。
- **遊玩中也能驗算**：左下的 SCORE（例：562）= PG×2+GR（215×2+132 = 562 ✓）。打鍵計數讀到的判定可以每格都用這條規則檢查，讀錯的那一格直接丟掉。

**選曲畫面**（樣本：SP、PLAYER 01 SIDE，共 9 張：Shooting Star、FLOWER、She is my wife、RED ZONE、Re:GENERATION、Second Heaven、HAPPY☆ANGEL、Apocalypse ~dirge of swans~）

| 區塊 | 內容 | 用途 |
| --- | --- | --- |
| 左上 | `SP`／`DP` 字樣（PLAY STYLE） | 判斷 SP 或 DP |
| 中間 | 版本標誌、類型（DANCE POP）、大字曲名、演出者、BPM | 選曲時的曲名辨識（字型和結算的不同，要另外建表） |
| 難度列 | BEGINNER／NORMAL／HYPER／ANOTHER／LEGGENDARIA 的圖示和等級數字；目前選中的那個下方有 `PLAYER 01` 標記 | 判斷目前選的難度和等級 |
| 左下 SCORE DATA | SCORE、MISS COUNT、燈號文字、DJ LEVEL（大的點陣字）和「離下一級的差距」（例 `AA-0043`）、DJ LEVEL 條 | **F7 從選曲畫面登錄最佳成績的主要來源** |
| RIVAL SCORE DATA（中下） | 名次、玩家名、DJ LEVEL、分數、燈號；自己那一列是橘色底 | 有對手時才有。可以拿自己那一列交叉確認 SCORE DATA。不做對手功能，也不讀其他人的資料 |
| 右側曲目列表 | 等級、曲名、左邊的燈號色條；捲軸旁的 CLEAR VISUALIZATION 也會跟著目前曲子的燈號變色。依燈號排序時，會出現 `EASY CLEAR` 等分組標題 | 之後可以考慮一次讀整頁的燈號，第一版不做。列表裡的曲名是細字型、高度固定，**可能比中間的大字曲名好辨識**，P3b 兩種都試 |
| 其他 | CLEAR RATE、NOTES RADAR、DJ NAME | 辨識不用 |

SCORE DATA 的燈號文字，**8 種都有樣本了**（各自有固定顏色）：

| 燈號文字 | 樣本 | MISS COUNT |
| --- | --- | --- |
| `NO PLAY` | Apocalypse [SPB1] | `----` |
| `FAILED` | HAPPY☆ANGEL [SPN5]：374、DJ D | `----`（這次是關門，所以沒有 BP） |
| `ASSIST CLEAR` | Second Heaven [SPB1]：105、DJ C | `----`（輔助選項不記錄 BP） |
| `EASY CLEAR` | Re:GENERATION [SPN5]：466、DJ C | 55 |
| `CLEAR` | RED ZONE [SPB1]：103、DJ B | 1 |
| `HARD CLEAR` | She is my wife [SPB3]：213、DJ C | 6 |
| `EX HARD CLEAR` | FLOWER [SPB2]：216、DJ B | 4 |
| `FULLCOMBO CLEAR` | Shooting Star [SPN4]：611、DJ A | 0 |

- MISS COUNT 是 `----` 時，辨識結果記成「沒有資料」，不是 0。
- 長曲名的大字會用不同字型或縮小（例：`Apocalypse~dirge of swans~`），所以大字曲名不能假設字型和大小固定。

**結算畫面**

| 項目 | 觀察 |
| --- | --- |
| 1P／2P | 整個版面左右相反：1P 的成績欄在左、RIVAL 欄在右；2P 相反。PLAYER 01／02 字樣可以用來判斷是哪一邊 |
| SP／DP | 下方中間的曲名欄會顯示 `SP` 或 `DP`，旁邊是難度和等級（`NORMAL 4`）、NOTES 數（SP 420、DP 425） |
| 背景顏色 | 過關是藍色，FAILED 是紅色。錨點不能用背景，要用固定的框線或文字 |
| 第一次玩 | 最佳成績欄顯示「—」，辨識結果要能表示「沒有資料」 |
| 輔助選項 | `USE OPTION LEGACY` 等輔助選項時，燈號是 A-CLEAR，MISS COUNT 顯示「—」（不記錄），PACEMAKER 欄顯示「グラフ表示なし」 |
| NEW RECORD | 燈號和 MISS COUNT 刷新時，旁邊會有 NEW RECORD 標記，可以用來交叉確認「有沒有刷新」 |
| RIVAL 欄 | 會顯示其他玩家的名字，F5 的馬賽克／裁切就是處理這一塊 |
| 標題 | INFINITAS 的一般遊玩**一律是 `EXTRA STAGE`**，所以只有這一種標題。段位認定的畫面不同，目前不支援（見「不做」） |
| DEAD POINT（關門） | 中途停下來時才會出現的欄位（NOTE、MEASURE）。這時圓餅圖顯示 NO DATA，MISS COUNT 是「—」。打到最後才 FAILED 的話不會出現 |
| RADAR 分頁 | 右側面板切到 RADAR 時，會列出 NOTES RADAR 排行和曲名（字型和結算的曲名欄不同）。辨識不用它，F5 也不需要處理 |

各難度的結算畫面版型相同，只有難度文字和等級不同（使用者確認）。2P 和 DP 的結算判斷邏輯和 1P 相同（使用者確認），DEAD POINT 等欄位在 2P 就是左右相反的位置。各難度文字的樣本，平常遊玩時自然會收集到，不用特別去截。

**選項畫面**（GAME OPTION SETTING；選曲時打開。樣本：SP 的 OFF／A-EASY／EASY／HARD／EX-HARD 各一張，DP 一張，都是 PLAYER 01 SIDE）

| 區塊 | SP | DP |
| --- | --- | --- |
| 主面板範圍（約） | x 540–1425 | x 560–1845（比較寬） |
| 欄位 | 譜面の配置、グルーブゲージ減り幅、アシストプレー機能、譜面の表示エリア（4 欄） | 配置（左レーン）、配置（右レーン）、グルーブゲージ、アシスト、左右の譜面位置（OFF／FLIP）、表示エリア（6 欄）；兩個配置欄下面另有橫跨的 SYNCHRONIZE RANDOM、SYMMETRY RANDOM |
| 選項列的 Y（約） | 559／599／639／679／719（表示エリア還有 759） | 同左；SYNC／SYMM 約 755／787 |

- **選中的選項整格是橘色**，沒選的是黑底白字。判斷方式：每一欄逐列看那一格的底色是不是橘色。座標和參考專案的大致相同（例如 SP 的血條欄 X 約 935）。
- 主面板上方的說明文字在切換時會變（例：「HARDがONになりました。…」），但只是暫時的，辨識不用它。
- **EXTRA OPTION**（左下）：BIT BOOST、SUB TARGET、REGUL-SPEED、ALL-SCR、BATTLE、H-RAN、SPECIAL GAUGE OPTION。打開的是橘色字（樣本：SP 的 SUB TARGET、DP 的 BATTLE），沒打開的是白色，不能用的是灰色。BATTLE、H-RAN、ALL-SCR、REGUL-SPEED 要讀，其他不用。
- 左上的 TARGET SCORE GRAPH 清單（PACEMAKER A、グラフ表示なし…）不用讀。
- 讀到的配置選項（RANDOM 等）也一起記進「目前的選項」，給 `plays.option` 和 RANDOM 配置辨識（F10）用。

- 使用者確認：**選曲畫面在 SP／DP、1P／2P 都是同一個版面**，只有燈號（色條、SCORE DATA）分成 SP 和 DP 兩套；**2P 的選項畫面和 1P 相同**。

**SP 2P 圖表和 BGA 互換**：2026-11-09 已收到遊玩畫面樣本（判定面板位置見 P1 實作結果）。2P 的 DEAD POINT 先假設是 1P 左右相反，P3b 遇到實際樣本時再確認。

**血條種類（gauge）的判斷**

結算畫面不會直接寫出這次用的是哪種血條。過關時看燈號就知道，但 FAILED 和關門時需要另外判斷。

| 來源 | 能判斷的種類 | 說明 |
| --- | --- | --- |
| ① 選項畫面（**主要來源**，參考專案的做法） | OFF／A-EASY／EASY／HARD／EX-HARD 全部 | 參考專案在選曲時按 START 打開的選項畫面上，檢查每個血條選項固定位置的像素，R 值大於 100 的就是選中的那一個（SP 的 X 約 935、DP 約 1158，Y 是 559／599／639／679／719）。辨識到之後記成「目前的選項」，等結算時套用。我們照同樣的原理，從自己的截圖定座標 |
| ② 結算圖表的背景色 | HARD、EX-HARD | 樣本：EX-HARD 整片黃色、HARD 整片紅色、NORMAL 上紅下綠 |
| ③ 燈號 | 過關時全部 | E-CLEAR／CLEAR／H-CLEAR／EXH-CLEAR 本身就是血條種類 |

- **EASY 和 NORMAL 的圖表看起來一樣**，只能靠 ① 分辨。
- 決定順序：過關時以 ③ 為準。FAILED 和關門時用 ①，沒有 ① 就用 ②；兩個都沒有，就記成「不明（NORMAL 或 EASY）」。
- 參考專案的另一個用法：DBx 這類 Battle 選項的結算燈號一律顯示 A-CLEAR。這時 CB = 0 就改成 F-COMBO，否則用 ① 的血條種類換算成燈號。我們照做。
- 參考專案在辨識到選項之前，會**直接略過結算**不記錄。我們不略過，只把血條記成「不明」，避免遺失紀錄；啟動後還沒看過選項畫面時，主畫面提示「請在選曲畫面按一次 START 打開選項」。

**關門的處理方式**（使用者決定）

- 資料庫照常記錄，不丟掉：加上關門標記、DEAD POINT 的 NOTE 和 MEASURE，以及當下的所有數據。
- 今日紀錄、統計、收據這些 HTML 畫面和成績檢視器，**預設不顯示**關門的紀錄；設定裡可以打開。
- 打鍵數照算，因為鍵確實按下去了。
- 錄影檔照常改名，加上關門標記（見 5.2）。

### 4.6 授權

- 參考 Apache-2.0 的 inf_daken_counter_obsw 程式碼時，在 `NOTICE` 註明出處。HTML 畫面要改寫的話也一樣。
- inf-notebook（沒有標示授權）和 RandomLaneAnalysis_OCR（Other）只參考原理，不複製程式碼和資源。

## 5. 資料模型（初稿）

- `plays`：id、played_at、song_id、difficulty、play_mode、option、gauge（OFF/A-EASY/EASY/HARD/EXH/不明，以及判斷來源）、score、miss_count、lamp、dj_level、判定內訳、notes、is_dead、dead_note、dead_measure、lane_order、image_path
- `bests`：song_id + difficulty + play_mode 為主鍵；最佳分數、最佳 BP、最佳燈號、來源（`play` / `select` / `import`）
- `songs`：從曲目主表匯入（見 5.1）
- `learned_titles`：特徵 key（色系、字型種類、列 key 序列）→ song_id，記錄來源（`bundled` / `user`）
- `unknown_queue`：辨識不出來的特徵 key、截圖路徑、當時讀到的等級和 notes 數
- `random_charts`：正規譜面的事件序列（RANDOM 配置用）
- `daily_judge`：每次啟動或每天的判定累計（打鍵數）

「今日」是遊戲日：日本時間 05:00（每日維護開始）換日，見 P1 實作結果。

### 5.1 曲目主表格式

`resources/songs.json`，型別在 `iu-core::songs`，用本機工具 `tools/songlist` 產生（一首歌一行，方便看 diff）：

```json
{"source":"BEMANIWiki: …","generated":"2026-11-09","songs":[
{"id":"<title+artist 的 FNV-1a 64>","title":"Shooting Star","artist":"ReGLOSS","charts":{"SPN":{"level":4,"notes":420}, …}}
]}
```

- 譜面 key：`SPB/SPN/SPH/SPA/SPL/DPN/DPH/DPA/DPL`；`notes` 不明時省略。
- `id` 由曲名和演出者算出，文字不變 ID 就不變。
- 只取辨識和紀錄需要的欄位；類型、BPM、收錄區分、CN／BSS 等標記不取。單手難度（F12）到 P10 再找來源。
- 2026-11-09 產生結果：1897 首、11687 個譜面，其中 11681 個有 NOTES 數。只有 `Timepiece phase II (CN Ver.)` 不在 NOTES 表裡。`草原の王女-軌跡を辿って-` 在 NOTES 表寫成「奇跡」，使用者在遊戲選曲畫面確認是「軌跡」（等級 N3／H7／A10 也和 wiki 一致），工具用 `NOTES_TITLE_FIXES` 對照表修正。
- 處理過的格式：曲名欄附帶各譜面曲名（`crew[N]crew -original mix-…`）時只取前面的主曲名；`*1` 這類註記符號去掉；兩頁比對時忽略空白、大小寫、`…`／`...`、`～`／`~` 的差異；等級寫成 `4(5)` 時取括號外的值：只有 DEEP ROAR [SPN] 一筆，使用者在遊戲的選曲畫面確認是 4（2026-11-09），括號裡的 5 是其他版本的等級。
- 曲名和遊戲畫面上不一致的情況，P3b 標註時遇到再修正。
- **曲包的曲子使用者無法確認**（使用者沒有購買曲包，2026-11-09）。曲包的曲子在選曲畫面還是看得到曲名和等級（未購買也一樣），可以用來確認曲單；但沒辦法遊玩，所以收集不到結算畫面的樣本。曲包曲子的標註樣本也收集不到，曲名辨識要靠其他協作者的樣本，或使用者端的「邊玩邊學」（4.3）。

### 5.2 錄影檔名格式

預設格式：`{title} [{style}{diff}] {lamp} {djlevel}{dead}`，副檔名保留 OBS 原本的。

例：`Mermaid Girl [SPN] H-CLEAR B.mkv`、`Mermaid Girl [DPA] EXH-CLEAR AA.mkv`、關門時 `Shooting Star [SPN] FAILED F (DEAD 1).mkv`

| 變數 | 內容 |
| --- | --- |
| `{title}` | 曲名 |
| `{style}` | `SP` / `DP` |
| `{diff}` | 難度字母：`B` Beginner、`N` Normal、`H` Hyper、`A` Another、`L` Leggendaria |
| `{lamp}` | 這次遊玩的 Clear Type（不是最佳燈號） |
| `{djlevel}` | 這次遊玩的 DJ LEVEL（`AAA`～`F`） |
| `{dead}` | 關門時是 ` (DEAD <小節數>)`，沒關門時是空字串。預設格式的最後面會自動加上 |
| 其他可選 | `{level}`、`{score}`、`{bp}`、`{date}`、`{time}`、`{updated}` |

規則：

- Clear Type 的寫法：`FAILED`、`A-CLEAR`、`E-CLEAR`、`CLEAR`、`H-CLEAR`、`EXH-CLEAR`、`F-COMBO`。
- Windows 檔名不能用的字元（`\ / : * ? " < > |`）換成對應的全形字元；結尾的空白和句點要拿掉。
- 檔名已經存在的話，在後面加 ` (2)`、` (3)`…，不覆蓋舊檔。
- 關門也會進結算畫面，所以照一般規則命名，再加上 `{dead}`。
- 完全沒有進結算畫面（例如遊戲當掉、OBS 先停止）時，`{lamp}` 用 `FAILED`，DJ LEVEL 省略；曲名也辨識不出來的話，維持 OBS 原本的檔名。
- 改名失敗（例如檔案被鎖住）時，延遲幾秒重試，還是失敗就記到 log，不影響主程式。

## 6. 測試策略

- **樣本截圖回歸測試**：讀取 `dev-capture\`（見 6.2），依解析度和畫面種類分類，每張圖附上預期辨識結果（JSON）。辨識程式碼的改動都要通過這組測試。
- 預期值以人工標註為準。
- DB、錄影規則、RANDOM 推算寫單元測試。
- 擷取、疊加視窗、熱鍵、OBS 錄影只能實機驗證，每個階段結束時列出檢查清單。
- 截圖裡有遊戲的美術素材（著作權屬於 Konami）。**不要把截圖放進公開的 MIT repo**，只放在本機（見 6.2）；公開 repo 只放產生出來的查找表。

### 6.1 CI / 發佈（GitHub Actions）

- **每次 push 和 PR**：在 `windows-latest` 上跑 `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`、前端的型別檢查和 build。
- **截圖回歸測試只在本機跑**：CI 拿不到截圖，所以這組測試在找不到截圖資料夾時自動略過，並印出「已略過」，不可以算成通過。修改辨識程式碼的 PR，要在說明裡附上本機執行的結果。
- **發佈**：推送版本 tag 後，用 `tauri-action` 打包 NSIS 安裝檔、產生 updater 的簽章和 `latest.json`，建立 GitHub Release 草稿，人工確認後再公開。
- **Secrets**：`TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。exe 不做 Authenticode 簽章。

### 6.2 截圖存放位置

根目錄是 `<圖片資料夾>\INFINITAS Screenshots\`。圖片資料夾用 Windows 的 Known Folder（`FOLDERID_Pictures`）取得，使用者搬過圖片資料夾也能找到正確位置。

```
INFINITAS Screenshots\
├─ results\        # 結算圖片（F5 自動存、F6 手動存）
└─ dev-capture\    # 開發者擷取模式的原始畫面，給 annotator 用
   └─ <寬>x<高>\<畫面種類>\
```

- 根目錄可以在設定裡改，預設就是上面這個位置。
- 回歸測試從 `dev-capture\` 和其中的標註檔讀取資料；位置可以用環境變數 `IU_FIXTURES_DIR` 覆寫。

## 7. 已決定事項

- App 名稱 `infinitas-utilities`，repo `naruko-studio/infinitas-utilities`。
- 介面：繁中／日／英。
- 前端：Svelte 5 + TS；OBS 畫面用純 JS。
- 曲目主表：從 BEMANIWiki 的全曲リスト和総ノーツ数リスト用 `tools/songlist` 產生 `resources/songs.json`，只取事實資料（曲名、演出者、各譜面等級和 NOTES 數），跟著 repo 發佈（2026-11-09 使用者決定）。
- 結算圖片：預設刷新紀錄時才存（和參考專案相同）。
- OBS：只做錄影控制（場景切換＋開始／停止錄影）。預設每首歌一個檔，改檔名的預設格式是 `{title} [{style}{diff}] {lamp} {djlevel}`，例如 `Mermaid Girl [SPN] H-CLEAR B`（見 5.2）。
- 不匯入參考專案 v3 的資料。
- 抓畫面只用 Spout2；解析度自動判斷（每種解析度各一套資源組）。
- 授權：MIT。
- exe 不做 Authenticode 簽章（之後有贊助或低價方案再加）。
- 本機資料夾先維持 `infinitas-stream-tools`，使用者之後自己改名。
- 打包和發佈走 GitHub Actions（見 6.1）。
- 截圖一律放在 `<使用者的圖片資料夾>\INFINITAS Screenshots\` 底下（見 6.2）。
- 主要的解析度是 1920×1080，第一版的資源組就做這個。
- 要做 `today_receipt.html`（拿掉 BPI 區塊）。
- 辨識：參考演算法自己用 Rust 實作，資源自己產生。
- commit 要有 GPG 簽章。目前 Windows 上的 GPG 有問題，**修好之前不 commit**。

## 8. 待決定事項

目前沒有。實作途中遇到需要決定的事再補上。

（已解決：選曲畫面的 SCORE DATA 之前顯示 `NO PLAY`，是第一次玩之前截的。新樣本顯示 611 / 0 / FULLCOMBO CLEAR / A，和結算一致。）
