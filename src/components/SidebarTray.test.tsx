import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SidebarTray from "./SidebarTray";

describe("SidebarTray", () => {
  it("opens the settings", () => {
    const onOpenSettings = vi.fn();
    render(<SidebarTray collapsed={false} onOpenSettings={onOpenSettings} />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(onOpenSettings).toHaveBeenCalled();
  });

  // The list closes behind the strip; the way into the settings does not go
  // with it.
  it("keeps the button when the sidebar is collapsed", () => {
    render(<SidebarTray collapsed onOpenSettings={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Settings" })).toBeTruthy();
    expect(screen.queryByText("Settings")).toBeNull();
  });
});
