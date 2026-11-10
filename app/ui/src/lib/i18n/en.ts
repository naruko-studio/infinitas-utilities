// English is the reference dictionary: its keys define `MessageKey`, and
// the other languages must provide every key (checked by `pnpm check`).
export const en = {
  "app.title": "infinitas-utilities",
  "app.version": "Version {version}",
  "settings.language": "Language",
  "settings.language.auto": "Follow system",
  "settings.devCapture": "Developer capture mode",
  "settings.devCapture.hint":
    "Saves frames to Pictures\\INFINITAS Screenshots\\dev-capture when the judge panel cannot be read, and every 10 s during play.",
  "error.configSave": "Could not save the settings: {error}",

  "connection.starting": "Starting…",
  "connection.waiting":
    "Waiting for Spout sender “{sender}”. Is OBS running with the Spout Filter on the game capture?",
  "connection.connected": "Receiving {width}×{height} from “{sender}”",
  "connection.unsupportedResolution":
    "{width}×{height} is not supported yet (only 1920×1080 for now).",
  "connection.error": "Capture error: {detail}",

  "play.onScreen": "Playing ({layout})",
  "play.notOnScreen": "Not on the play screen",
  "play.finished": "Finished ({layout})",
  "play.hint":
    "If the play screen is not recognised, set “判定の数リアルタイム表示” (live judge count) to ON in the game's ADVANCED GAME OPTION SETTING.",
  "today.title": "Today",
  "today.notes": "Keystrokes",
  "today.plays": "Plays",
  "current.title": "Current play",
  "unnamed.title": "Which song was this?",
  "unnamed.hint":
    "The title of these results was not recognised. Pick the song once and it will be recognised from now on.",
  "unnamed.search": "Search the song list",
  "unnamed.noCandidates": "No song in the list has this chart and note count. Search by title:",
  "unnamed.failed": "Could not save: {error}",
  "ocr.missing":
    "Song titles cannot be read: Windows has no Japanese text recognition. Add Japanese in Settings → Time & language → Language & region (no need to make it the display language), then restart the app.",
  "capture.button": "Save current frame",
  "capture.saved": "Saved: {path}",
  "capture.failed": "Could not save: {error}",

  "web.title": "OBS and phone",
  "web.obs": "OBS browser sources (this PC):",
  "web.copy": "Copy",
  "web.size": "Recommended size {size}",
  "web.copied": "Copied: {url}",
  "web.lan": "Allow phones on the LAN to connect",
  "web.lan.hint":
    "Open this address on a phone on the same network. Windows may ask to allow the app through the firewall. The pages are read-only and have no password: only turn this on at home, not on public Wi-Fi.",
  "web.noLanIp": "Could not find this PC's LAN address.",
  "web.error": "The web server could not start on port {port}: {error}",

  "tab.main": "Main",
  "tab.annotator": "Annotator",
  "annotator.filter.todo": "To label",
  "annotator.filter.all": "All",
  "annotator.filter.unsorted": "Unsorted (dev capture)",
  "annotator.filter.game": "Game F12 folder",
  "annotator.filter.play": "play",
  "annotator.filter.result": "result",
  "annotator.filter.select": "select",
  "annotator.filter.option": "option",
  "annotator.filter.other": "other",
  "annotator.pick": "Pick a screenshot on the left.",
  "annotator.suggested": "Pre-filled from recognition ({screen}). Check every value.",
  "annotator.song": "Song",
  "annotator.unknownSong": "Not in the song list: saved with the title only.",
  "annotator.best": "Best (left column)",
  "annotator.consistent": "Numbers are consistent.",
  "annotator.save": "Save (Ctrl+S)",
  "annotator.saved": "Saved.",
  "annotator.discard": "Discard",
} as const;

export type MessageKey = keyof typeof en;
export type Dictionary = Record<MessageKey, string>;
