import en from "./en";
import ja from "./ja";
import type { Translations } from "./en";

export type Locale = "en" | "ja";
export type { Translations };

export const translations: Record<Locale, Translations> = { en, ja };

export function detectLocale(): Locale {
  if (typeof navigator === "undefined") {
    return "en";
  }
  const lang = navigator.language?.slice(0, 2);
  return lang === "ja" ? "ja" : "en";
}
