import { configureStore } from "@reduxjs/toolkit";
import { useDispatch, useSelector } from "react-redux";

import authReducer, { persistAuth } from "./authSlice";
import brandingReducer from "./brandingSlice";
import i18nReducer from "./i18nSlice";
import notifyReducer, { persistSeenChatUuids } from "./notifySlice";

export const store = configureStore({
  reducer: {
    auth: authReducer,
    branding: brandingReducer,
    i18n: i18nReducer,
    notify: notifyReducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();

let lastAuth = store.getState().auth;
let lastSeen = store.getState().notify.seenChatUuids;

store.subscribe(() => {
  const state = store.getState();
  if (state.auth !== lastAuth) {
    lastAuth = state.auth;
    persistAuth(state.auth);
  }
  if (state.notify.seenChatUuids !== lastSeen) {
    lastSeen = state.notify.seenChatUuids;
    persistSeenChatUuids(lastSeen);
  }
});
