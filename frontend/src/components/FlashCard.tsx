import { useState } from "react";
import type { ContentItem, Rating } from "../types";

interface Props {
  item: ContentItem;
  onRate: (rating: Rating) => void;
  disabled?: boolean;
}

const RATINGS: { value: Rating; label: string; className: string }[] = [
  { value: 1, label: "Again", className: "bg-rose-600 hover:bg-rose-500" },
  { value: 2, label: "Hard", className: "bg-amber-600 hover:bg-amber-500" },
  { value: 3, label: "Good", className: "bg-emerald-600 hover:bg-emerald-500" },
  { value: 4, label: "Easy", className: "bg-sky-600 hover:bg-sky-500" },
];

export function FlashCard({ item, onRate, disabled }: Props) {
  const [flipped, setFlipped] = useState(false);

  return (
    <div className="flex flex-col items-center gap-4">
      <div
        className="[perspective:1200px] w-full max-w-md h-56 cursor-pointer"
        role="button"
        aria-label="flashcard"
        tabIndex={0}
        onClick={() => setFlipped((f) => !f)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") setFlipped((f) => !f);
        }}
      >
        <div
          data-testid="flip-inner"
          className="relative w-full h-full transition-transform duration-500 [transform-style:preserve-3d]"
          style={{ transform: flipped ? "rotateY(180deg)" : "rotateY(0deg)" }}
        >
          <div className="absolute inset-0 [backface-visibility:hidden] rounded-2xl bg-slate-800 border border-slate-700 flex flex-col items-center justify-center p-6 text-center">
            <span className="text-xs uppercase tracking-wide text-slate-400">
              {item.kind}
            </span>
            <p className="mt-3 text-2xl font-semibold text-slate-100">{item.front}</p>
            <span className="mt-auto text-xs text-slate-500">tap to reveal</span>
          </div>
          <div className="absolute inset-0 [backface-visibility:hidden] [transform:rotateY(180deg)] rounded-2xl bg-slate-700 border border-slate-600 flex flex-col items-center justify-center p-6 text-center">
            <p className="text-lg text-slate-100">{item.back}</p>
            {item.source_url && (
              <a
                href={item.source_url}
                target="_blank"
                rel="noreferrer"
                className="mt-3 text-sm text-sky-300 underline"
                onClick={(e) => e.stopPropagation()}
              >
                source
              </a>
            )}
          </div>
        </div>
      </div>

      <div className="flex gap-2" data-testid="rating-row">
        {RATINGS.map((r) => (
          <button
            key={r.value}
            disabled={disabled}
            onClick={() => onRate(r.value)}
            className={`px-4 py-2 rounded-lg text-white text-sm font-medium disabled:opacity-40 ${r.className}`}
          >
            {r.label}
          </button>
        ))}
      </div>
    </div>
  );
}
