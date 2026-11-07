// Typed wrappers around the Tauri commands in `src-tauri/src/commands.rs`.
import { invoke } from "@tauri-apps/api/core";
import type { LanguageSetting } from "./i18n/index.svelte";

/** Subset of `iu_core::Config` the UI uses so far. */
export interface Config {
  language: LanguageSetting;
}

export const getConfig = () => invoke<Config>("get_config");

export const setLanguage = (language: LanguageSetting) =>
  invoke<void>("set_language", { language });
