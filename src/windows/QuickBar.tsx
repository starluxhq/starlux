import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import Answer from "../components/Answer";
import Composer from "../components/Composer";
import Keycap from "../components/Keycap";
import ModelBadge from "../components/ModelBadge";
import Rail from "../components/Rail";
import { hideQuickBar, openWorkspace } from "../lib/ipc";
import { useChat } from "../stores/useChat";

export default function QuickBar() {
  const [draft, setDraft] = useState("");
  const { providers, providerId, model, turns, status, send, stop, loadProviders } = useChat();
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void loadProviders();
  }, [loadProviders]);

  useEffect(() => {
    scroller.current?.scrollTo({ top: scroller.current.scrollHeight });
  }, [turns]);

  useEffect(() => {
    const unlisten = listen<string>("starlux://ask", (event) => {
      setDraft("");
      void send(event.payload);
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, [send]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const accel = event.metaKey || event.ctrlKey;

      if (event.key === "Escape") {
        if (status === "streaming") void stop();
        else void hideQuickBar();
        return;
      }
      if (accel && event.key.toLowerCase() === "e") {
        event.preventDefault();
        void openWorkspace();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [status, stop]);

  const provider = providers.find((candidate) => candidate.id === providerId);
  const answers = turns.filter((turn) => turn.role === "assistant");
  const hasConversation = turns.length > 0;

  const submit = () => {
    void send(draft);
    setDraft("");
  };

  return (
    <div className="surface flex h-full gap-3 overflow-hidden rounded-[14px] border border-white/8 p-px">
      <Rail status={status} className="my-3 ml-2" />

      <div className="flex min-w-0 flex-1 flex-col">
        <div className="px-4 py-3.5">
          <Composer
            value={draft}
            placeholder={hasConversation ? "Ask a follow-up" : "Ask anything"}
            onChange={setDraft}
            onSubmit={submit}
          />
        </div>

        {hasConversation ? (
          <div
            ref={scroller}
            className="min-h-0 flex-1 space-y-5 overflow-y-auto border-t border-white/6 px-4 py-4"
          >
            {answers.map((turn) => (
              <Answer key={turn.id} turn={turn} />
            ))}
          </div>
        ) : null}

        <div className="flex items-center justify-between gap-4 border-t border-white/6 px-4 py-2.5">
          {provider ? (
            <ModelBadge providerId={providerId} name={provider.name} model={model} />
          ) : (
            <span className="font-mono text-[10px] tracking-wide text-class-m uppercase">
              no provider found
            </span>
          )}

          <div className="flex items-center gap-3">
            {status === "streaming" ? (
              <Keycap label="stop">esc</Keycap>
            ) : (
              <Keycap label="send">⏎</Keycap>
            )}
            <Keycap label="expand">⌘E</Keycap>
          </div>
        </div>
      </div>
    </div>
  );
}
