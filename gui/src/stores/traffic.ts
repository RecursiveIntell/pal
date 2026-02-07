import { create } from "zustand";
import type { LiveCounterDelta, LiveFlowEntry, TrafficSummary } from "../types/traffic";

export interface TrafficPoint {
  timestamp: number;
  bytesPerSec: number;
  packetsPerSec: number;
}

interface TrafficState {
  summary: TrafficSummary;
  history: TrafficPoint[];
  liveFlows: LiveFlowEntry[];
  liveCounterDeltas: LiveCounterDelta[];
  error?: string;
  monitoring: boolean;
  setSummary: (summary: TrafficSummary, lingerMs?: number) => void;
  setError: (error?: string) => void;
  setMonitoring: (monitoring: boolean) => void;
  resetHistory: () => void;
}

export const useTrafficStore = create<TrafficState>((set) => ({
  summary: { totalBytesPerSec: 0, totalPacketsPerSec: 0, topFlows: [], counterDeltas: [] },
  history: [],
  liveFlows: [],
  liveCounterDeltas: [],
  error: undefined,
  monitoring: false,
  setSummary: (summary, lingerMs = 5_000) =>
    set((state) => {
      const now = Date.now();
      const ttlMs = Math.max(1_000, Math.min(99_000, Math.trunc(lingerMs) || 5_000));
      const point: TrafficPoint = {
        timestamp: now,
        bytesPerSec: summary.totalBytesPerSec,
        packetsPerSec: summary.totalPacketsPerSec,
      };

      const active = new Map<string, LiveFlowEntry>();
      for (const flow of state.liveFlows) {
        if (flow.expiresAt > now) {
          active.set(flow.key, flow);
        }
      }

      for (const flow of summary.topFlows) {
        const key = `${flow.src}|${flow.dst}|${flow.protocol}`;
        active.set(key, {
          ...flow,
          key,
          lastSeenAt: now,
          expiresAt: now + ttlMs,
        });
      }

      const liveFlows = [...active.values()]
        .sort((a, b) => b.lastSeenAt - a.lastSeenAt)
        .slice(0, 120);

      const activeRuleHits = new Map<string, LiveCounterDelta>();
      for (const delta of state.liveCounterDeltas) {
        if (delta.expiresAt > now) {
          activeRuleHits.set(delta.key, delta);
        }
      }

      for (const delta of summary.counterDeltas) {
        const key = `${delta.family}|${delta.table}|${delta.chain}|${delta.handle}`;
        activeRuleHits.set(key, {
          ...delta,
          key,
          lastSeenAt: now,
          expiresAt: now + ttlMs,
        });
      }

      const liveCounterDeltas = [...activeRuleHits.values()]
        .sort((a, b) => b.lastSeenAt - a.lastSeenAt)
        .slice(0, 160);

      return {
        summary,
        history: [...state.history.filter((item) => now - item.timestamp <= 120_000), point],
        liveFlows,
        liveCounterDeltas,
      };
    }),
  setError: (error) => set({ error }),
  setMonitoring: (monitoring) => set({ monitoring }),
  resetHistory: () => set({ history: [], liveFlows: [], liveCounterDeltas: [] }),
}));
