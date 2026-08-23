import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ContextMenu from "./ContextMenu";

const items = (onSelect = vi.fn()) => [{ label: "Rename", onSelect }];

describe("ContextMenu", () => {
  it("runs the item picked and then gets out of the way", () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(<ContextMenu x={10} y={10} items={items(onSelect)} onClose={onClose} />);

    fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("closes on Escape and on a click outside it", () => {
    const onClose = vi.fn();
    render(<ContextMenu x={10} y={10} items={items()} onClose={onClose} />);

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();

    fireEvent.mouseDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("stays open when the click lands inside it", () => {
    const onClose = vi.fn();
    render(<ContextMenu x={10} y={10} items={items()} onClose={onClose} />);

    fireEvent.mouseDown(screen.getByRole("menuitem", { name: "Rename" }));
    expect(onClose).not.toHaveBeenCalled();
  });
});
