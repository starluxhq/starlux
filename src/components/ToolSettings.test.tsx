import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ToolSettings from "./ToolSettings";
import type { Availability, Provider, ToolId } from "../lib/types";

const provider = (
  id: string,
  name: string,
  tools: ToolId[],
  availability: Availability = { state: "ready", plan: null },
): Provider => ({ id, name, binary: id, login: `${id} login`, availability, models: [], tools });

const NONE = { webSearch: false, webFetch: false };

describe("ToolSettings", () => {
  it("offers a switch for every tool, off until it is granted", () => {
    render(<ToolSettings tools={NONE} providers={[]} onChange={vi.fn()} />);

    const switches = screen.getAllByRole("switch");
    expect(switches.map((one) => one.getAttribute("aria-label"))).toEqual([
      "Web search",
      "Web fetch",
    ]);
    expect(switches.every((one) => one.getAttribute("aria-checked") === "false")).toBe(true);
  });

  it("grants one tool without touching the other", () => {
    const onChange = vi.fn();
    render(
      <ToolSettings
        tools={{ webSearch: true, webFetch: false }}
        providers={[provider("claude-cli", "Claude Code", ["webSearch", "webFetch"])]}
        onChange={onChange}
      />,
    );

    fireEvent.click(screen.getByRole("switch", { name: "Web fetch" }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("webFetch", true);
    expect(screen.getByRole("switch", { name: "Web search" }).getAttribute("aria-checked")).toBe(
      "true",
    );
  });

  it("gives a granted tool back", () => {
    const onChange = vi.fn();
    render(
      <ToolSettings
        tools={{ webSearch: true, webFetch: true }}
        providers={[provider("claude-cli", "Claude Code", ["webSearch", "webFetch"])]}
        onChange={onChange}
      />,
    );

    fireEvent.click(screen.getByRole("switch", { name: "Web search" }));
    expect(onChange).toHaveBeenCalledWith("webSearch", false);
  });

  // A switch with nothing behind it is worse than no switch: opencode has no
  // search tool, so a run granted one would reach exactly nothing.
  it("will not grant a tool no signed-in provider offers", () => {
    const onChange = vi.fn();
    render(
      <ToolSettings
        tools={NONE}
        providers={[provider("opencode-cli", "opencode", ["webFetch"])]}
        onChange={onChange}
      />,
    );

    const search = screen.getByRole("switch", { name: "Web search" });
    expect(search.hasAttribute("disabled")).toBe(true);
    fireEvent.click(search);
    expect(onChange).not.toHaveBeenCalled();

    expect(screen.getByRole("switch", { name: "Web fetch" }).hasAttribute("disabled")).toBe(false);
  });

  // Signing out of the only provider that offered it must not strand the grant:
  // what stands can always be given back.
  it("still gives back a grant nothing is left to spend", () => {
    const onChange = vi.fn();
    render(
      <ToolSettings
        tools={{ webSearch: true, webFetch: false }}
        providers={[provider("opencode-cli", "opencode", ["webFetch"])]}
        onChange={onChange}
      />,
    );

    const search = screen.getByRole("switch", { name: "Web search" });
    expect(search.hasAttribute("disabled")).toBe(false);
    fireEvent.click(search);
    expect(onChange).toHaveBeenCalledWith("webSearch", false);
  });

  it("names the providers that would actually run each tool", () => {
    render(
      <ToolSettings
        tools={NONE}
        providers={[
          provider("claude-cli", "Claude Code", ["webSearch", "webFetch"]),
          provider("opencode-cli", "opencode", ["webFetch"]),
        ]}
        onChange={vi.fn()}
      />,
    );

    expect(screen.getByText("Claude Code")).toBeTruthy();
    expect(screen.getByText("Claude Code · opencode")).toBeTruthy();
  });

  // Installed is not signed in, and a provider nobody is signed in to cannot
  // run the tool it advertises.
  it("does not count a signed-out provider as offering anything", () => {
    render(
      <ToolSettings
        tools={NONE}
        providers={[
          provider("claude-cli", "Claude Code", ["webSearch", "webFetch"], { state: "signedOut" }),
        ]}
        onChange={vi.fn()}
      />,
    );

    expect(screen.getAllByText("No signed-in provider offers this")).toHaveLength(2);
  });
});
