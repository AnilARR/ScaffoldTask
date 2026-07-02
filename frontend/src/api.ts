import type {
  Course,
  CourseDetail,
  Profile,
  Rating,
  Recommendation,
  Stats,
} from "./types";

const BASE = import.meta.env.VITE_API_BASE ?? "";

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { "content-type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    let detail = res.statusText;
    try {
      const body = await res.json();
      detail = body.error ?? detail;
    } catch {
      // ignore parse error
    }
    throw new Error(`${res.status}: ${detail}`);
  }
  return (await res.json()) as T;
}

export const api = {
  health: () => req<{ status: string }>("/api/health"),
  profiles: () => req<Profile[]>("/api/profiles"),
  courses: () => req<Course[]>("/api/courses"),
  course: (slug: string) => req<CourseDetail>(`/api/courses/${slug}`),
  recommend: (profileId: string, slug: string) =>
    req<Recommendation[]>(`/api/profiles/${profileId}/courses/${slug}/recommend`),
  stats: (profileId: string) => req<Stats>(`/api/profiles/${profileId}/stats`),
  review: (profileId: string, itemId: string, rating: Rating) =>
    req(`/api/review`, {
      method: "POST",
      body: JSON.stringify({ profile_id: profileId, item_id: itemId, rating }),
    }),
  ankiDecks: () => req<string[]>("/api/anki/decks"),
};
