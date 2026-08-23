import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import WindowControls from "./WindowControls";

const chrome = vi.hoisted(() => ({
  minimiseWindow: vi.fn(),
  toggleMaximiseWindow: vi.fn(),
  closeWindow: vi.fn(),
  windowIsMaximised: vi.fn(() => Promise.resolve(false)),
  onWindowResized: vi.fn(() => () => {}),
}));

vi.mock("../lib/chrome", () => chrome);

describe("WindowControls", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    chrome.windowIsMaximised.mockResolvedValue(false);
    chrome.onWindowResized.mockReturnValue(() => {});
  });

  it("works the window rather than the page", () => {
    render(<WindowControls />);
    fireEvent.click(screen.getByLabelText("Minimise"));
    fireEvent.click(screen.getByLabelText("Maximise"));
    fireEvent.click(screen.getByLabelText("Close"));

    expect(chrome.minimiseWindow).toHaveBeenCalledOnce();
    expect(chrome.toggleMaximiseWindow).toHaveBeenCalledOnce();
    expect(chrome.closeWindow).toHaveBeenCalledOnce();
  });

  // The window can be maximised without the button — a drag to the top edge, a
  // keyboard shortcut — so the state is read back rather than remembered.
  it("offers to restore a window that is already maximised", async () => {
    chrome.windowIsMaximised.mockResolvedValue(true);
    render(<WindowControls />);
    expect(await screen.findByLabelText("Restore")).toBeTruthy();
  });

  it("stops listening for resizes when it goes away", () => {
    const off = vi.fn();
    chrome.onWindowResized.mockReturnValue(off);
    render(<WindowControls />).unmount();
    expect(off).toHaveBeenCalledOnce();
  });
});
