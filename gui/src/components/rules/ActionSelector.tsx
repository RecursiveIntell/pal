import type { RuleAction } from "../../types/nftables";

interface Props {
  action: RuleAction;
  onChange: (action: RuleAction) => void;
}

const actions: RuleAction["type"][] = [
  "accept",
  "drop",
  "reject",
  "jump",
  "goto",
  "return",
  "masquerade",
  "snat",
  "dnat",
  "redirect",
  "log",
  "counter",
  "limit",
];

export function ActionSelector({ action, onChange }: Props) {
  return (
    <div>
      <h4 className="mb-2 text-sm font-semibold">Action</h4>
      <div className="grid grid-cols-3 gap-2 text-xs">
        {actions.map((value) => (
          <label key={value} className="flex items-center gap-2 rounded border border-slate-700 bg-slate-900 px-2 py-1">
            <input
              type="radio"
              checked={action.type === value}
              onChange={() => onChange({ type: value })}
            />
            {value}
          </label>
        ))}
      </div>
    </div>
  );
}
