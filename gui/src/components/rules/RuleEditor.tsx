import { useMemo, useState } from "react";
import type { RuleSpec } from "../../types/changeset";
import type { RuleAction, RuleMatch } from "../../types/nftables";
import { ActionSelector } from "./ActionSelector";
import { MatchConditionForm } from "./MatchConditionForm";
import { RulePreview } from "./RulePreview";

interface Props {
  onSave: (rule: RuleSpec) => void;
  readOnlyReason?: string;
}

export function RuleEditor({ onSave, readOnlyReason }: Props) {
  const [matches, setMatches] = useState<RuleMatch[]>([]);
  const [action, setAction] = useState<RuleAction>({ type: "accept" });
  const [comment, setComment] = useState("");

  const draft = useMemo<RuleSpec>(() => ({ matches, action, comment }), [matches, action, comment]);

  return (
    <section className="space-y-4 border-t border-slate-700 bg-slate-900/40 p-4">
      <h3 className="text-base font-semibold">Rule Editor</h3>
      {readOnlyReason && (
        <div className="rounded border border-amber-700/50 bg-amber-600/10 px-3 py-2 text-xs text-amber-300">
          {readOnlyReason}
        </div>
      )}
      <MatchConditionForm matches={matches} onChange={setMatches} />
      <ActionSelector action={action} onChange={setAction} />
      <div>
        <h4 className="mb-2 text-sm font-semibold">Comment</h4>
        <input
          value={comment}
          onChange={(event) => setComment(event.target.value)}
          className="w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm"
          placeholder="Optional comment"
          disabled={Boolean(readOnlyReason)}
        />
      </div>
      <RulePreview rule={draft} />
      <button
        type="button"
        onClick={() => onSave(draft)}
        className="rounded bg-blue-600 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-60"
        disabled={Boolean(readOnlyReason)}
      >
        Stage Rule
      </button>
    </section>
  );
}
