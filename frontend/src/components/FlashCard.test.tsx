import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FlashCard } from "./FlashCard";
import type { ContentItem } from "../types";

const item: ContentItem = {
  id: "i1",
  course_id: "c1",
  kind: "flashcard",
  title: "Water",
  front: "水",
  back: "water (mizu)",
  concept_ids: ["k1"],
  tags: ["language"],
  difficulty: 0.2,
  source_url: null,
};

describe("FlashCard", () => {
  it("shows the front and flips to reveal the back", () => {
    render(<FlashCard item={item} onRate={() => {}} />);
    expect(screen.getByText("水")).toBeInTheDocument();

    const inner = screen.getByTestId("flip-inner");
    expect(inner.style.transform).toBe("rotateY(0deg)");

    fireEvent.click(screen.getByRole("button", { name: /flashcard/i }));
    expect(inner.style.transform).toBe("rotateY(180deg)");
    expect(screen.getByText("water (mizu)")).toBeInTheDocument();
  });

  it("flips with keyboard (Enter)", () => {
    render(<FlashCard item={item} onRate={() => {}} />);
    const card = screen.getByRole("button", { name: /flashcard/i });
    fireEvent.keyDown(card, { key: "Enter" });
    expect(screen.getByTestId("flip-inner").style.transform).toBe("rotateY(180deg)");
  });

  it("fires onRate with the chosen rating", () => {
    const onRate = vi.fn();
    render(<FlashCard item={item} onRate={onRate} />);
    fireEvent.click(screen.getByText("Good"));
    expect(onRate).toHaveBeenCalledWith(3);
  });

  it("disables rating buttons when disabled", () => {
    render(<FlashCard item={item} onRate={() => {}} disabled />);
    expect(screen.getByText("Again")).toBeDisabled();
  });
});
