import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SidebarToolbar from "./SidebarToolbar";

describe("SidebarToolbar", () => {
  it("names both icons, which carry no text of their own", () => {
    render(<SidebarToolbar collapsed={false} onNew={vi.fn()} onToggle={vi.fn()} />);
    expect(screen.getByLabelText("New conversation")).toBeTruthy();
    expect(screen.getByLabelText("Hide conversations")).toBeTruthy();
  });

  // The same button both ways: it survives the collapse, so it has to say
  // which direction it now goes in.
  it("offers to show the list once it is hidden", () => {
    render(<SidebarToolbar collapsed onNew={vi.fn()} onToggle={vi.fn()} />);
    expect(screen.getByLabelText("Show conversations")).toBeTruthy();
    expect(screen.queryByLabelText("Hide conversations")).toBeNull();
  });

  it("keeps offering a new conversation while the list is hidden", () => {
    const onNew = vi.fn();
    render(<SidebarToolbar collapsed onNew={onNew} onToggle={vi.fn()} />);
    fireEvent.click(screen.getByLabelText("New conversation"));
    expect(onNew).toHaveBeenCalledOnce();
  });

  it("reports each button to its own handler", () => {
    const onNew = vi.fn();
    const onToggle = vi.fn();
    render(<SidebarToolbar collapsed={false} onNew={onNew} onToggle={onToggle} />);

    fireEvent.click(screen.getByLabelText("New conversation"));
    expect(onNew).toHaveBeenCalledOnce();
    expect(onToggle).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText("Hide conversations"));
    expect(onToggle).toHaveBeenCalledOnce();
  });
});
