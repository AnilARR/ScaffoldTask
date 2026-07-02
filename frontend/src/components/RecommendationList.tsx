import type { Recommendation } from "../types";

interface Props {
  recommendations: Recommendation[];
  selectedId?: string;
  onSelect: (itemId: string) => void;
}

export function RecommendationList({ recommendations, selectedId, onSelect }: Props) {
  if (recommendations.length === 0) {
    return <p className="text-slate-400 text-sm">No content available for this profile yet.</p>;
  }
  return (
    <ul className="space-y-2" data-testid="recommendation-list">
      {recommendations.map((r) => (
        <li key={r.item.id}>
          <button
            onClick={() => onSelect(r.item.id)}
            className={`w-full text-left rounded-lg border p-3 transition ${
              r.item.id === selectedId
                ? "bg-slate-700 border-sky-500"
                : "bg-slate-800 border-slate-700 hover:border-slate-500"
            }`}
          >
            <div className="flex items-center justify-between">
              <span className="font-medium text-slate-100">{r.item.title}</span>
              <span className="text-xs text-slate-400">{r.item.kind}</span>
            </div>
            <div className="mt-1 flex gap-3 text-xs text-slate-400">
              <span>fit {Math.round(r.score * 100)}%</span>
              <span>{Math.round(r.comprehensible_ratio * 100)}% comprehensible</span>
              <span>{r.new_concepts} new</span>
            </div>
          </button>
        </li>
      ))}
    </ul>
  );
}
