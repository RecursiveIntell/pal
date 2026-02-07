interface Props {
  snapshots: string[];
  selected?: string;
  onSelect: (id: string) => void;
  onRefresh: () => void;
  onCreate: () => void;
  onRestore: () => void;
  onExportSelected: () => void;
  onExportCurrent: () => void;
  onDeleteSelected: () => void;
}

export function SnapshotList({
  snapshots,
  selected,
  onSelect,
  onRefresh,
  onCreate,
  onRestore,
  onExportSelected,
  onExportCurrent,
  onDeleteSelected,
}: Props) {
  return (
    <aside className="flex min-h-0 w-80 flex-col border-r border-slate-700 bg-slate-900/30 p-3">
      <h3 className="mb-3 text-sm font-semibold">Snapshots</h3>
      <div className="mb-2 grid grid-cols-2 gap-2">
        <button type="button" onClick={onRefresh} className="rounded bg-slate-700 px-2 py-1 text-xs hover:bg-slate-600">
          Refresh
        </button>
        <button type="button" onClick={onCreate} className="rounded bg-blue-600 px-2 py-1 text-xs hover:bg-blue-500">
          Create Now
        </button>
      </div>
      <div className="mb-3 grid grid-cols-2 gap-2">
        <button
          type="button"
          onClick={onExportSelected}
          disabled={!selected}
          className="rounded bg-violet-600 px-2 py-1 text-xs hover:bg-violet-500 disabled:cursor-not-allowed disabled:opacity-40"
        >
          Export Selected
        </button>
        <button type="button" onClick={onExportCurrent} className="rounded bg-indigo-600 px-2 py-1 text-xs hover:bg-indigo-500">
          Export Current
        </button>
      </div>
      <button
        type="button"
        onClick={onRestore}
        disabled={!selected}
        className="mb-3 rounded bg-emerald-600 px-2 py-1 text-xs hover:bg-emerald-500 disabled:cursor-not-allowed disabled:opacity-40"
      >
        Restore Selected
      </button>
      <button
        type="button"
        onClick={onDeleteSelected}
        disabled={!selected}
        className="mb-3 rounded bg-rose-600 px-2 py-1 text-xs hover:bg-rose-500 disabled:cursor-not-allowed disabled:opacity-40"
      >
        Delete Selected
      </button>
      <div className="min-h-0 flex-1 space-y-2 overflow-y-auto">
        {snapshots.map((id) => (
          <button
            key={id}
            type="button"
            className={`w-full rounded px-2 py-2 text-left text-sm ${
              id === selected ? "bg-blue-600" : "bg-slate-800/70 hover:bg-slate-700"
            }`}
            onClick={() => onSelect(id)}
          >
            {id}
          </button>
        ))}
      </div>
    </aside>
  );
}
