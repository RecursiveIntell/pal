import type { RuleSpec } from "../../types/changeset";

interface Props {
  rule: RuleSpec;
}

export function RulePreview({ rule }: Props) {
  const matchText = rule.matches
    .map((m) => `${m.negated ? "! " : ""}${m.field} ${m.operator} ${m.value}`)
    .join(" ");
  const action = rule.action.type;

  return (
    <div>
      <h4 className="mb-2 text-sm font-semibold">Live nft preview</h4>
      <pre className="overflow-x-auto rounded border border-slate-700 bg-slate-950 p-2 text-xs text-blue-200">
        {`add rule inet palisade input ${matchText} ${action}`}
      </pre>
    </div>
  );
}
