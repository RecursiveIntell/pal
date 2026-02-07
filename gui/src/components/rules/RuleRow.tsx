import type { NftRule } from "../../types/nftables";

interface Props {
  rule: NftRule;
  selected: boolean;
  onClick: () => void;
}

export function RuleRow({ rule, selected, onClick }: Props) {
  const color =
    rule.summary.action.type === "accept"
      ? "bg-emerald-700/30 text-emerald-300"
      : rule.summary.action.type === "drop" || rule.summary.action.type === "reject"
        ? "bg-rose-700/30 text-rose-300"
        : "bg-slate-700 text-slate-200";

  return (
    <tr
      onClick={onClick}
      className={`cursor-pointer border-b border-slate-800 text-sm hover:bg-slate-800/60 ${selected ? "bg-slate-800/80" : ""}`}
    >
      <td className="px-3 py-2 font-mono text-xs">{rule.handle}</td>
      <td className="px-3 py-2">{rule.summary.description}</td>
      <td className="px-3 py-2">
        <span className={`rounded px-2 py-1 text-xs ${color}`}>{rule.summary.action.type}</span>
      </td>
      <td className="px-3 py-2 text-slate-400">{rule.comment ?? ""}</td>
    </tr>
  );
}
