import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ProviderHint from "./ProviderHint";
import type { Availability, Provider } from "../lib/types";

const provider = (id: string, binary: string, availability: Availability): Provider => ({
  id,
  name: id,
  binary,
  login: `${binary} auth login`,
  availability,
  models: [],
  web: true,
});

describe("ProviderHint", () => {
  it("names signing in as the fix when a provider is installed", () => {
    render(<ProviderHint providers={[provider("claude-cli", "claude", { state: "signedOut" })]} />);
    const hint = screen.getByText("signed out");
    expect(hint.title).toContain("claude auth login");
  });

  it("reports nothing installed when no provider is present", () => {
    render(<ProviderHint providers={[provider("claude-cli", "claude", { state: "missing" })]} />);
    expect(screen.getByText("no provider").title).toContain("PATH");
  });

  it("says nothing was found when there are no providers at all", () => {
    render(<ProviderHint providers={[]} />);
    expect(screen.getByText("no provider")).toBeTruthy();
  });

  // Signing in is one command; installing is not. Whichever is on the machine
  // decides which of the two the user is told to do.
  it("prefers the signed-out provider over an absent one", () => {
    render(
      <ProviderHint
        providers={[
          provider("gemini-cli", "gemini", { state: "missing" }),
          provider("claude-cli", "claude", { state: "signedOut" }),
        ]}
      />,
    );
    expect(screen.getByText("signed out").title).toContain("claude auth login");
  });
});
