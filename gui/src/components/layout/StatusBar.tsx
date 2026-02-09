import { useUiStore } from "../../stores/ui";

export function StatusBar() {
  const connected = useUiStore((s) => s.daemonConnected);
  const error = useUiStore((s) => s.daemonError);

  return (
    <footer className="flex h-8 items-center justify-between border-t border-slate-700 bg-slate-950/80 px-4 text-xs text-slate-400">
      {connected ? (
        <span>Dead man switch enabled</span>
      ) : (
        <span className="text-rose-400">{error ?? "Cannot connect to daemon"}</span>
      )}
      <span>nft JSON API</span>
    </footer>
  );
}
