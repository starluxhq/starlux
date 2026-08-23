import { Streamdown } from "streamdown";
import { markdown } from "../lib/markdown/plugins";
import { TurnContext } from "../lib/turn";
import type { Turn } from "../stores/useChat";

export default function Answer({ turn }: { turn: Turn }) {
  if (turn.error) {
    return (
      <div className="space-y-2 select-text">
        <p className="text-[13px] text-class-m">{turn.error}</p>
        {turn.stderrTail ? (
          <pre className="overflow-x-auto rounded-md border border-rule bg-void/60 p-3 font-mono text-[11px] leading-relaxed text-faint">
            {turn.stderrTail}
          </pre>
        ) : null}
      </div>
    );
  }

  return (
    <div className="prose-starlux text-[13.5px] leading-[1.65] text-ink/90 select-text">
      <TurnContext.Provider value={turn.id}>
        <Streamdown parseIncompleteMarkdown plugins={markdown}>
          {turn.text}
        </Streamdown>
      </TurnContext.Provider>
    </div>
  );
}
