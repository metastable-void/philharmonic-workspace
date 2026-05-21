import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export interface AwaitingChat {
  instanceId: string;
  firstSeenAt: number;
}

export interface NotifyState {
  seenChatUuids: string[];
  awaiting: AwaitingChat[];
}

function storedSeen(): string[] {
  try {
    const raw = window.localStorage.getItem("seen_chat_uuids");
    const parsed = raw === null ? [] : (JSON.parse(raw) as unknown);
    return Array.isArray(parsed)
      ? parsed.filter((value): value is string => typeof value === "string")
      : [];
  } catch {
    return [];
  }
}

export function persistSeenChatUuids(values: string[]): void {
  try {
    window.localStorage.setItem("seen_chat_uuids", JSON.stringify(values));
  } catch {
  }
}

const initialState: NotifyState = {
  seenChatUuids: storedSeen(),
  awaiting: [],
};

export const notifySlice = createSlice({
  name: "notify",
  initialState,
  reducers: {
    addAwaiting(state, action: PayloadAction<string>) {
      const instanceId = action.payload;
      if (!state.seenChatUuids.includes(instanceId)) {
        state.seenChatUuids.push(instanceId);
      }
      if (!state.awaiting.some((item) => item.instanceId === instanceId)) {
        state.awaiting.unshift({ instanceId, firstSeenAt: Date.now() });
      }
      state.awaiting.sort((left, right) => right.firstSeenAt - left.firstSeenAt);
    },
  },
});

export const { addAwaiting } = notifySlice.actions;
export default notifySlice.reducer;
