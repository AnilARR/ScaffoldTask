import type { ActivityDay } from "../types";

interface Props {
  activity: ActivityDay[];
  weeks?: number;
}

function levelClass(count: number): string {
  if (count === 0) return "bg-slate-800";
  if (count < 2) return "bg-emerald-900";
  if (count < 4) return "bg-emerald-700";
  if (count < 7) return "bg-emerald-500";
  return "bg-emerald-300";
}

/** GitHub-style contribution grid over the last `weeks` weeks. */
export function ActivityGraph({ activity, weeks = 12 }: Props) {
  const counts = new Map(activity.map((a) => [a.date, a.count]));
  const days: { date: string; count: number }[] = [];
  const today = new Date();
  const total = weeks * 7;
  for (let i = total - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    const key = d.toISOString().slice(0, 10);
    days.push({ date: key, count: counts.get(key) ?? 0 });
  }

  const columns: { date: string; count: number }[][] = [];
  for (let w = 0; w < weeks; w++) {
    columns.push(days.slice(w * 7, w * 7 + 7));
  }

  return (
    <div className="flex gap-1" data-testid="activity-graph">
      {columns.map((col, ci) => (
        <div key={ci} className="flex flex-col gap-1">
          {col.map((d) => (
            <div
              key={d.date}
              title={`${d.date}: ${d.count} reviews`}
              data-testid="activity-cell"
              className={`w-3 h-3 rounded-sm ${levelClass(d.count)}`}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
