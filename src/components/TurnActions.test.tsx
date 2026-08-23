import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import TurnActions from "./TurnActions";

describe("TurnActions", () => {
  it("offers only what the turn it belongs to can do", () => {
    const { unmount } = render(<TurnActions onCopy={vi.fn()} onEdit={vi.fn()} />);
    expect(screen.getByLabelText("Edit")).toBeTruthy();
    expect(screen.queryByLabelText("Retry")).toBeNull();
    unmount();

    render(<TurnActions onCopy={vi.fn()} onRetry={vi.fn()} />);
    expect(screen.getByLabelText("Retry")).toBeTruthy();
    expect(screen.queryByLabelText("Edit")).toBeNull();
  });

  // A run that only errored has nothing to put on the clipboard, but is still
  // the one most worth asking again.
  it("drops copy when there is nothing to copy", () => {
    render(<TurnActions onCopy={null} onRetry={vi.fn()} />);
    expect(screen.queryByLabelText("Copy")).toBeNull();
    expect(screen.getByLabelText("Retry")).toBeTruthy();
  });

  it("says so once it has copied", () => {
    const onCopy = vi.fn();
    render(<TurnActions onCopy={onCopy} />);
    fireEvent.click(screen.getByLabelText("Copy"));
    expect(onCopy).toHaveBeenCalledOnce();
    expect(screen.getByLabelText("Copied")).toBeTruthy();
  });

  it("reports each button to its own handler", () => {
    const onEdit = vi.fn();
    const onRetry = vi.fn();
    render(<TurnActions onCopy={vi.fn()} onEdit={onEdit} onRetry={onRetry} />);
    fireEvent.click(screen.getByLabelText("Edit"));
    fireEvent.click(screen.getByLabelText("Retry"));
    expect(onEdit).toHaveBeenCalledOnce();
    expect(onRetry).toHaveBeenCalledOnce();
  });
});
