import { cleanup, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ModelMenu } from "./ModelPicker";
import type { Availability, Provider, RateLimit } from "../lib/types";

const NOW = Math.floor(Date.now() / 1000);

const provider = (availability: Availability): Provider => ({
  id: "claude-cli",
  name: "Claude Code",
  binary: "claude",
  login: "opencode auth login",
  availability,
  models: ["opus", "sonnet"],
  web: true,
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
const menu = (availability: Availability, limits: Record<string, RateLimit> = {}) => {
  cleanup();
  return render(
    <ModelMenu
      providers={[provider(availability)]}
      providerId="claude-cli"
      model="opus"
      limits={limits}
      onSelect={() => {}}
    />,
  );
};

describe("ModelMenu", () => {
  it("shows the window for a provider that can be run", () => {
    menu({ state: "ready", plan: "max" }, { "claude-cli": limit() });
    expect(screen.getByText(/5-hour resets/)).toBeTruthy();
  });

  // The defect that escaped: a window the user is no longer inside, shown for
  // an account nothing was run against.
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

  it("names the plan in the header only when there is one", () => {
    menu({ state: "ready", plan: "max" });
    expect(screen.getByText(/Claude Code · max/)).toBeTruthy();
    menu({ state: "ready", plan: null });
    expect(screen.queryByText(/Claude Code ·/)).toBeNull();
  });

  it("offers models when ready and a fix when signed out", () => {
    menu({ state: "ready", plan: null });
    expect(screen.getByRole("button", { name: /Opus/ })).toBeTruthy();

    menu({ state: "signedOut" });
    expect(screen.queryByRole("button", { name: /Opus/ })).toBeNull();
    // The command comes from the provider, not from its binary name: `opencode
    // login` is not a command, and a launcher that guesses sends the user
    // somewhere that does not exist.
    expect(screen.getByText(/Signed out — run `opencode auth login`/)).toBeTruthy();
  });

  it("says the provider is missing rather than signed out when it is absent", () => {
    menu({ state: "missing" });
    expect(screen.getByText(/Not found — install `claude`/)).toBeTruthy();
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
});
