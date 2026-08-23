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

  // Its row is what keeps a turn from shifting under the pointer the moment it
  // is hovered, so it is there whether or not it has anything in it.
  it("keeps its row when there is nothing to offer at all", () => {
    const { container } = render(<TurnActions onCopy={null} />);
    const row = container.firstChild as HTMLElement;
    expect(row.className).toContain("h-6");
    expect(row.querySelectorAll("button")).toHaveLength(0);
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
