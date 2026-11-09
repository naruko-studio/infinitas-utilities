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

/** Mirrors `engine::Status`. */
export interface Status {
  connection: Connection;
  detail: string | null;
  sender: string;
  resolution: [number, number] | null;
  layout: string | null;
  current: Judge | null;
  lastPlay: { layout: string; judge: Judge } | null;
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

export const onStatus = (handler: (s: Status) => void): Promise<UnlistenFn> =>
  listen<Status>("engine://status", (e) => handler(e.payload));
