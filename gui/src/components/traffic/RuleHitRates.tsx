import { useEffect, useState } from "react";
import type { LiveCounterDelta } from "../../types/traffic";

interface Props {
  deltas: LiveCounterDelta[];
}

export function RuleHitRates({ deltas }: Props) {
  const [now, setNow] = useState(Date.now());
  const total = deltas.reduce((sum, item) => sum + item.packetsPerSec, 0);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 200);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <section className="rounded border border-slate-700 bg-slate-900/50 p-3">
      <h3 className="mb-2 text-sm font-semibold">Rule Hit Rates (Live)</h3>
      <div className="space-y-2">
        {deltas.slice(0, 12).map((delta) => {
          const pct = total > 0 ? (delta.packetsPerSec / total) * 100 : 0;
          return (
            <div key={delta.key} className="text-xs">
              <div className="mb-1 flex items-center justify-between">
                <span className="truncate">{delta.table}/{delta.chain} #{delta.handle}</span>
                <span>{pct.toFixed(1)}%</span>
              </div>
              <div className="h-2 overflow-hidden rounded bg-slate-800">
                <div className="h-full bg-emerald-600" style={{ width: `${Math.min(100, pct)}%` }} />
              </div>
              <div className="mt-1 flex items-center justify-between text-[11px] text-slate-300">
                <span>{Math.max(0, (now - delta.lastSeenAt) / 1000).toFixed(1)}s ago</span>
                <span className="text-amber-300">{Math.max(0, (delta.expiresAt - now) / 1000).toFixed(1)}s</span>
              </div>
            </div>
          );
        })}
        {deltas.length === 0 && <div className="text-xs text-slate-400">No live rule hit activity.</div>}
      </div>
    </section>
  );
}
