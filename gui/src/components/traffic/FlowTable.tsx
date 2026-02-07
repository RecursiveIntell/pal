import { useEffect, useState } from "react";
import type { LiveFlowEntry } from "../../types/traffic";

interface Props {
  flows: LiveFlowEntry[];
}

export function FlowTable({ flows }: Props) {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 200);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <section className="rounded border border-slate-700 bg-slate-900/50 p-3">
      <h3 className="mb-2 text-sm font-semibold">Live Flow Feed</h3>
      <div className="max-h-72 overflow-auto">
        <table className="w-full text-xs">
          <thead className="text-left text-slate-400">
            <tr>
              <th className="px-2 py-1">Source</th>
              <th className="px-2 py-1">Target</th>
              <th className="px-2 py-1">Protocol</th>
              <th className="px-2 py-1">Bytes/s</th>
              <th className="px-2 py-1">Pkts/s</th>
              <th className="px-2 py-1">Last Seen</th>
              <th className="px-2 py-1">TTL</th>
            </tr>
          </thead>
          <tbody>
            {flows.map((flow) => (
              <tr key={flow.key} className="border-t border-slate-800">
                <td className="px-2 py-1">{flow.src}</td>
                <td className="px-2 py-1">{flow.dst}</td>
                <td className="px-2 py-1">{flow.protocol}</td>
                <td className="px-2 py-1">{flow.bytesPerSec.toLocaleString()}</td>
                <td className="px-2 py-1">{flow.packetsPerSec.toLocaleString()}</td>
                <td className="px-2 py-1 text-slate-300">
                  {Math.max(0, (now - flow.lastSeenAt) / 1000).toFixed(1)}s ago
                </td>
                <td className="px-2 py-1 text-amber-300">
                  {Math.max(0, (flow.expiresAt - now) / 1000).toFixed(1)}s
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {flows.length === 0 && <div className="px-2 py-3 text-xs text-slate-400">No live flow events yet.</div>}
      </div>
    </section>
  );
}
