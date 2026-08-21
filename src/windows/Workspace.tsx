import { useEffect, useState } from "react";
import Answer from "../components/Answer";
import ArtifactViewer from "../components/ArtifactViewer";
import Composer from "../components/Composer";
import ConversationList from "../components/ConversationList";
import ModelBadge from "../components/ModelBadge";
import Rail from "../components/Rail";
import { onConversationsChanged, onFocusConversation, onStream } from "../lib/events";
import { activeConversation } from "../lib/ipc";
import { useArtifact } from "../stores/useArtifact";
import { applyMirrored, useChat } from "../stores/useChat";
import { useConversations } from "../stores/useConversations";

export default function Workspace() {
  const [draft, setDraft] = useState("");
  const {
    providers,
    providerId,
    model,
    turns,
    status,
    conversationId,
    loadProviders,
    openConversation,
    newConversation,
    send,
    stop,
  } = useChat();
  const { items, load, remove } = useConversations();
  const { expanded, collapse } = useArtifact();

  useEffect(() => {
    void loadProviders();
    void load();
    void activeConversation().then((id) => {
      if (id) void openConversation(id);
    });
  }, [loadProviders, load, openConversation]);

  useEffect(() => onStream(applyMirrored), []);
  useEffect(() => onConversationsChanged(() => void load()), [load]);
  useEffect(
    () =>
      onFocusConversation((id) => {
        if (id) void openConversation(id);
      }),
    [openConversation],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const accel = event.metaKey || event.ctrlKey;
      // An open artifact is the nearest thing to dismiss, so it goes first.
      if (event.key === "Escape" && expanded) collapse();
      else if (event.key === "Escape" && status === "streaming") void stop();
      if (accel && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setDraft("");
        newConversation();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [status, stop, newConversation, expanded, collapse]);

  const provider = providers.find((candidate) => candidate.id === providerId);

  const submit = () => {
    void send(draft);
    setDraft("");
  };

  return (
    <div className="flex h-full bg-void text-ink">
      <aside className="flex w-64 shrink-0 flex-col border-r border-white/6 bg-dust/60">
        <div className="px-5 py-4">
          <span className="font-serif text-[19px] tracking-tight">Starlux</span>
        </div>
        <div className="flex items-baseline justify-between px-5 pb-3">
          <p className="font-mono text-[10px] tracking-wider text-faint uppercase">Conversations</p>
          <button
            type="button"
            onClick={() => {
              setDraft("");
              newConversation();
            }}
            className="font-mono text-[10px] tracking-wider text-muted uppercase hover:text-ink"
          >
            New
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
          <ConversationList
            items={items}
            activeId={conversationId}
            onOpen={(id) => void openConversation(id)}
            onDelete={(id) => {
              if (id === conversationId) newConversation();
              void remove(id);
            }}
          />
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-white/6 px-6 py-3">
          {provider ? (
            <ModelBadge providerId={providerId} name={provider.name} model={model} />
          ) : null}
        </header>

        <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-6">
          {turns.length === 0 ? (
            <p className="max-w-md font-serif text-[22px] leading-snug text-muted">
              Light that left a long time ago, arriving one token at a time.
            </p>
          ) : (
            turns.map((turn) => (
              <article key={turn.id} className="flex gap-4">
                <Rail
                  status={turn.role === "assistant" ? status : "idle"}
                  className="mt-1 mb-1"
                />
                <div className="min-w-0 flex-1">
                  {turn.role === "user" ? (
                    <p className="text-[13.5px] leading-[1.65] text-muted">{turn.text}</p>
                  ) : (
                    <Answer turn={turn} />
                  )}
                </div>
              </article>
            ))
          )}
        </div>

        <div className="shrink-0 border-t border-white/6 px-6 py-4">
          <Composer
            value={draft}
            placeholder={turns.length > 0 ? "Ask a follow-up" : "Ask anything"}
            onChange={setDraft}
            onSubmit={submit}
            maxRows={8}
          />
        </div>
      </main>

      {expanded ? (
        <aside className="flex w-[46%] min-w-0 shrink-0 flex-col border-l border-white/6">
          <ArtifactViewer
            html={expanded.html}
            title={expanded.title}
            variant="pane"
            onCollapse={collapse}
          />
        </aside>
      ) : null}
    </div>
  );
}
