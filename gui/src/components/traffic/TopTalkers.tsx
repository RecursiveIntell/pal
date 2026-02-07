import { useEffect, useState } from "react";
import type { LiveFlowEntry } from "../../types/traffic";

interface Props {
  flows: LiveFlowEntry[];
}

export function TopTalkers({ flows }: Props) {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 200);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <section className="rounded border border-slate-700 bg-slate-900/50 p-3">
      <h3 className="mb-2 text-sm font-semibold">Top Talkers (Live)</h3>
      <div className="space-y-2">
        {flows.slice(0, 8).map((flow) => (
          <div key={flow.key} className="rounded bg-slate-800/60 p-2 text-xs">
            <div className="truncate text-slate-100">{flow.src}</div>
            <div className="truncate text-slate-400">{flow.dst}</div>
            <div className="mt-1 text-blue-300">{flow.bytesPerSec.toLocaleString()} B/s</div>
            <div className="mt-1 flex items-center justify-between text-[11px] text-slate-300">
              <span>{Math.max(0, (now - flow.lastSeenAt) / 1000).toFixed(1)}s ago</span>
              <span className="text-amber-300">{Math.max(0, (flow.expiresAt - now) / 1000).toFixed(1)}s</span>
            </div>
          </div>
        ))}
        {flows.length === 0 && <div className="text-xs text-slate-400">No active flow events.</div>}
      </div>
    </section>
  );
}
