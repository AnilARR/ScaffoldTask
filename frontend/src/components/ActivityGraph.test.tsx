import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { ActivityGraph } from "./ActivityGraph";

describe("ActivityGraph", () => {
  it("renders weeks*7 cells", () => {
    render(<ActivityGraph activity={[]} weeks={4} />);
    expect(screen.getAllByTestId("activity-cell")).toHaveLength(28);
  });

  it("marks today's activity with a non-empty level", () => {
    const today = new Date().toISOString().slice(0, 10);
    render(<ActivityGraph activity={[{ date: today, count: 5 }]} weeks={4} />);
    const cells = screen.getAllByTestId("activity-cell");
    const active = cells.filter((c) => !c.className.includes("bg-slate-800"));
    expect(active.length).toBeGreaterThan(0);
  });

  it("shows all-empty grid when no activity", () => {
    render(<ActivityGraph activity={[]} weeks={2} />);
    const cells = screen.getAllByTestId("activity-cell");
    expect(cells.every((c) => c.className.includes("bg-slate-800"))).toBe(true);
  });
});
