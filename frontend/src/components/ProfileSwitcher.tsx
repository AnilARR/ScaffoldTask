import type { Profile } from "../types";

interface Props {
  profiles: Profile[];
  selectedId?: string;
  onSelect: (id: string) => void;
}

export function ProfileSwitcher({ profiles, selectedId, onSelect }: Props) {
  return (
    <div className="flex flex-wrap gap-2" data-testid="profile-switcher">
      {profiles.map((p) => (
        <button
          key={p.id}
          onClick={() => onSelect(p.id)}
          aria-pressed={p.id === selectedId}
          className={`px-3 py-2 rounded-lg text-sm border transition ${
            p.id === selectedId
              ? "bg-sky-600 border-sky-400 text-white"
              : "bg-slate-800 border-slate-700 text-slate-300 hover:border-slate-500"
          }`}
        >
          <span className="font-medium">{p.name}</span>
          <span className="ml-2 text-xs opacity-70">{p.level}</span>
        </button>
      ))}
    </div>
  );
}
