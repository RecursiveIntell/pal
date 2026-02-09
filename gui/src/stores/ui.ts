import { create } from "zustand";

export type ViewName = "rules" | "traffic" | "templates" | "snapshots" | "hosts";

interface UiState {
  view: ViewName;
  deadManSeconds?: number;
  trafficRefreshMs: number;
  trafficLingerSeconds: number;
  daemonConnected: boolean;
  daemonError?: string;
  setView: (view: ViewName) => void;
  setDeadManSeconds: (seconds?: number) => void;
  setTrafficRefreshMs: (ms: number) => void;
  setTrafficLingerSeconds: (seconds: number) => void;
  setDaemonConnected: (connected: boolean) => void;
  setDaemonError: (error?: string) => void;
}

export const useUiStore = create<UiState>((set) => ({
  view: "rules",
  trafficRefreshMs: 1000,
  trafficLingerSeconds: 5,
  daemonConnected: false,
  daemonError: undefined,
  setView: (view) => set({ view }),
  setDeadManSeconds: (deadManSeconds) => set({ deadManSeconds }),
  setTrafficRefreshMs: (trafficRefreshMs) => set({ trafficRefreshMs }),
  setTrafficLingerSeconds: (trafficLingerSeconds) =>
    set({ trafficLingerSeconds: Math.max(1, Math.min(99, Math.trunc(trafficLingerSeconds) || 1)) }),
  setDaemonConnected: (daemonConnected) => set({ daemonConnected }),
  setDaemonError: (daemonError) => set({ daemonError }),
}));
