import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import AgentMode from "./AgentMode";

describe("AgentMode", () => {
  // A hotkey question must never be one click from either grant, so the Quick
  // Bar is handed no callbacks at all.
  it("shows the grants without offering to change them", () => {
    render(<AgentMode dir="/home/a/work" web />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("work")).toBeTruthy();
    expect(screen.getByText("Web")).toBeTruthy();
  });

  it("says nothing at all when neither grant is given", () => {
    const { container } = render(<AgentMode dir={null} web={false} />);
    expect(container.textContent).toBe("");
  });

  it("toggles the web grant on its own", () => {
    const onWeb = vi.fn();
    const onPick = vi.fn();
    render(<AgentMode dir={null} web={false} onPick={onPick} onWeb={onWeb} />);

    fireEvent.click(screen.getByRole("button", { name: /web/i }));
    expect(onWeb).toHaveBeenCalledWith(true);
    expect(onPick).not.toHaveBeenCalled();
  });

  // Two grants, not a ladder: releasing the folder leaves the web grant alone.
  it("keeps the folder and the web grant apart", () => {
    const onClear = vi.fn();
    const onWeb = vi.fn();
    render(<AgentMode dir="/home/a/work" web onClear={onClear} onWeb={onWeb} />);

    fireEvent.click(screen.getByRole("button", { name: "Chat only" }));
    expect(onClear).toHaveBeenCalled();
    expect(onWeb).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /web/i }).getAttribute("aria-pressed")).toBe("true");
  });
});
