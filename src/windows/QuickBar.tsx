import { open } from "@tauri-apps/plugin-dialog";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import AgentMode from "../components/AgentMode";
import Attachments from "../components/Attachments";
import Composer from "../components/Composer";
import ContextMeter from "../components/ContextMeter";
import { ModelMenu, ModelTrigger } from "../components/ModelPicker";
import ProviderHint from "../components/ProviderHint";
import Turn from "../components/Turn";
import { onAsk, onStream } from "../lib/events";
import { PICKER } from "../lib/models";
import type { Attachment } from "../lib/types";
import {
  hideQuickBar,
  openWorkspace,
  setBlurHideSuppressed,
  setQuickbarHeight,
} from "../lib/ipc";
import { applyMirrored, currentContext, useChat } from "../stores/useChat";

const THREAD_HEIGHT = 450;

export default function QuickBar() {
  const [draft, setDraft] = useState("");
  const [files, setFiles] = useState<Attachment[]>([]);
  const [picking, setPicking] = useState(false);
  const {
    providers,
    limits,
    providerId,
    model,
    agentDir,
    web,
    turns,
    status,
    runId,
    send,
    retry,
    edit,
    stop,
    selectModel,
    loadProviders,
    newConversation,
  } = useChat();
  const context = currentContext(turns);
  const scroller = useRef<HTMLDivElement>(null);
  const shell = useRef<HTMLDivElement>(null);

  const hasThread = turns.length > 0;

  useEffect(() => {
    void loadProviders();
  }, [loadProviders]);

  useEffect(() => {
    scroller.current?.scrollTo({ top: scroller.current.scrollHeight });
  }, [turns]);

  useEffect(() => onStream(applyMirrored), []);

  useEffect(
    () =>
      onAsk((prompt) => {
        setDraft("");
        void send(prompt);
      }),
    [send],
  );

  // The window is the bar until there is an answer to put above it. Measured
  // rather than fixed, so attachments and the open model list get room instead
  // of being clipped by the window edge.
  const asked = useRef(0);
  const fit = useCallback(() => {
    // Always the wrapper, never the constant: the panel carries its own fixed
    // height once there is a thread, so this still picks up the room an open
    // model list needs below it.
    const height = Math.ceil(shell.current?.getBoundingClientRect().height ?? 0);
    if (height > 0 && height !== asked.current) {
      asked.current = height;
      void setQuickbarHeight(height);
    }
  }, []);

  // Every render, because the things that change the bar's height — a file
  // added, the model list opened, the composer growing a line — are all state
  // changes here. The observer is for what is not: web fonts arriving late.
  useLayoutEffect(fit);
  useEffect(() => {
    const node = shell.current;
    if (!node) return;
    const observer = new ResizeObserver(fit);
    observer.observe(node);
    return () => observer.disconnect();
  }, [fit]);

  useEffect(() => {
    if (!picking) return;
    const dismiss = (event: MouseEvent) => {
      if (!(event.target as HTMLElement).closest(`[${PICKER}]`))
        setPicking(false);
    };
    // Capture: the composer holds focus and stops the bubble phase.
    document.addEventListener("mousedown", dismiss, true);
    return () => document.removeEventListener("mousedown", dismiss, true);
  }, [picking]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const accel = event.metaKey || event.ctrlKey;

      if (event.key === "Escape") {
        if (picking) setPicking(false);
        else if (status === "streaming") void stop();
        else void hideQuickBar();
        return;
      }
      if (accel && event.key.toLowerCase() === "e") {
        event.preventDefault();
        void openWorkspace();
      }
      if (accel && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setDraft("");
        setFiles([]);
        newConversation();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [status, stop, newConversation, picking]);

  const submit = () => {
    void send(draft, files);
    setDraft("");
    setFiles([]);
  };

  // A native dialog takes focus, and the bar hides the moment it loses it.
  const attach = useCallback(async () => {
    await setBlurHideSuppressed(true);
    try {
      const picked = await open({ multiple: true, title: "Attach files" });
      const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
      setFiles((current) => {
        const merged = [...current];
        for (const path of paths) {
          if (!merged.some((file) => file.path === path)) {
            merged.push({ path, name: path.split(/[\\/]/).pop() ?? path });
          }
        }
        return merged;
      });
    } finally {
      await setBlurHideSuppressed(false);
    }
  }, []);

  return (
    // The window is measured from this wrapper, which is transparent. Only the
    // panel inside it is the bar, so an open model list grows the window without
    // the bar moving or changing size — the list reads as floating outside the
    // app, which is the one thing a single window cannot actually do.
    <div ref={shell} className="flex h-fit flex-col">
      <div
        className="surface flex flex-col overflow-hidden rounded-[14px] border border-white/8"
        style={hasThread ? { height: THREAD_HEIGHT } : undefined}
      >
        {hasThread ? (
          <div
            ref={scroller}
            className="min-h-0 flex-1 space-y-4 overflow-y-auto border-b border-white/6 px-4 py-4"
          >
            {turns.map((turn) => (
              <Turn
                key={turn.id}
                turn={turn}
                status={status}
                runId={runId}
                dense
                onRetry={(id) => void retry(id)}
                onEdit={(id, text) => void edit(id, text)}
              />
            ))}
          </div>
        ) : null}

        <Attachments
          className="px-3 pt-3"
          items={files}
          onRemove={(path) =>
            setFiles((current) => current.filter((file) => file.path !== path))
          }
        />

        <div className="flex shrink-0 items-center gap-2 px-3 py-2.5">
          <button
            type="button"
            onClick={() => void attach()}
            title="Attach files"
            className="shrink-0 rounded-md px-1.5 text-[19px] leading-none text-faint hover:text-ink"
          >
            +
          </button>

          <Composer
            value={draft}
            placeholder={hasThread ? "Ask a follow-up" : "Ask anything"}
            onChange={setDraft}
            onSubmit={submit}
            maxRows={4}
            marker={false}
          />

          <AgentMode dir={agentDir} web={web} />

          {context ? <ContextMeter context={context} /> : null}

          {model ? (
            <ModelTrigger
              providerId={providerId}
              model={model}
              open={picking}
              onToggle={() => setPicking((was) => !was)}
            />
          ) : (
            <ProviderHint providers={providers} />
          )}

          <button
            type="button"
            onClick={() => void openWorkspace()}
            title="Expand to the Workspace (⌘E)"
            className="shrink-0 rounded-md px-1 py-1 text-muted hover:text-ink"
          >
            <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
              <path
                d="M9.5 2h4.5v4.5M6.5 14H2V9.5M14 2l-5 5M2 14l5-5"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.6"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        </div>
      </div>

      {picking && model ? (
        <ModelMenu
          className="px-3 pt-2"
          providers={providers}
          providerId={providerId}
          model={model}
          limits={limits}
          onSelect={(nextProvider, nextModel) => {
            selectModel(nextProvider, nextModel);
            setPicking(false);
          }}
        />
      ) : null}
    </div>
  );
}
