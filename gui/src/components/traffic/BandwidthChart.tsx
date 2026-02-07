import { Area, AreaChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import type { TrafficPoint } from "../../stores/traffic";

interface Props {
  history: TrafficPoint[];
  sampleMs: number;
}

export function BandwidthChart({ history, sampleMs }: Props) {
  const chartData = history.map((point) => ({
    t: new Date(point.timestamp).toLocaleTimeString(),
    bytes: point.bytesPerSec,
    packets: point.packetsPerSec,
  }));

  const sampleLabel = sampleMs % 1000 === 0 ? `${sampleMs / 1000}s` : `${(sampleMs / 1000).toFixed(1)}s`;

  return (
    <section className="rounded border border-slate-700 bg-slate-900/50 p-3">
      <h3 className="mb-2 text-sm font-semibold">Bandwidth ({sampleLabel} samples)</h3>
      <div className="h-64">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={chartData}>
            <defs>
              <linearGradient id="bytes" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#2563eb" stopOpacity={0.8} />
                <stop offset="95%" stopColor="#2563eb" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
            <XAxis dataKey="t" hide />
            <YAxis stroke="#94a3b8" />
            <Tooltip
              contentStyle={{ backgroundColor: "#0f172a", border: "1px solid #334155", borderRadius: 6 }}
              labelStyle={{ color: "#cbd5e1" }}
            />
            <Area type="monotone" dataKey="bytes" stroke="#2563eb" fillOpacity={1} fill="url(#bytes)" />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </section>
  );
}
