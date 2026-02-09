import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";
import { useUiStore } from "../stores/ui";

export async function daemonCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export function useDaemonConnection() {
  const setDaemonConnected = useUiStore((s) => s.setDaemonConnected);
  const setDaemonError = useUiStore((s) => s.setDaemonError);
  const timerRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;

    const check = async () => {
      try {
        await daemonCall<boolean>("check_daemon_connection");
        if (!cancelled) {
          setDaemonConnected(true);
          setDaemonError(undefined);
        }
      } catch (error) {
        if (!cancelled) {
          setDaemonConnected(false);
          setDaemonError(String(error));
        }
      }
    };

    void check();
    timerRef.current = window.setInterval(() => {
      void check();
    }, 5_000);

    return () => {
      cancelled = true;
      if (timerRef.current !== undefined) {
        window.clearInterval(timerRef.current);
      }
    };
  }, [setDaemonConnected, setDaemonError]);
}
