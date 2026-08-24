import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import AgentMode from "./AgentMode";
import type { Tools } from "../lib/types";

const NONE: Tools = { webSearch: false, webFetch: false };
const WEB: Tools = { webSearch: true, webFetch: true };

describe("AgentMode", () => {
  // A hotkey question must never be one click from the filesystem, so the Quick
  // Bar is handed no callbacks at all.
  it("shows the grants without offering to change them", () => {
    render(<AgentMode dir="/home/a/work" tools={WEB} />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("work")).toBeTruthy();
    expect(screen.getByText("Web")).toBeTruthy();
  });

  it("says nothing at all when neither grant is given", () => {
    const { container } = render(<AgentMode dir={null} tools={NONE} />);
    expect(container.textContent).toBe("");
  });

  // The tools are granted in Settings, for the whole app. Where they are shown
  // they are only ever reported; there is no control here to change them.
  it("never offers to change the tools", () => {
    render(<AgentMode dir={null} tools={WEB} onPick={vi.fn()} />);

    expect(screen.getByText("Web")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /web/i })).toBeNull();
    expect(screen.getAllByRole("button")).toHaveLength(1);
  });

  // The Workspace has the settings in its own sidebar, so a badge repeating
  // what that panel says is chrome. It passes no tools and shows none.
  it("says nothing about the tools where it was given none", () => {
    render(<AgentMode dir="/home/a/work" onPick={vi.fn()} />);

    expect(screen.getByText("work")).toBeTruthy();
    expect(screen.queryByText("Web")).toBeNull();
  });

  it("shows the network as reached when only one of the two tools is on", () => {
    render(<AgentMode dir={null} tools={{ webSearch: false, webFetch: true }} />);
    expect(screen.getByText("Web")).toBeTruthy();
  });

  // Two grants, not a ladder: releasing the folder leaves the tools alone.
  it("keeps the folder and the tools apart", () => {
    const onClear = vi.fn();
    render(<AgentMode dir="/home/a/work" tools={WEB} onClear={onClear} />);

    fireEvent.click(screen.getByRole("button", { name: "Chat only" }));
    expect(onClear).toHaveBeenCalled();
    expect(screen.getByText("Web")).toBeTruthy();
  });
});
