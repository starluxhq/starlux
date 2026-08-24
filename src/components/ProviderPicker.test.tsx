import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderMenu } from "./ProviderPicker";
import type { Availability, Provider, RateLimit } from "../lib/types";

const NOW = Math.floor(Date.now() / 1000);

const provider = (availability: Availability, over: Partial<Provider> = {}): Provider => ({
  id: "claude-cli",
  name: "Claude Code",
  binary: "claude",
  login: "claude login",
  availability,
  models: ["opus", "sonnet"],
  web: true,
  ...over,
});

const limit = (over: Partial<RateLimit> = {}): RateLimit => ({
  providerId: "claude-cli",
  kind: "five_hour",
  status: "allowed",
  resetsAt: NOW + 3600,
  usingOverage: false,
  observedAt: NOW,
  ...over,
});

// Several cases below are only meaningful as a pair, so each render replaces
// the last rather than stacking beside it.
const menu = (
  availability: Availability,
  limits: Record<string, RateLimit> = {},
  onSelect: (id: string) => void = () => {},
) => {
  cleanup();
  return render(
    <ProviderMenu
      providers={[provider(availability)]}
      providerId="claude-cli"
      limits={limits}
      onSelect={onSelect}
    />,
  );
};

describe("ProviderMenu", () => {
  it("shows the window for a provider that can be run", () => {
    menu({ state: "ready", plan: "max" }, { "claude-cli": limit() });
    expect(screen.getByText(/5-hour resets/)).toBeTruthy();
  });

  // The defect that escaped once: a window the user is no longer inside, shown
  // for an account nothing was run against.
  it("shows no window for a provider that cannot be run", () => {
    for (const availability of [{ state: "signedOut" }, { state: "missing" }] as Availability[]) {
      menu(availability, { "claude-cli": limit() });
      expect(screen.queryByText(/resets/)).toBeNull();
      screen.getByText(/Claude Code/);
    }
  });

  it("drops a window whose reset has already passed", () => {
    menu({ state: "ready", plan: null }, { "claude-cli": limit({ resetsAt: NOW - 60 }) });
    expect(screen.queryByText(/resets/)).toBeNull();
  });

  it("names the plan only when there is one", () => {
    menu({ state: "ready", plan: "max" });
    expect(screen.getByText("max")).toBeTruthy();
    menu({ state: "ready", plan: null });
    expect(screen.queryByText("max")).toBeNull();
  });

  it("ages the window only once the reading is stale", () => {
    menu({ state: "ready", plan: null }, { "claude-cli": limit() });
    expect(screen.queryByText(/ago/)).toBeNull();

    menu({ state: "ready", plan: null }, { "claude-cli": limit({ observedAt: NOW - 30 * 60 }) });
    expect(screen.getByText(/ago/)).toBeTruthy();
  });

  it("leads with the status when the window is not allowing requests", () => {
    menu({ state: "ready", plan: null }, { "claude-cli": limit({ status: "rejected_hard" }) });
    expect(screen.getByText(/rejected hard ·/)).toBeTruthy();
  });

  // Installed-but-signed-out and absent have different fixes, and the command
  // comes from the provider rather than a guess made from its binary name.
  it("names the fix a provider that cannot be run actually needs", () => {
    menu({ state: "signedOut" });
    expect(screen.getByText(/Signed out — run `claude login`/)).toBeTruthy();

    menu({ state: "missing" });
    expect(screen.getByText(/Not found — install `claude`/)).toBeTruthy();
  });

  it("picks a provider that can be run, and refuses one that cannot", () => {
    const onSelect = vi.fn();
    menu({ state: "ready", plan: null }, {}, onSelect);
    fireEvent.click(screen.getByRole("button", { name: /Claude Code/ }));
    expect(onSelect).toHaveBeenCalledWith("claude-cli");

    onSelect.mockClear();
    menu({ state: "signedOut" }, {}, onSelect);
    fireEvent.click(screen.getByRole("button", { name: /Claude Code/ }));
    expect(onSelect).not.toHaveBeenCalled();
  });
});
