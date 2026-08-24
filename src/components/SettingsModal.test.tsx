import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SettingsModal from "./SettingsModal";
import { useChat } from "../stores/useChat";
import { useSettings } from "../stores/useSettings";

const ipc = vi.hoisted(() => ({
  tools: vi.fn(() => Promise.resolve({ webSearch: true, webFetch: false })),
  setTool: vi.fn(() => Promise.resolve({ webSearch: true, webFetch: true })),
}));

vi.mock("../lib/ipc", () => ipc);

beforeEach(() => {
  vi.clearAllMocks();
  useSettings.setState({ tools: { webSearch: false, webFetch: false } });
  useChat.setState({
    providers: [
      {
        id: "claude-cli",
        name: "Claude Code",
        binary: "claude",
        login: "claude login",
        availability: { state: "ready", plan: null },
        models: ["opus"],
        tools: ["webSearch", "webFetch"],
      },
    ],
  });
});

describe("SettingsModal", () => {
  it("opens on Tools and shows what stands", async () => {
    render(<SettingsModal onClose={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Tools" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Tools" }).getAttribute("aria-current")).toBe("true");
    // Read from the core on open rather than trusted from the last time this
    // window looked: the other one may have changed it since.
    expect(
      await screen.findByRole("switch", { name: "Web search", checked: true }),
    ).toBeTruthy();
  });

  it("grants a tool", async () => {
    render(<SettingsModal onClose={vi.fn()} />);
    await screen.findByRole("switch", { name: "Web search", checked: true });

    fireEvent.click(screen.getByRole("switch", { name: "Web fetch" }));
    expect(ipc.setTool).toHaveBeenCalledWith("webFetch", true);
  });

  it("closes from the button", () => {
    const onClose = vi.fn();
    render(<SettingsModal onClose={onClose} />);

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // Clicking off the panel closes it, and the way it does is the platform's:
  // a submit inside a `method="dialog"` form.
  it("closes when the backdrop is clicked", () => {
    const onClose = vi.fn();
    const { container } = render(<SettingsModal onClose={onClose} />);

    const backdrop = container.querySelector('form[method="dialog"] button');
    expect(backdrop).toBeTruthy();
    expect(backdrop!.getAttribute("type")).toBe("submit");
    expect(backdrop!.getAttribute("tabindex")).toBe("-1");
  });

  // The element can close itself, so what tells the Workspace has to be the
  // close event rather than the click that happened to cause it.
  it("tells the Workspace when Escape closes it", () => {
    const onClose = vi.fn();
    render(<SettingsModal onClose={onClose} />);

    (screen.getByRole("dialog") as HTMLDialogElement).close();
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
