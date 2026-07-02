import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import App from "./App";
import type { Course, Profile, Recommendation, Stats } from "./types";

const profiles: Profile[] = [
  { id: "p1", name: "Aiko", level: "beginner", interests: ["language"], created_at: "2024-01-01" },
];
const courses: Course[] = [
  {
    id: "c1",
    slug: "japanese-103",
    title: "Japanese 103",
    kind: "language",
    description: "",
  },
];
const recs: Recommendation[] = [
  {
    item: {
      id: "i1",
      course_id: "c1",
      kind: "flashcard",
      title: "Water card",
      front: "水",
      back: "water",
      concept_ids: ["k1"],
      tags: ["language"],
      difficulty: 0.2,
      source_url: null,
    },
    score: 0.8,
    comprehensible_ratio: 0.9,
    new_concepts: 1,
  },
];
const stats: Stats = {
  profile_id: "p1",
  concepts_tracked: 6,
  concepts_known: 2,
  average_freshness: 0.42,
  reviews_total: 3,
  activity: [],
};

function mockFetch() {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    const json = (data: unknown) =>
      new Response(JSON.stringify(data), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    if (url.endsWith("/api/profiles")) return json(profiles);
    if (url.endsWith("/api/courses")) return json(courses);
    if (url.includes("/recommend")) return json(recs);
    if (url.includes("/stats")) return json(stats);
    if (url.endsWith("/api/review")) return json([{ concept_id: "k1" }]);
    return json({});
  });
}

describe("App", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", mockFetch());
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("loads profiles, courses, recommendations and stats", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("Aiko")).toBeInTheDocument());
    expect(screen.getByText("Japanese 103")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("Water card")).toBeInTheDocument());
    expect(screen.getByText("6")).toBeInTheDocument(); // concepts tracked
  });

  it("submits a review when a rating is chosen", async () => {
    const fetchSpy = mockFetch();
    vi.stubGlobal("fetch", fetchSpy);
    render(<App />);
    await waitFor(() => expect(screen.getByText("Good")).toBeInTheDocument());
    fireEvent.click(screen.getByText("Good"));
    await waitFor(() => {
      const called = fetchSpy.mock.calls.some((c) => String(c[0]).endsWith("/api/review"));
      expect(called).toBe(true);
    });
  });

  it("shows an error banner when the API fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("boom", { status: 500 })),
    );
    render(<App />);
    await waitFor(() => expect(screen.getByRole("alert")).toBeInTheDocument());
  });
});
