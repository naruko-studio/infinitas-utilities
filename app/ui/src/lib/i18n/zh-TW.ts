import type { Dictionary } from "./en";

export const zhTW: Dictionary = {
  "app.title": "infinitas-utilities",
  "app.version": "版本 {version}",
  "settings.language": "語言",
  "settings.language.auto": "跟隨系統",
  "settings.devCapture": "開發者擷取模式",
  "settings.devCapture.hint":
    "讀不到判定面板時，以及遊玩中每 10 秒，把畫面存到「圖片\\INFINITAS Screenshots\\dev-capture」。",
  "error.configSave": "無法儲存設定：{error}",

  "connection.starting": "啟動中…",
  "connection.waiting":
    "等待 Spout 送出端「{sender}」。請確認 OBS 有開，而且遊戲擷取來源加了 Spout Filter。",
  "connection.connected": "正在接收「{sender}」的 {width}×{height} 畫面",
  "connection.unsupportedResolution": "目前還不支援 {width}×{height}（現在只支援 1920×1080）。",
  "connection.error": "擷取錯誤：{detail}",

  "play.onScreen": "遊玩中（{layout}）",
  "play.notOnScreen": "不在遊玩畫面",
  "play.finished": "已結束（{layout}）",
  "play.hint":
    "如果認不出遊玩畫面，請在遊戲的 ADVANCED GAME OPTION SETTING 把「判定の数リアルタイム表示」設為 ON。",
  "today.title": "今日",
  "today.notes": "打鍵數",
  "today.plays": "遊玩次數",
  "current.title": "這首歌",
  "capture.button": "儲存目前畫面",
  "capture.saved": "已儲存：{path}",
  "capture.failed": "無法儲存：{error}",
};
