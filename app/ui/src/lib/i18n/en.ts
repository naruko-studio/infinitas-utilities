// English is the reference dictionary: its keys define `MessageKey`, and
// the other languages must provide every key (checked by `pnpm check`).
export const en = {
  "app.title": "infinitas-utilities",
  "app.version": "Version {version}",
  "settings.language": "Language",
  "settings.language.auto": "Follow system",
  "status.notImplemented": "Nothing to show yet. Features arrive from P1 on.",
  "error.configSave": "Could not save the settings: {error}",
} as const;

export type MessageKey = keyof typeof en;
export type Dictionary = Record<MessageKey, string>;
