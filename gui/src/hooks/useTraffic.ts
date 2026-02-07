import { useEffect } from "react";
import { daemonCall } from "./useDaemon";
import { useTrafficStore } from "../stores/traffic";
import type { CounterDelta, FlowEntry, TrafficSummary } from "../types/traffic";

interface DaemonTrafficSummary {
  total_bytes_per_sec: unknown;
  total_packets_per_sec: unknown;
  top_flows: unknown;
  counter_deltas: unknown;
}

export function useTraffic(enabled: boolean, refreshMs: number, lingerSeconds: number) {
  const setSummary = useTrafficStore((s) => s.setSummary);
  const setError = useTrafficStore((s) => s.setError);
  const setMonitoring = useTrafficStore((s) => s.setMonitoring);
  const resetHistory = useTrafficStore((s) => s.resetHistory);

  useEffect(() => {
    if (!enabled) {
      setMonitoring(false);
      return;
    }

    let stopped = false;
    let socketPath = "";
    const lingerMs = Math.max(1, Math.min(99, Math.trunc(lingerSeconds) || 1)) * 1000;

    const boot = async () => {
      try {
        resetHistory();
        setError(undefined);
        socketPath = await daemonCall<string>("get_monitor_socket_path");
        await daemonCall<boolean>("start_monitoring");
        setMonitoring(true);

        const poll = async () => {
          if (stopped) {
            return;
          }
          try {
            const raw = await daemonCall<string>("read_monitor_sample", { socketPath });
            const parsed = toTrafficSummary(JSON.parse(raw) as DaemonTrafficSummary);
            setSummary(parsed, lingerMs);
            setError(undefined);
          } catch (error) {
            if (!stopped) {
              setError(`Unable to read traffic sample from monitor socket: ${String(error)}`);
            }
          }
        };

        await poll();
        const timer = window.setInterval(() => {
          void poll();
        }, refreshMs);

        return () => {
          window.clearInterval(timer);
        };
      } catch (error) {
        setSummary({ totalBytesPerSec: 0, totalPacketsPerSec: 0, topFlows: [], counterDeltas: [] }, lingerMs);
        setError(`Monitoring unavailable: ${String(error)}`);
        setMonitoring(false);
        return () => {};
      }
    };

    let cleanupInner: (() => void) | undefined;
    void boot().then((cleanup) => {
      cleanupInner = cleanup;
    });

    return () => {
      stopped = true;
      cleanupInner?.();
      setMonitoring(false);
    };
  }, [enabled, lingerSeconds, refreshMs, resetHistory, setError, setMonitoring, setSummary]);
}

function toTrafficSummary(input: DaemonTrafficSummary): TrafficSummary {
  return {
    totalBytesPerSec: asNumber(input.total_bytes_per_sec),
    totalPacketsPerSec: asNumber(input.total_packets_per_sec),
    topFlows: asFlowEntries(input.top_flows),
    counterDeltas: asCounterDeltas(input.counter_deltas),
  };
}

function asFlowEntries(value: unknown): FlowEntry[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .filter(isRecord)
    .map((item) => ({
      src: asString(item.src),
      dst: asString(item.dst),
      protocol: asString(item.protocol),
      bytesPerSec: asNumber(item.bytes_per_sec),
      packetsPerSec: asNumber(item.packets_per_sec),
    }));
}

function asCounterDeltas(value: unknown): CounterDelta[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .filter(isRecord)
    .map((item) => ({
      family: asString(item.family),
      table: asString(item.table),
      chain: asString(item.chain),
      handle: asNumber(item.handle),
      packetsPerSec: asNumber(item.packets_per_sec),
      bytesPerSec: asNumber(item.bytes_per_sec),
    }));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function asNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}
