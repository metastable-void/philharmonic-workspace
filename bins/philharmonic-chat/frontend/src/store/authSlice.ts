import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

const TOKEN_STORAGE_KEY = "agent_token";
const NAME_STORAGE_KEY = "agent_name";

export interface AuthState {
  agentToken: string;
  agentName: string;
  isSignedIn: boolean;
}

function storedValue(key: string): string {
  try {
    return window.localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

export function persistAuth(state: AuthState): void {
  try {
    if (state.agentToken.length === 0) {
      window.localStorage.removeItem(TOKEN_STORAGE_KEY);
    } else {
      window.localStorage.setItem(TOKEN_STORAGE_KEY, state.agentToken);
    }
    if (state.agentName.length === 0) {
      window.localStorage.removeItem(NAME_STORAGE_KEY);
    } else {
      window.localStorage.setItem(NAME_STORAGE_KEY, state.agentName);
    }
  } catch {
  }
}

const initialAgentToken = storedValue(TOKEN_STORAGE_KEY);

const initialState: AuthState = {
  agentToken: initialAgentToken,
  agentName: storedValue(NAME_STORAGE_KEY),
  isSignedIn: initialAgentToken.length > 0,
};

export const authSlice = createSlice({
  name: "auth",
  initialState,
  reducers: {
    setAgentToken(state, action: PayloadAction<string>) {
      state.agentToken = action.payload.trim();
      state.isSignedIn = state.agentToken.length > 0;
    },
    setAgentName(state, action: PayloadAction<string>) {
      state.agentName = action.payload.trim();
    },
    signOut(state) {
      state.agentToken = "";
      state.isSignedIn = false;
    },
  },
});

export const { setAgentName, setAgentToken, signOut } = authSlice.actions;
export default authSlice.reducer;
