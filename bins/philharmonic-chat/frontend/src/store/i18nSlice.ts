import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

import { detectLocale, type Locale } from "../i18n";

const LOCALE_KEY = "philharmonic.chat.locale";

function storedLocale(): Locale {
  try {
    const stored = window.localStorage.getItem(LOCALE_KEY);
    if (stored === "en" || stored === "ja") {
      return stored;
    }
  } catch {
    // Browser storage can be unavailable in private or restricted contexts.
  }
  return detectLocale();
}

interface I18nState {
  locale: Locale;
}

const i18nSlice = createSlice({
  name: "i18n",
  initialState: { locale: storedLocale() } as I18nState,
  reducers: {
    setLocale(state, action: PayloadAction<Locale>) {
      state.locale = action.payload;
      try {
        window.localStorage.setItem(LOCALE_KEY, action.payload);
      } catch {
        // Redux state still reflects the user's selection when storage fails.
      }
    },
  },
});

export const { setLocale } = i18nSlice.actions;
export default i18nSlice.reducer;
