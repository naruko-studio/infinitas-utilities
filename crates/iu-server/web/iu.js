// Shared client for the OBS pages and the phone page.
//
//   IU.connect({ status(s) {...}, playFinished(p) {...} })
//
// Opens the WebSocket feed of this server and reconnects forever (OBS keeps
// browser sources open while the app restarts). The first message after
// each (re)connect is a full status snapshot.
"use strict";

const IU = (() => {
  const JUDGES = ["pg", "gr", "gd", "bd", "pr", "cb"];

  function connect(handlers) {
    let delay = 500;
    let connectedBefore = false;
    const open = () => {
      const ws = new WebSocket(`ws://${location.host}/ws`);
      ws.onopen = () => {
        // Back after a disconnect: the app may have restarted with newer
        // pages, so load them rather than keep running the old ones.
        if (connectedBefore) {
          location.reload();
          return;
        }
        connectedBefore = true;
        delay = 500;
        handlers.open?.();
      };
      ws.onmessage = (msg) => {
        let ev;
        try {
          ev = JSON.parse(msg.data);
        } catch {
          return;
        }
        handlers[ev.type]?.(ev);
      };
      ws.onclose = () => {
        handlers.close?.();
        setTimeout(open, delay);
        delay = Math.min(delay * 2, 5000);
      };
    };
    open();
  }

  async function getJSON(path) {
    const res = await fetch(path, { cache: "no-store" });
    if (!res.ok) throw new Error(`${path}: ${res.status}`);
    return res.json();
  }

  /** Keystrokes that hit a note: PG + GR + GD + BD. */
  const notes = (j) => j.pg + j.gr + j.gd + j.bd;

  /** The judge counts to show: the play in progress, else the last one. */
  const shownJudge = (s) => s.current ?? s.lastPlay?.judge ?? null;

  const fmt = (n) => n.toLocaleString();

  // Minimal UI strings for the phone page; the app UI has the full set.
  const STRINGS = {
    en: {
      today: "Today",
      keystrokes: "keystrokes",
      plays: "plays",
      current: "Current play",
      finished: "Finished",
      playing: "Playing",
      none: "No play yet",
      todayPlays: "Today's plays",
      recentDays: "Recent days",
      todayResults: "Today's results",
      naming: "Which song was this?",
      namingHint: "The title of these results was not recognised. Pick the song once and it will be recognised from now on.",
      noCandidates: "No song in the list has this chart and note count. Search by title:",
      search: "Search the song list",
      offline: "Disconnected — reconnecting…",
    },
    "zh-TW": {
      today: "今日",
      keystrokes: "打鍵數",
      plays: "遊玩次數",
      current: "這首歌",
      finished: "已結束",
      playing: "遊玩中",
      none: "還沒有遊玩紀錄",
      todayPlays: "今天的紀錄",
      recentDays: "最近幾天",
      todayResults: "今天的結算",
      naming: "這是哪首歌？",
      namingHint: "這些結算認不出曲名。選一次之後，下次就會自動認出來。",
      noCandidates: "清單裡沒有這個譜面和 NOTES 數的歌，請用曲名搜尋：",
      search: "搜尋曲目清單",
      offline: "連線中斷，重新連線中…",
    },
    ja: {
      today: "今日",
      keystrokes: "打鍵数",
      plays: "プレー回数",
      current: "この曲",
      finished: "プレー終了",
      playing: "プレー中",
      none: "まだプレー記録がありません",
      todayPlays: "今日の記録",
      recentDays: "最近の記録",
      todayResults: "今日のリザルト",
      naming: "この曲はどれ？",
      namingHint: "これらのリザルトは曲名を認識できませんでした。一度選ぶと、次からは自動で認識します。",
      noCandidates: "この譜面とノーツ数の曲がリストにありません。曲名で検索：",
      search: "曲リストを検索",
      offline: "切断されました。再接続中…",
    },
  };

  const LANGS = { auto: "Auto / 自動", en: "English", "zh-TW": "繁體中文", ja: "日本語" };
  const LANG_KEY = "iu.lang";

  /** Chosen language: `?lang=` (OBS pages), else the phone page's saved
   *  choice, else "auto". */
  function lang() {
    const q = new URLSearchParams(location.search).get("lang");
    if (q && q in LANGS) return q;
    try {
      const saved = localStorage.getItem(LANG_KEY);
      if (saved && saved in LANGS) return saved;
    } catch {}
    return "auto";
  }

  function setLang(value) {
    try {
      localStorage.setItem(LANG_KEY, value);
    } catch {}
  }

  function strings() {
    let l = lang();
    if (l === "auto") {
      const nav = (navigator.language || "en").toLowerCase();
      l = nav.startsWith("ja") ? "ja" : nav.startsWith("zh") ? "zh-TW" : "en";
    }
    return STRINGS[l];
  }

  return { JUDGES, LANGS, connect, getJSON, notes, shownJudge, fmt, lang, setLang, strings };
})();
