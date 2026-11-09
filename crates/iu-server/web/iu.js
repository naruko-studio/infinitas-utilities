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
    const open = () => {
      const ws = new WebSocket(`ws://${location.host}/ws`);
      ws.onopen = () => {
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
      offline: "切断されました。再接続中…",
    },
  };

  function strings() {
    const lang = (navigator.language || "en").toLowerCase();
    if (lang.startsWith("ja")) return STRINGS.ja;
    if (lang.startsWith("zh")) return STRINGS["zh-TW"];
    return STRINGS.en;
  }

  return { JUDGES, connect, getJSON, notes, shownJudge, fmt, strings };
})();
