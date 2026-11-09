<div align="center">

# infinitas-utilities

[English](README.md) | [繁體中文](README.zh-TW.md) | **日本語**

beatmania IIDX INFINITAS の配信・プレー補助ツール（Windows x64）。

[![CI](https://github.com/naruko-studio/infinitas-utilities/actions/workflows/ci.yml/badge.svg?branch=development)](https://github.com/naruko-studio/infinitas-utilities/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/naruko-studio/infinitas-utilities?include_prereleases&sort=semver)](https://github.com/naruko-studio/infinitas-utilities/releases)
[![Downloads](https://img.shields.io/github/downloads/naruko-studio/infinitas-utilities/total)](https://github.com/naruko-studio/infinitas-utilities/releases)
[![License: MIT](https://img.shields.io/github/license/naruko-studio/infinitas-utilities)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D6?logo=windows)](#)

[![Rust](https://img.shields.io/badge/Rust-2024-000000?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte)](https://svelte.dev/)
[![AI-assisted](https://img.shields.io/badge/AI--assisted-Claude%20Code-D97757?logo=claude)](#ai-支援について)

[![Stars](https://img.shields.io/github/stars/naruko-studio/infinitas-utilities?style=flat)](https://github.com/naruko-studio/infinitas-utilities/stargazers)
[![Issues](https://img.shields.io/github/issues/naruko-studio/infinitas-utilities)](https://github.com/naruko-studio/infinitas-utilities/issues)
[![Contributors](https://img.shields.io/github/contributors/naruko-studio/infinitas-utilities)](https://github.com/naruko-studio/infinitas-utilities/graphs/contributors)
[![Last commit](https://img.shields.io/github/last-commit/naruko-studio/infinitas-utilities/development)](https://github.com/naruko-studio/infinitas-utilities/commits/development)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](https://github.com/naruko-studio/infinitas-utilities/pulls)

</div>

> [!NOTE]
> 開発中（P2：OBS・スマートフォン用ページ）。まだ実際には使えません。機能範囲と進捗は [docs/PLAN.md](docs/PLAN.md)（繁体字中国語）を参照してください。

## 予定している機能

- 打鍵カウンター：今日の累計（1P／2P／DP 対応）。日付の切り替えは日本時間 05:00（毎日のメンテナンス開始時刻）
- リザルト・選曲画面の認識、スコアデータベース、スコアビューアー
- リザルト画像の自動保存（初期設定では自己ベスト更新時のみ）
- OBS ブラウザソース用の HTML 画面（今日の記録、統計、レシート…）。LAN 内のスマートフォンからも閲覧可能
- OBS 録画制御：1 曲ごとに 1 ファイル、終了後に曲名・難易度・クリアランプでファイル名を変更
- ゲーム画面オーバーレイ、RANDOM 配置の認識
- UI 言語：English、繁體中文、日本語

## 仕組み

情報はすべて**画面キャプチャと画像処理**で取得します。ゲームのメモリやファイルの読み取り・改変、リバースエンジニアリングは一切行いません。

画面は OBS の Spout2 フィルター経由で受け取ります：

1. OBS に Spout2 プラグインをインストールする。
2. `bm2dx.exe` を取り込むゲームキャプチャソースに **Spout Filter** を追加し、フィルターの一番上に置く（またはこのフィルターだけにする）。初期設定の送信名は `Spout_OBS_Filter`。
3. ゲームの ADVANCED GAME OPTION SETTING で「判定の数リアルタイム表示」を ON にする。

現在は 1920×1080 のみ対応しています。ほかの解像度はコントリビューターからの PR を歓迎します（PLAN 4.1.1 参照）。

### OBS とスマートフォン

アプリは `http://localhost:8767` でページを提供します（メイン画面に URL とコピーボタンがあります）：

| ページ | 内容 |
| --- | --- |
| `/obs/counter.html` | 今日の打鍵数とプレー回数（OBS ブラウザソース） |
| `/obs/judge.html` | この曲の判定数と割合バー。`?scope=today` で今日の合計 |
| `/` | スマートフォン用ページ：今日の合計、この曲、今日の記録、最近の記録 |

OBS 用ページは背景が透明で、フォント・色・サイズはブラウザソースの「カスタム CSS」で CSS 変数（`--iu-size`、`--iu-color`…）を使って変更できます。スマートフォンで見るには、メイン画面で「LAN 内のスマートフォンからの接続を許可」をオンにし、表示されたアドレスを開いてください。ページは閲覧のみでパスワードはないため、自宅のネットワークでのみ使用してください。

## 開発

Rust（stable、MSVC）、Node.js LTS、pnpm が必要です。

```powershell
cd app
pnpm install
pnpm tauri dev                 # アプリを起動

# リポジトリのルートで
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### データの保存場所

| ファイル | 場所 |
| --- | --- |
| 設定 `config.json`、データベース `data.db` | `%APPDATA%\studio.naruko.infinitas-utilities\` |
| ログ | `%LOCALAPPDATA%\studio.naruko.infinitas-utilities\logs\` |

`data.db` は SQLite（WAL モード）です。閲覧には [HeidiSQL](https://www.heidisql.com/)、[DataGrip](https://www.jetbrains.com/datagrip/)（非商用は無料）、[Beekeeper Studio Community Edition](https://www.beekeeperstudio.io/)、[DbGate](https://dbgate.org/) が使えます。アプリ実行中に開いても構いませんが、データは変更しないでください。

### スクリーンショット回帰テスト

ゲームのスクリーンショットの著作権は Konami にあるため、**リポジトリには含めていません**。テストは
`<ピクチャ>\INFINITAS Screenshots\dev-capture\<幅>x<高さ>\<画面>\` からスクリーンショットを読み込み、
フォルダーがない場合はスキップします（CI はこの状態です）。

- `IU_FIXTURES_DIR`：別のフォルダーを使う。
- `IU_REQUIRE_FIXTURES=1`：見つからない場合にスキップせず失敗にする（ローカルでうっかりスキップしないように）。

## 謝辞

本プロジェクトは以下のプロジェクトを参考にしています。感謝いたします。

- [dj-kata/inf_daken_counter_obsw](https://github.com/dj-kata/inf_daken_counter_obsw)（Apache-2.0）：機能範囲と全体設計の主な参考。
- [kaktuswald/inf-notebook](https://github.com/kaktuswald/inf-notebook)：リザルト・選曲画面の認識手法を参考。**アルゴリズムのみ**を参考にしており、コードや認識用リソースの使用・変換・複製はしていません。
- [yuki-RAI01/RandomLaneAnalysis_OCR](https://github.com/yuki-RAI01/RandomLaneAnalysis_OCR)：RANDOM 配置の認識手法を参考。**アルゴリズムのみ**を参考にしており、コードやリソースは使用していません。
- [Spout2](https://github.com/leadedge/Spout2)（BSD-2-Clause）：画面キャプチャ。
- [Tauri](https://tauri.app/)、[Svelte](https://svelte.dev/)。

認識用のマスクと参照テーブルは、すべて本プロジェクトの `tools/resgen` で自前のラベル付きスクリーンショットから生成しています。

## AI 支援について

本プロジェクトは AI の支援を受けて開発しています。コードとドキュメントの多くは [Claude Code](https://claude.com/claude-code)（Anthropic）が作成し、メンテナーが方針の決定、変更のレビュー、実機での検証を行っています。AI が関わったコミットには `Co-Authored-By: Claude` を付けています。

## 免責事項

本プロジェクトは KONAMI とは無関係であり、KONAMI の承認を受けたものではありません。beatmania IIDX および INFINITAS は KONAMI の商標です。

## ライセンス

[MIT](LICENSE)。サードパーティのライセンスと出典は [NOTICE](NOTICE) を参照してください。
