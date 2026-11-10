// Typed wrappers around the Tauri commands in `src-tauri/src/commands.rs`.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LanguageSetting } from "./i18n/index.svelte";

/** Subset of `iu_core::Config` the UI uses so far. */
export interface Config {
  language: LanguageSetting;
  capture: { spout_sender: string; fps: number; dev_capture: boolean };
}

export interface Judge {
  pg: number;
  gr: number;
  gd: number;
  bd: number;
  pr: number;
  cb: number;
}

export type Connection =
  | "starting"
  | "waiting"
  | "connected"
  | "unsupportedResolution"
  | "error";

/** Mirrors `iu_core::status::Status`. */
export interface Status {
  connection: Connection;
  detail: string | null;
  sender: string;
  resolution: [number, number] | null;
  layout: string | null;
  current: Judge | null;
  lastPlay: { layout: string; judge: Judge } | null;
  lastResult: ResultRecord | null;
  currentSong: { id: string; title: string; artist: string } | null;
  ocrAvailable: boolean | null;
  today: Judge;
  todayNotes: number;
  playsToday: number;
  devCapture: boolean;
}

export const getConfig = () => invoke<Config>("get_config");

export const setLanguage = (language: LanguageSetting) =>
  invoke<void>("set_language", { language });

export const getStatus = () => invoke<Status>("get_status");

export const setDevCapture = (enabled: boolean) =>
  invoke<void>("set_dev_capture", { enabled });

export const captureFrame = () => invoke<string>("capture_frame");

/** Mirrors `web::ServerInfo`. */
export interface ServerInfo {
  running: boolean;
  port: number;
  lanAccess: boolean;
  error: string | null;
  localUrl: string;
  lanUrl: string | null;
}

export const getServerInfo = () => invoke<ServerInfo>("get_server_info");

export const setLanAccess = (enabled: boolean) =>
  invoke<ServerInfo>("set_lan_access", { enabled });

export const onStatus = (handler: (s: Status) => void): Promise<UnlistenFn> =>
  listen<Status>("engine://status", (e) => handler(e.payload));

// ---- annotator (developer screen) ------------------------------------------

export type Screen = "play" | "result" | "select" | "option" | "other";
export type Chart = "SPB" | "SPN" | "SPH" | "SPA" | "SPL" | "DPN" | "DPH" | "DPA" | "DPL";
export type Lamp =
  | "no-play"
  | "failed"
  | "assist-clear"
  | "easy-clear"
  | "clear"
  | "hard-clear"
  | "ex-hard-clear"
  | "full-combo";
export type DjLevel = "F" | "E" | "D" | "C" | "B" | "A" | "AA" | "AAA";
export type Gauge = "off" | "assist-easy" | "easy" | "hard" | "ex-hard";

export interface PlayLabel {
  screen: "play";
  layout: string;
  judge: Judge;
}

export interface ResultLabel {
  screen: "result";
  side: "1p" | "2p";
  chart: Chart;
  level: number;
  songId: string | null;
  title: string;
  notes: number;
  lamp: Lamp;
  bestLamp: Lamp | null;
  djLevel: DjLevel;
  bestDjLevel: DjLevel | null;
  score: number;
  bestScore: number | null;
  missCount: number | null;
  bestMissCount: number | null;
  judge: { pg: number; gr: number; gd: number; bd: number; pr: number };
  comboBreak: number;
  fast: number;
  slow: number;
  dead: { note: number; measure: number } | null;
  useOption: string | null;
}

export interface SelectLabel {
  screen: "select";
  style: "sp" | "dp";
  chart: Chart;
  level: number;
  songId: string | null;
  title: string;
  lamp: Lamp;
  djLevel: DjLevel | null;
  score: number;
  missCount: number | null;
}

export interface OptionLabel {
  screen: "option";
  style: "sp" | "dp";
  gauge: Gauge;
}

export type Label = PlayLabel | ResultLabel | SelectLabel | OptionLabel | { screen: "other" };

export interface Shot {
  path: string;
  name: string;
  source: "devCapture" | "game";
  folder: string | null;
  width: number;
  height: number;
  labeled: boolean;
  modified: number;
}

export interface Loaded {
  label: Label | null;
  suggestion: Label | null;
}

export interface Song {
  id: string;
  title: string;
  artist: string;
  charts: Partial<Record<Chart, { level: number; notes?: number }>>;
}

export const annotatorList = () => invoke<Shot[]>("annotator_list");
export const annotatorImage = (path: string) =>
  invoke<ArrayBuffer>("annotator_image", { path });
export const annotatorLoad = (path: string) => invoke<Loaded>("annotator_load", { path });
export const annotatorCheck = (label: Label) => invoke<string[]>("annotator_check", { label });
export const annotatorSave = (path: string, label: Label) =>
  invoke<string>("annotator_save", { path, label });
export const annotatorDiscard = (path: string) => invoke<void>("annotator_discard", { path });
export const songList = () => invoke<{ songs: Song[] }>("song_list");

/** Mirrors `iu_core::result::ResultRecord`. */
export interface ResultRecord {
  id: number | null;
  title: string | null;
  gameDay: string;
  recordedAt: string;
  side: "1p" | "2p";
  chart: Chart | null;
  level: number | null;
  songId: string | null;
  notes: number;
  lamp: Lamp | null;
  bestLamp: Lamp | null;
  djLevel: DjLevel | null;
  bestDjLevel: DjLevel | null;
  score: number;
  bestScore: number | null;
  missCount: number | null;
  bestMissCount: number | null;
  judge: { pg: number; gr: number; gd: number; bd: number; pr: number };
  comboBreak: number;
  fast: number;
  slow: number;
  dead: boolean;
  useOption: string | null;
}

export interface Candidate {
  id: string;
  title: string;
  artist: string;
}

/** A result whose song is not known yet. */
export interface UnnamedResult {
  result: ResultRecord;
  candidates: Candidate[];
}

export const unnamedResults = () => invoke<UnnamedResult[]>("unnamed_results");
export const nameResult = (id: number, songId: string) =>
  invoke<void>("name_result", { id, songId });

/** A result was recorded, or a stored one changed (its song was named). */
export const onResultChange = async (handler: () => void): Promise<UnlistenFn> => {
  const a = await listen("engine://result", handler);
  const b = await listen("engine://result-updated", handler);
  return () => {
    a();
    b();
  };
};

/** The game's own lamp names. */
export const LAMP_NAMES: Record<Lamp, string> = {
  "no-play": "NO PLAY",
  failed: "FAILED",
  "assist-clear": "ASSIST CLEAR",
  "easy-clear": "EASY CLEAR",
  clear: "CLEAR",
  "hard-clear": "HARD CLEAR",
  "ex-hard-clear": "EX HARD CLEAR",
  "full-combo": "FULL COMBO",
};
