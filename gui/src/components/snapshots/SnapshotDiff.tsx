interface Props {
  left?: string;
  right?: string;
  leftTitle?: string;
  rightTitle?: string;
}

export function SnapshotDiff({ left, right, leftTitle = "Snapshot", rightTitle = "Current Ruleset" }: Props) {
  return (
    <section className="grid min-h-0 flex-1 grid-cols-2 gap-3 p-4">
      <div className="flex min-h-0 flex-col">
        <h4 className="mb-2 text-sm font-semibold">{leftTitle}</h4>
        <pre className="min-h-0 flex-1 overflow-auto rounded border border-slate-700 bg-slate-950 p-3 text-xs">
          {left ?? "Select a snapshot to load its content."}
        </pre>
      </div>
      <div className="flex min-h-0 flex-col">
        <h4 className="mb-2 text-sm font-semibold">{rightTitle}</h4>
        <pre className="min-h-0 flex-1 overflow-auto rounded border border-slate-700 bg-slate-950 p-3 text-xs">
          {right ?? ""}
        </pre>
      </div>
    </section>
  );
}
