import { useUiStore } from "../../stores/ui";

export function TopBar() {
  const connected = useUiStore((s) => s.daemonConnected);
  const error = useUiStore((s) => s.daemonError);

  return (
    <header className="flex h-12 items-center justify-between border-b border-slate-700 bg-slate-950/80 px-4">
      <span className="text-sm text-slate-300">org.palisade.Daemon1</span>
      {connected ? (
        <span className="rounded bg-emerald-700/30 px-2 py-1 text-xs text-emerald-300">Connected</span>
      ) : (
        <span className="rounded bg-rose-700/30 px-2 py-1 text-xs text-rose-300" title={error}>
          Disconnected
        </span>
      )}
    </header>
  );
}
