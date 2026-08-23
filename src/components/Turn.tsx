import { useState } from "react";
import { copyText } from "../lib/clipboard";
import { railState } from "../lib/turn";
import type { Status, Turn as ChatTurn } from "../stores/useChat";
import Answer from "./Answer";
import Composer from "./Composer";
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
  const streaming = turn.id === runId && status === "streaming";

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
      <div className="group">
        <Question text={turn.text} />
        <div className="flex justify-end">
          <TurnActions onCopy={() => void copyText(turn.text)} onEdit={() => setDraft(turn.text)} />
        </div>
      </div>
    );
  }

  return (
    <article className={`group flex ${dense ? "gap-3" : "gap-4"}`}>
      <Rail status={railState(turn, runId, status)} className={dense ? "mb-1" : "mt-1 mb-1"} />
      <div className="min-w-0 flex-1">
        <Answer turn={turn} />
        {streaming ? null : (
          <TurnActions
            onCopy={turn.text ? () => void copyText(turn.text) : null}
            onRetry={() => onRetry(turn.id)}
          />
        )}
      </div>
    </article>
  );
}
