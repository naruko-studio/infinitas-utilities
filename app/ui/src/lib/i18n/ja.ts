import type { Dictionary } from "./en";

export const ja: Dictionary = {
  "app.title": "infinitas-utilities",
  "app.version": "バージョン {version}",
  "settings.language": "言語",
  "settings.language.auto": "システムに合わせる",
  "settings.devCapture": "開発者キャプチャモード",
  "settings.devCapture.hint":
    "判定表示を読めなかったときと、プレー中 10 秒ごとに、画面を「ピクチャ\\INFINITAS Screenshots\\dev-capture」に保存します。",
  "error.configSave": "設定を保存できませんでした：{error}",

  "connection.starting": "起動中…",
  "connection.waiting":
    "Spout 送信元「{sender}」を待っています。OBS が起動していて、ゲームキャプチャに Spout Filter が付いているか確認してください。",
  "connection.connected": "「{sender}」から {width}×{height} を受信中",
  "connection.unsupportedResolution":
    "{width}×{height} にはまだ対応していません（現在は 1920×1080 のみ）。",
  "connection.error": "キャプチャエラー：{detail}",

  "play.onScreen": "プレー中（{layout}）",
  "play.notOnScreen": "プレー画面ではありません",
  "play.finished": "プレー終了（{layout}）",
  "play.hint":
    "プレー画面を認識しない場合は、ADVANCED GAME OPTION SETTING の「判定の数リアルタイム表示」を ON にしてください。",
  "today.title": "今日",
  "today.notes": "打鍵数",
  "today.plays": "プレー回数",
  "current.title": "この曲",
  "capture.button": "現在の画面を保存",
  "capture.saved": "保存しました：{path}",
  "capture.failed": "保存できませんでした：{error}",

  "web.title": "OBS とスマートフォン",
  "web.obs": "OBS ブラウザソース（この PC）：",
  "web.copy": "コピー",
  "web.copied": "コピーしました：{url}",
  "web.lan": "LAN 内のスマートフォンからの接続を許可",
  "web.lan.hint":
    "同じネットワークのスマートフォンでこのアドレスを開いてください。Windows がファイアウォールの許可を求める場合があります。ページは閲覧のみでパスワードはありません。自宅のネットワークでのみオンにし、公衆 Wi-Fi ではオンにしないでください。",
  "web.noLanIp": "この PC の LAN アドレスが見つかりませんでした。",
  "web.error": "ポート {port} で Web サーバーを起動できませんでした：{error}",
};
