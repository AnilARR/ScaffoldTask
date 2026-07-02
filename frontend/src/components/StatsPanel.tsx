import type { Stats } from "../types";
import { ActivityGraph } from "./ActivityGraph";

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl bg-slate-800 border border-slate-700 p-4">
      <div className="text-2xl font-semibold text-slate-100">{value}</div>
      <div className="text-xs uppercase tracking-wide text-slate-400 mt-1">{label}</div>
    </div>
  );
}

export function StatsPanel({ stats }: { stats: Stats }) {
  const pct = (n: number) => `${Math.round(n * 100)}%`;
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <Metric label="Concepts tracked" value={String(stats.concepts_tracked)} />
        <Metric label="Concepts known" value={String(stats.concepts_known)} />
        <Metric label="Avg freshness" value={pct(stats.average_freshness)} />
        <Metric label="Reviews" value={String(stats.reviews_total)} />
      </div>
      <div>
        <h3 className="text-sm text-slate-400 mb-2">Activity</h3>
        <ActivityGraph activity={stats.activity} />
      </div>
    </div>
  );
}
