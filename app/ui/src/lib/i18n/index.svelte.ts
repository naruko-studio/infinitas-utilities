import { en, type Dictionary, type MessageKey } from "./en";
import { ja } from "./ja";
import { zhTW } from "./zh-TW";

/** Matches `iu_core::Language` (serde names). */
export type LanguageSetting = "auto" | "zh-TW" | "ja" | "en";
export type Locale = Exclude<LanguageSetting, "auto">;

export const LOCALES: readonly Locale[] = ["zh-TW", "ja", "en"];
export const LOCALE_NAMES: Record<Locale, string> = {
  "zh-TW": "繁體中文",
  ja: "日本語",
  en: "English",
};

const dictionaries: Record<Locale, Dictionary> = { "zh-TW": zhTW, ja, en };

/** Picks a supported locale from the browser/OS language list. */
export function detectLocale(languages: readonly string[] = navigator.languages): Locale {
  for (const tag of languages) {
    const lower = tag.toLowerCase();
    if (lower.startsWith("zh")) return "zh-TW";
    if (lower.startsWith("ja")) return "ja";
    if (lower.startsWith("en")) return "en";
  }
  return "en";
}

let locale = $state<Locale>(detectLocale());

export function getLocale(): Locale {
  return locale;
}

export function applyLanguage(setting: LanguageSetting): void {
  locale = setting === "auto" ? detectLocale() : setting;
  document.documentElement.lang = locale;
}

/** Looks up `key` and fills `{name}` placeholders from `params`. */
export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  const template = dictionaries[locale][key] ?? en[key];
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}
