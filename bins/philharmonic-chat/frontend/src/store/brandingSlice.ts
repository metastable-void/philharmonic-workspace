import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export interface BrandingState {
  name: string;
  monogram: string;
  loaded: boolean;
}

const initialState: BrandingState = {
  name: "Philharmonic",
  monogram: "P",
  loaded: false,
};

export const brandingSlice = createSlice({
  name: "branding",
  initialState,
  reducers: {
    setBranding(state, action: PayloadAction<{ name: string; monogram: string }>) {
      state.name = action.payload.name;
      state.monogram = action.payload.monogram;
      state.loaded = true;
    },
  },
});

export const { setBranding } = brandingSlice.actions;
export default brandingSlice.reducer;
