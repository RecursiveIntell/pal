import { create } from "zustand";

export type ViewName = "rules" | "traffic" | "templates" | "snapshots";

interface UiState {
  view: ViewName;
  deadManSeconds?: number;
  trafficRefreshMs: number;
  trafficLingerSeconds: number;
  setView: (view: ViewName) => void;
  setDeadManSeconds: (seconds?: number) => void;
  setTrafficRefreshMs: (ms: number) => void;
  setTrafficLingerSeconds: (seconds: number) => void;
}

export const useUiStore = create<UiState>((set) => ({
  view: "rules",
  trafficRefreshMs: 1000,
  trafficLingerSeconds: 5,
  setView: (view) => set({ view }),
  setDeadManSeconds: (deadManSeconds) => set({ deadManSeconds }),
  setTrafficRefreshMs: (trafficRefreshMs) => set({ trafficRefreshMs }),
  setTrafficLingerSeconds: (trafficLingerSeconds) =>
    set({ trafficLingerSeconds: Math.max(1, Math.min(99, Math.trunc(trafficLingerSeconds) || 1)) }),
}));
