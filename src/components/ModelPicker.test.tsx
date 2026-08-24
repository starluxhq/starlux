import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ModelMenu, ModelTrigger } from "./ModelPicker";
import type { Provider } from "../lib/types";

const provider = (models: string[]): Provider => ({
  id: "opencode-cli",
  name: "opencode",
  binary: "opencode",
  login: "opencode auth login",
  availability: { state: "ready", plan: null },
  models,
  tools: ["webSearch", "webFetch"],
});

describe("ModelMenu", () => {
  it("offers the chosen provider's models and nobody else's", () => {
    const onSelect = vi.fn();
    render(
      <ModelMenu provider={provider(["opus", "sonnet"])} model="opus" onSelect={onSelect} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Sonnet" }));
    expect(onSelect).toHaveBeenCalledWith("sonnet");
  });

  // A vendor-prefixed id is listed in full: with 29 of them under one provider,
  // the prefix is what tells two apart.
  it("lists a vendor-prefixed model by its whole id", () => {
    render(
      <ModelMenu
        provider={provider(["opencode/hy3-free", "opencode-go/hy3"])}
        model="opencode-go/hy3"
        onSelect={() => {}}
      />,
    );
    expect(screen.getByRole("button", { name: "opencode/hy3-free" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "opencode-go/hy3" })).toBeTruthy();
  });

  it("says nothing at all when there is no provider to speak for", () => {
    const { container } = render(
      <ModelMenu provider={undefined} model="opus" onSelect={() => {}} />,
    );
    expect(container.textContent).toBe("");
  });
});

describe("ModelTrigger", () => {
  // The provider is named right beside it, so repeating the vendor here would
  // spend the bar's width saying the same thing twice.
  it("drops the vendor the provider trigger already names", () => {
    render(<ModelTrigger model="opencode-go/glm-5.3" open={false} onToggle={() => {}} />);
    expect(screen.getByRole("button").textContent).toBe("glm-5.3");
  });

  it("capitalises a bare alias", () => {
    render(<ModelTrigger model="opus" open={false} onToggle={() => {}} />);
    expect(screen.getByRole("button").textContent).toBe("Opus");
  });
});
