import type { RuleMatch } from "../../types/nftables";

interface Props {
  matches: RuleMatch[];
  onChange: (matches: RuleMatch[]) => void;
}

export function MatchConditionForm({ matches, onChange }: Props) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-semibold">Match Conditions</h4>
        <button
          type="button"
          className="rounded bg-blue-600 px-2 py-1 text-xs"
          onClick={() =>
            onChange([...matches, { field: "tcp_dport", operator: "==", value: "22", negated: false }])
          }
        >
          Add
        </button>
      </div>
      {matches.map((match, index) => (
        <div key={`${match.field}-${index}`} className="grid grid-cols-4 gap-2">
          <input
            value={match.field}
            onChange={(event) => {
              const next = [...matches];
              next[index] = { ...match, field: event.target.value };
              onChange(next);
            }}
            className="rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs"
          />
          <input
            value={match.operator}
            onChange={(event) => {
              const next = [...matches];
              next[index] = { ...match, operator: event.target.value };
              onChange(next);
            }}
            className="rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs"
          />
          <input
            value={match.value}
            onChange={(event) => {
              const next = [...matches];
              next[index] = { ...match, value: event.target.value };
              onChange(next);
            }}
            className="rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs"
          />
          <button
            type="button"
            className="rounded bg-rose-700/30 px-2 py-1 text-xs text-rose-300"
            onClick={() => onChange(matches.filter((_, i) => i !== index))}
          >
            Remove
          </button>
        </div>
      ))}
    </div>
  );
}
