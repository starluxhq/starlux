import { useState } from "react";
import { copyText } from "../lib/clipboard";
import { railState } from "../lib/turn";
import type { Status, Turn as ChatTurn } from "../stores/useChat";
import Answer from "./Answer";
import Composer from "./Composer";
import ContextMenu from "./ContextMenu";
import Question from "./Question";
import Rail from "./Rail";
import TurnActions from "./TurnActions";

interface TurnProps {
  turn: ChatTurn;
  status: Status;
  runId: string | null;
  /** The Quick Bar is a bar, and gives a turn less room around it. */
  dense?: boolean;
  onRetry: (id: string) => void;
  onEdit: (id: string, text: string) => void;
}

export default function Turn({ turn, status, runId, dense = false, onRetry, onEdit }: TurnProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; text: string } | null>(null);
  const streaming = turn.id === runId && status === "streaming";
  const spoken = turn.text || turn.error || "";

  // What is selected wins, even where the selection runs past this turn into
  // the ones around it — that is the copy the user marked out. Taken as the
  // menu opens, before clicking an item can disturb it.
  const openMenu = (event: React.MouseEvent) => {
    const selected = window.getSelection()?.toString().trim() ?? "";
    const text = selected || spoken;
    if (text) setMenu({ x: event.clientX, y: event.clientY, text });
  };

  const copyMenu = menu ? (
    <ContextMenu
      x={menu.x}
      y={menu.y}
      onClose={() => setMenu(null)}
      items={[{ label: "Copy", onSelect: () => void copyText(menu.text) }]}
    />
  ) : null;

  if (turn.role === "user") {
    if (draft !== null) {
      return (
        <div
          className="rounded-[10px] border border-rule bg-haze/70 px-3.5 py-2"
          // Caught here so cancelling an edit does not also reach the window's
          // own Escape ladder and dismiss something behind it.
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.stopPropagation();
              setDraft(null);
            }
          }}
        >
          <Composer
            value={draft}
            placeholder="Ask it differently"
            onChange={setDraft}
            onSubmit={() => {
              setDraft(null);
              onEdit(turn.id, draft);
            }}
            maxRows={8}
            marker={false}
          />
        </div>
      );
    }

    return (
      <div className="group" onContextMenu={openMenu}>
        <Question text={turn.text} />
        <div className="flex justify-end">
          <TurnActions onCopy={() => void copyText(turn.text)} onEdit={() => setDraft(turn.text)} />
        </div>
        {copyMenu}
      </div>
    );
  }

  return (
    <article className={`group flex ${dense ? "gap-3" : "gap-4"}`} onContextMenu={openMenu}>
      <Rail status={railState(turn, runId, status)} className={dense ? "mb-1" : "mt-1 mb-1"} />
      <div className="min-w-0 flex-1">
        <Answer turn={turn} />
        {/* Rendered even while the answer is still arriving, so its row is
            already reserved when there is finally something to do with it. */}
        <TurnActions
          onCopy={streaming || !spoken ? null : () => void copyText(spoken)}
          onRetry={streaming ? undefined : () => onRetry(turn.id)}
        />
        {copyMenu}
      </div>
    </article>
  );
}
