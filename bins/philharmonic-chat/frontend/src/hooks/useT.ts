import { translations, type Translations } from "../i18n";
import { useAppSelector } from "../store";

export function useT(): Translations {
  const locale = useAppSelector((state) => state.i18n.locale);
  return translations[locale];
}
