import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ModelMenu, ModelTrigger } from "./ModelPicker";
import type { Provider } from "../lib/types";

const provider = (ids: string[]): Provider => ({
  id: "opencode-cli",
  name: "opencode",
  binary: "opencode",
  login: "opencode auth login",
  availability: { state: "ready", plan: null },
  models: ids.map((id) => ({ id, efforts: [] })),
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

  // opencode answers with every vendor at once, so which account a model comes
  // from is the first thing to know about it.
  it("gathers the models under the vendor they belong to", () => {
    const { container } = render(
      <ModelMenu
        provider={provider(["opencode/hy3-free", "opencode-go/hy3", "opencode-go/glm-5.3"])}
        model="opencode-go/hy3"
        onSelect={() => {}}
      />,
    );

    expect(container.textContent).toBe("opencodehy3-freeopencode-gohy3glm-5.3");
  });

  // The heading above it already says which vendor, so repeating it on every
  // row would spend the menu's width saying the same thing thirty times.
  it("names a model without the vendor its heading carries", () => {
    const onSelect = vi.fn();
    render(
      <ModelMenu
        provider={provider(["opencode-go/glm-5.3"])}
        model="opencode-go/glm-5.3"
        onSelect={onSelect}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "glm-5.3" }));
    expect(onSelect).toHaveBeenCalledWith("opencode-go/glm-5.3");
  });

  // Claude and Gemini name their models outright, and there is no vendor to
  // head a list of three with.
  it("heads nothing where the ids carry no vendor", () => {
    const { container } = render(
      <ModelMenu provider={provider(["opus", "sonnet"])} model="opus" onSelect={() => {}} />,
    );
    expect(container.textContent).toBe("OpusSonnet");
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
