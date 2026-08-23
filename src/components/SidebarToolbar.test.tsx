import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SidebarToolbar from "./SidebarToolbar";

describe("SidebarToolbar", () => {
  it("names both icons, which carry no text of their own", () => {
    render(<SidebarToolbar onNew={vi.fn()} onCollapse={vi.fn()} />);
    expect(screen.getByLabelText("New conversation")).toBeTruthy();
    expect(screen.getByLabelText("Hide conversations")).toBeTruthy();
  });

  it("reports each button to its own handler", () => {
    const onNew = vi.fn();
    const onCollapse = vi.fn();
    render(<SidebarToolbar onNew={onNew} onCollapse={onCollapse} />);

    fireEvent.click(screen.getByLabelText("New conversation"));
    expect(onNew).toHaveBeenCalledOnce();
    expect(onCollapse).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText("Hide conversations"));
    expect(onCollapse).toHaveBeenCalledOnce();
  });
});
