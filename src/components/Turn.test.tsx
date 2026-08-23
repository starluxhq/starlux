import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Turn from "./Turn";
import type { Turn as ChatTurn } from "../stores/useChat";

const clipboard = vi.hoisted(() => ({ copyText: vi.fn(), pasteText: vi.fn() }));
vi.mock("../lib/clipboard", () => clipboard);

const question: ChatTurn = { id: "run-1:u", role: "user", text: "what is a spectral class?" };

const turn = (props: Partial<Parameters<typeof Turn>[0]> = {}) =>
  render(
    <Turn
      turn={question}
      status="idle"
      runId={null}
      onRetry={vi.fn()}
      onEdit={vi.fn()}
      {...props}
    />,
  );

const select = (text: string) =>
  vi.spyOn(window, "getSelection").mockReturnValue({ toString: () => text } as Selection);

describe("Turn", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("offers copy alone on a right-click", () => {
    turn();
    fireEvent.contextMenu(screen.getByText(question.text));
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Copy"]);
  });

  it("copies the message that was right-clicked when nothing is selected", () => {
    select("   ");
    turn();
    fireEvent.contextMenu(screen.getByText(question.text));
    fireEvent.click(screen.getByRole("menuitem", { name: "Copy" }));
    expect(clipboard.copyText).toHaveBeenCalledWith(question.text);
  });

  // The selection can run past this turn into the ones around it, and that is
  // still the copy the user marked out.
  it("copies the selection when there is one", () => {
    select("read off the absorption lines");
    turn();
    fireEvent.contextMenu(screen.getByText(question.text));
    fireEvent.click(screen.getByRole("menuitem", { name: "Copy" }));
    expect(clipboard.copyText).toHaveBeenCalledWith("read off the absorption lines");
  });

  it("offers retry on an answer, and nothing while one is still arriving", () => {
    const answer: ChatTurn = { id: "run-1", role: "assistant", text: "surface temperature" };
    const { unmount } = turn({ turn: answer });
    expect(screen.getByLabelText("Retry")).toBeTruthy();
    expect(screen.getByLabelText("Copy")).toBeTruthy();
    unmount();

    turn({ turn: answer, status: "streaming", runId: "run-1" });
    expect(screen.queryByLabelText("Retry")).toBeNull();
    expect(screen.queryByLabelText("Copy")).toBeNull();
  });

  // Selection is off for the app as a whole, so what was said has to ask for
  // it back — and it is the only thing a copy is ever taken from.
  it("leaves what was said selectable", () => {
    const { unmount } = turn();
    expect(screen.getByText(question.text).className).toContain("select-text");
    unmount();

    const answer: ChatTurn = { id: "run-1", role: "assistant", text: "surface temperature" };
    const { container } = turn({ turn: answer });
    expect(container.querySelector(".prose-starlux")?.className).toContain("select-text");
  });

  it("edits in place and reports the rewritten question", () => {
    const onEdit = vi.fn();
    turn({ onEdit });
    fireEvent.click(screen.getByLabelText("Edit"));

    const field = screen.getByRole("textbox");
    fireEvent.change(field, { target: { value: "ask it differently" } });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(onEdit).toHaveBeenCalledWith("run-1:u", "ask it differently");
  });
});
