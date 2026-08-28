import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EffortMenu, EffortTrigger } from "./EffortPicker";

describe("EffortTrigger", () => {
  // Most models offer no choice at all, and a control that cannot change
  // anything is worse than no control: it implies the level was chosen.
  it("says nothing where the model offers no levels", () => {
    const { container } = render(
      <EffortTrigger efforts={[]} effort={null} open={false} onToggle={() => {}} />,
    );
    expect(container.textContent).toBe("");
  });

  it("reads as auto until a level is chosen", () => {
    const { rerender } = render(
      <EffortTrigger efforts={["low", "high"]} effort={null} open={false} onToggle={() => {}} />,
    );
    expect(screen.getByRole("button").textContent).toBe("auto");

    rerender(
      <EffortTrigger efforts={["low", "high"]} effort="high" open={false} onToggle={() => {}} />,
    );
    expect(screen.getByRole("button").textContent).toBe("high");
  });
});

describe("EffortMenu", () => {
  it("offers the model's own levels in the order they were ranked", () => {
    const { container } = render(
      <EffortMenu efforts={["low", "high", "max"]} effort={null} onSelect={() => {}} />,
    );
    expect(container.textContent).toBe("autolowhighmax");
  });

  // Not a rung: it sends no flag, which is the only way to ask for whatever the
  // provider would have done anyway.
  it("gives back nothing at all when auto is chosen", () => {
    const onSelect = vi.fn();
    render(<EffortMenu efforts={["low", "high"]} effort="high" onSelect={onSelect} />);

    fireEvent.click(screen.getByRole("button", { name: "auto" }));
    expect(onSelect).toHaveBeenCalledWith(null);
  });

  it("gives back the level that was clicked", () => {
    const onSelect = vi.fn();
    render(<EffortMenu efforts={["low", "high"]} effort={null} onSelect={onSelect} />);

    fireEvent.click(screen.getByRole("button", { name: "high" }));
    expect(onSelect).toHaveBeenCalledWith("high");
  });

  // `minimax-m3` offers `none` and `thinking`; neither is a rung on the usual
  // ladder, and the menu must not assume one.
  it("shows a pair of levels that are not a ladder", () => {
    const { container } = render(
      <EffortMenu efforts={["none", "thinking"]} effort={null} onSelect={() => {}} />,
    );
    expect(container.textContent).toBe("autononethinking");
  });

  it("says nothing where the model offers no levels", () => {
    const { container } = render(<EffortMenu efforts={[]} effort={null} onSelect={() => {}} />);
    expect(container.textContent).toBe("");
  });
});
