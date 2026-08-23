import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ConversationList from "./ConversationList";
import type { Conversation } from "../lib/types";

const item = (over: Partial<Conversation> = {}): Conversation => ({
  id: "c1",
  title: "Spectral classes",
  providerId: "claude-cli",
  sessionId: null,
  model: null,
  agentDir: null,
  web: false,
  updatedAt: Date.now(),
  pinned: false,
  ...over,
});

const list = (props: Partial<Parameters<typeof ConversationList>[0]> = {}) =>
  render(
    <ConversationList
      items={[item()]}
      activeId={null}
      onOpen={vi.fn()}
      onRename={vi.fn()}
      onPin={vi.fn()}
      onDelete={vi.fn()}
      {...props}
    />,
  );

describe("ConversationList", () => {
  it("offers the same three actions from the button and from a right-click", () => {
    list();
    fireEvent.click(screen.getByLabelText("Actions for Spectral classes"));
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
      "Pin",
      "Rename",
      "Delete",
    ]);

    fireEvent.keyDown(document, { key: "Escape" });
    fireEvent.contextMenu(screen.getByText("Spectral classes"));
    expect(screen.getAllByRole("menuitem")).toHaveLength(3);
  });

  it("offers to unpin one that is already pinned", () => {
    const onPin = vi.fn();
    list({ items: [item({ pinned: true })], onPin });
    fireEvent.click(screen.getByLabelText("Actions for Spectral classes"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Unpin" }));
    expect(onPin).toHaveBeenCalledWith("c1", false);
  });

  it("deletes only the conversation the menu was opened on", () => {
    const onDelete = vi.fn();
    list({ onDelete });
    fireEvent.click(screen.getByLabelText("Actions for Spectral classes"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    expect(onDelete).toHaveBeenCalledWith("c1");
  });

  it("renames in place, and reports nothing when the title is unchanged", () => {
    const onRename = vi.fn();
    list({ onRename });
    fireEvent.click(screen.getByLabelText("Actions for Spectral classes"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));

    const field = screen.getByLabelText("Rename Spectral classes") as HTMLInputElement;
    fireEvent.keyDown(field, { key: "Enter" });
    expect(onRename).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText("Actions for Spectral classes"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
    const again = screen.getByLabelText("Rename Spectral classes");
    fireEvent.change(again, { target: { value: "  Absorption lines  " } });
    fireEvent.keyDown(again, { key: "Enter" });
    expect(onRename).toHaveBeenCalledWith("c1", "Absorption lines");
  });

  it("cancels a rename on Escape", () => {
    const onRename = vi.fn();
    list({ onRename });
    fireEvent.click(screen.getByLabelText("Actions for Spectral classes"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));

    const field = screen.getByLabelText("Rename Spectral classes");
    fireEvent.change(field, { target: { value: "Something else" } });
    fireEvent.keyDown(field, { key: "Escape" });
    expect(onRename).not.toHaveBeenCalled();
  });
});
