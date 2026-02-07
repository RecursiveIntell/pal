interface Props {
  seconds: number;
  onKeep: () => void;
  onRollback: () => void;
}

export function DeadManCountdown({ seconds, onKeep, onRollback }: Props) {
  return (
    <div className="flex items-center justify-between rounded border border-amber-600/40 bg-amber-900/20 p-3 text-sm">
      <span>Rollback in {seconds}s unless confirmed.</span>
      <div className="flex gap-2">
        <button type="button" className="rounded bg-emerald-700 px-2 py-1 text-xs" onClick={onKeep}>
          Keep
        </button>
        <button type="button" className="rounded bg-rose-700 px-2 py-1 text-xs" onClick={onRollback}>
          Rollback
        </button>
      </div>
    </div>
  );
}
