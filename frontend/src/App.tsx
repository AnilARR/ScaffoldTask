import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import type { Course, Profile, Rating, Recommendation, Stats } from "./types";
import { ProfileSwitcher } from "./components/ProfileSwitcher";
import { RecommendationList } from "./components/RecommendationList";
import { FlashCard } from "./components/FlashCard";
import { StatsPanel } from "./components/StatsPanel";

export default function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [courses, setCourses] = useState<Course[]>([]);
  const [profileId, setProfileId] = useState<string>();
  const [courseSlug, setCourseSlug] = useState<string>();
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [selectedItemId, setSelectedItemId] = useState<string>();
  const [stats, setStats] = useState<Stats>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const [p, c] = await Promise.all([api.profiles(), api.courses()]);
        setProfiles(p);
        setCourses(c);
        if (p.length) setProfileId(p[0].id);
        if (c.length) setCourseSlug(c[0].slug);
      } catch (e) {
        setError(String(e));
      }
    })();
  }, []);

  const loadForProfile = useCallback(
    async (pid: string, slug: string) => {
      try {
        const [recs, s] = await Promise.all([api.recommend(pid, slug), api.stats(pid)]);
        setRecommendations(recs);
        setStats(s);
        setSelectedItemId(recs[0]?.item.id);
        setError(undefined);
      } catch (e) {
        setError(String(e));
      }
    },
    [],
  );

  useEffect(() => {
    if (profileId && courseSlug) loadForProfile(profileId, courseSlug);
  }, [profileId, courseSlug, loadForProfile]);

  const selectedItem = useMemo(
    () => recommendations.find((r) => r.item.id === selectedItemId)?.item,
    [recommendations, selectedItemId],
  );

  const handleRate = async (rating: Rating) => {
    if (!profileId || !selectedItem || !courseSlug) return;
    setBusy(true);
    try {
      await api.review(profileId, selectedItem.id, rating);
      await loadForProfile(profileId, courseSlug);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-900 text-slate-100">
      <header className="border-b border-slate-800 px-6 py-4">
        <h1 className="text-xl font-semibold">N+1 Comprehensible Input Platform</h1>
        <p className="text-sm text-slate-400">
          Freshness-tracked learning with i+1 content selection
        </p>
      </header>

      <main className="max-w-5xl mx-auto p-6 space-y-8">
        {error && (
          <div
            role="alert"
            className="rounded-lg border border-rose-700 bg-rose-950/50 text-rose-200 px-4 py-3 text-sm"
          >
            {error}
          </div>
        )}

        <section className="space-y-3">
          <h2 className="text-sm uppercase tracking-wide text-slate-400">Profile</h2>
          <ProfileSwitcher profiles={profiles} selectedId={profileId} onSelect={setProfileId} />
        </section>

        <section className="space-y-3">
          <h2 className="text-sm uppercase tracking-wide text-slate-400">Course</h2>
          <div className="flex flex-wrap gap-2">
            {courses.map((c) => (
              <button
                key={c.slug}
                onClick={() => setCourseSlug(c.slug)}
                aria-pressed={c.slug === courseSlug}
                className={`px-3 py-2 rounded-lg text-sm border ${
                  c.slug === courseSlug
                    ? "bg-emerald-600 border-emerald-400"
                    : "bg-slate-800 border-slate-700 hover:border-slate-500"
                }`}
              >
                {c.title}
              </button>
            ))}
          </div>
        </section>

        <div className="grid md:grid-cols-2 gap-8">
          <section className="space-y-3">
            <h2 className="text-sm uppercase tracking-wide text-slate-400">
              Recommended (N+1 fit)
            </h2>
            <RecommendationList
              recommendations={recommendations}
              selectedId={selectedItemId}
              onSelect={setSelectedItemId}
            />
          </section>

          <section className="space-y-4">
            <h2 className="text-sm uppercase tracking-wide text-slate-400">Study</h2>
            {selectedItem ? (
              <FlashCard item={selectedItem} onRate={handleRate} disabled={busy} />
            ) : (
              <p className="text-slate-400 text-sm">Select an item to study.</p>
            )}
          </section>
        </div>

        {stats && (
          <section className="space-y-3">
            <h2 className="text-sm uppercase tracking-wide text-slate-400">Analytics</h2>
            <StatsPanel stats={stats} />
          </section>
        )}
      </main>
    </div>
  );
}
