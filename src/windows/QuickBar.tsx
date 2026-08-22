import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import AgentMode from "../components/AgentMode";
import Answer from "../components/Answer";
import Attachments, { type Attachment } from "../components/Attachments";
import Composer from "../components/Composer";
import { ModelMenu, ModelTrigger } from "../components/ModelPicker";
import Rail from "../components/Rail";
import { onAsk, onStream } from "../lib/events";
import { PICKER } from "../lib/models";
import {
  hideQuickBar,
  openWorkspace,
  setBlurHideSuppressed,
  setQuickbarHeight,
} from "../lib/ipc";
import { applyMirrored, useChat } from "../stores/useChat";

const THREAD_HEIGHT = 450;

export default function QuickBar() {
  const [draft, setDraft] = useState("");
  const [files, setFiles] = useState<Attachment[]>([]);
  const [picking, setPicking] = useState(false);
  const {
    providers,
    providerId,
    model,
    agentDir,
    turns,
    status,
    send,
    stop,
    selectModel,
    loadProviders,
    newConversation,
  } = useChat();
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
    const height = hasThread ? THREAD_HEIGHT : Math.ceil(shell.current?.getBoundingClientRect().height ?? 0);
    if (height > 0 && height !== asked.current) {
      asked.current = height;
      void setQuickbarHeight(height);
    }
  }, [hasThread]);

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
      if (!(event.target as HTMLElement).closest(`[${PICKER}]`)) setPicking(false);
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

  const available = providers.filter((provider) => provider.available);
  const answers = turns.filter((turn) => turn.role === "assistant");

  const submit = () => {
    void send(draft);
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
    <div
      ref={shell}
      // Content-height while it is only a bar, so the window can be measured
      // from it; full-height once the window is fixed, so no transparent strip
      // is left over the desktop still catching clicks. Deliberately not capped
      // to the viewport: that would clamp the shell to a window sized from the
      // shell, and the bar could shrink but never grow back.
      className={`surface flex flex-col overflow-hidden rounded-[14px] border border-white/8 ${
        hasThread ? "h-full" : "h-fit"
      }`}
    >
      {hasThread ? (
        <div
          ref={scroller}
          className="flex min-h-0 flex-1 gap-3 overflow-y-auto border-b border-white/6 px-4 py-4"
        >
          <Rail status={status} className="mb-1" />
          <div className="min-w-0 flex-1 space-y-5">
            {answers.map((turn) => (
              <Answer key={turn.id} turn={turn} />
            ))}
          </div>
        </div>
      ) : null}

      {picking ? (
        <ModelMenu
          providers={available}
          providerId={providerId}
          model={model}
          onSelect={(nextProvider, nextModel) => {
            selectModel(nextProvider, nextModel);
            setPicking(false);
          }}
        />
      ) : null}

      <Attachments
        items={files}
        onRemove={(path) => setFiles((current) => current.filter((file) => file.path !== path))}
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

        <AgentMode dir={agentDir} />

        {available.length > 0 ? (
          <ModelTrigger
            providerId={providerId}
            model={model}
            open={picking}
            onToggle={() => setPicking((was) => !was)}
          />
        ) : (
          <span className="shrink-0 font-mono text-[10px] whitespace-nowrap text-class-m uppercase">
            no provider
          </span>
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
  );
}
