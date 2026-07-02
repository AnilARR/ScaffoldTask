import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProfileSwitcher } from "./ProfileSwitcher";
import type { Profile } from "../types";

const profiles: Profile[] = [
  { id: "a", name: "Aiko", level: "beginner", interests: [], created_at: "" },
  { id: "b", name: "Ben", level: "intermediate", interests: [], created_at: "" },
];

describe("ProfileSwitcher", () => {
  it("renders all profiles and marks the selected one", () => {
    render(<ProfileSwitcher profiles={profiles} selectedId="b" onSelect={() => {}} />);
    expect(screen.getByText("Aiko")).toBeInTheDocument();
    expect(screen.getByText("Ben").closest("button")).toHaveAttribute("aria-pressed", "true");
  });

  it("calls onSelect when a profile is clicked", () => {
    const onSelect = vi.fn();
    render(<ProfileSwitcher profiles={profiles} onSelect={onSelect} />);
    fireEvent.click(screen.getByText("Aiko"));
    expect(onSelect).toHaveBeenCalledWith("a");
  });
});
