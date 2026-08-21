import { useEffect } from "react";
import Answer from "../components/Answer";
import ModelBadge from "../components/ModelBadge";
import Rail from "../components/Rail";
import { useChat } from "../stores/useChat";

export default function Workspace() {
  const { providers, providerId, model, turns, status, loadProviders } = useChat();

  useEffect(() => {
    void loadProviders();
  }, [loadProviders]);

  const provider = providers.find((candidate) => candidate.id === providerId);

  return (
    <div className="flex h-full bg-void text-ink">
      <aside className="flex w-64 shrink-0 flex-col border-r border-white/6 bg-dust/60">
        <div className="px-5 py-4">
          <span className="font-serif text-[19px] tracking-tight">Starlux</span>
        </div>
        <div className="px-5 pb-3">
          <p className="font-mono text-[10px] tracking-wider text-faint uppercase">
            Conversations
          </p>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-2">
          <p className="px-3 py-2 text-[12.5px] text-faint">
            Conversations you start will collect here.
          </p>
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
      </main>
    </div>
  );
}
