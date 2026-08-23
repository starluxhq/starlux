import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import AgentMode from "../components/AgentMode";
import Answer from "../components/Answer";
import ArtifactViewer from "../components/ArtifactViewer";
import Attachments, { type Attachment } from "../components/Attachments";
import Composer from "../components/Composer";
import ConversationList from "../components/ConversationList";
import ContextMeter from "../components/ContextMeter";
import { ModelMenu, ModelTrigger } from "../components/ModelPicker";
import ProviderHint from "../components/ProviderHint";
import Question from "../components/Question";
import Rail from "../components/Rail";
import SidebarToolbar from "../components/SidebarToolbar";
import WindowControls from "../components/WindowControls";
import { PICKER } from "../lib/models";
import { platform } from "../lib/platform";
import { onConversationsChanged, onFocusConversation, onStream } from "../lib/events";
import { activeConversation, saveSidebarCollapsed, sidebarCollapsed } from "../lib/ipc";
import { railState } from "../lib/turn";
import { useArtifact } from "../stores/useArtifact";
import { applyMirrored, currentContext, useChat } from "../stores/useChat";
import { useConversations } from "../stores/useConversations";

export default function Workspace() {
  const [draft, setDraft] = useState("");
  const [files, setFiles] = useState<Attachment[]>([]);
  const [picking, setPicking] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  // The stored state has to land without sliding: restoring a hidden sidebar
  // is not a move the user made, so only their own toggles are animated.
  const [animate, setAnimate] = useState(false);
  const {
    providers,
    limits,
    providerId,
    model,
    turns,
    status,
    runId,
    conversationId,
    agentDir,
    loadProviders,
    openConversation,
    newConversation,
    selectModel,
    setAgentDir,
    send,
    stop,
  } = useChat();
  const context = currentContext(turns);
  const { items, load, rename, pin, remove } = useConversations();
  const { expanded, collapse } = useArtifact();

  useEffect(() => {
    void loadProviders();
    void load();
    void activeConversation().then((id) => {
      if (id) void openConversation(id);
    });
    void sidebarCollapsed().then(setCollapsed);
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
    if (!picking) return;
    const dismiss = (event: MouseEvent) => {
      if (!(event.target as HTMLElement).closest(`[${PICKER}]`)) setPicking(false);
    };
    document.addEventListener("mousedown", dismiss, true);
    return () => document.removeEventListener("mousedown", dismiss, true);
  }, [picking]);

  const startConversation = () => {
    setDraft("");
    newConversation();
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const accel = event.metaKey || event.ctrlKey;
      // An open artifact is the nearest thing to dismiss, so it goes first.
      if (event.key === "Escape" && picking) setPicking(false);
      else if (event.key === "Escape" && expanded) collapse();
      else if (event.key === "Escape" && status === "streaming") void stop();
      if (accel && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setDraft("");
        newConversation();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [status, stop, newConversation, expanded, collapse, picking]);


  const showSidebar = (shown: boolean) => {
    setAnimate(true);
    setCollapsed(!shown);
    void saveSidebarCollapsed(!shown);
  };

  const submit = () => {
    void send(draft);
    setDraft("");
    setFiles([]);
  };

  const attach = async () => {
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
  };

  const pickFolder = async () => {
    const picked = await open({
      directory: true,
      title: "Choose a folder this conversation may work in",
    });
    if (typeof picked === "string") await setAgentDir(picked);
  };

  return (
    <div className="flex h-full flex-col bg-void text-ink">
      {/* The window has no toolkit chrome, so this strip is both the handle it
          is moved by and the corner its controls sit in. Indented on macOS,
          where the traffic lights are drawn over the page's own top-left. */}
      <div
        data-tauri-drag-region
        className={`flex h-10 shrink-0 items-center gap-4 border-b border-white/6 pr-2 ${
          platform === "macos" ? "pl-[82px]" : "pl-5"
        }`}
      >
        {collapsed ? (
          <button
            type="button"
            onClick={() => showSidebar(true)}
            aria-label="Show conversations"
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted hover:bg-white/6 hover:text-ink"
          >
            <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
              <path
                d="M6.5 4l4 4-4 4"
                fill="none"
                stroke="currentColor"
                strokeWidth={1.4}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        ) : null}

        <span data-tauri-drag-region className="font-serif text-[15px] tracking-tight">
          Starlux
        </span>

        <div className="ml-auto flex min-w-0 items-center gap-4">
          <AgentMode
            dir={agentDir}
            onPick={() => void pickFolder()}
            onClear={() => void setAgentDir(null)}
          />
          {platform === "macos" ? null : <WindowControls />}
        </div>
      </div>

      <div className="flex min-h-0 flex-1">
        {/* The list slides away rather than squashing: a fixed-width column
            inside a wrapper that is all that changes width. */}
        <aside
          className={`shrink-0 overflow-hidden border-r border-white/6 bg-dust/60 ${
            animate ? "transition-[width] duration-200 ease-out motion-reduce:transition-none" : ""
          } ${collapsed ? "w-0 border-r-0" : "w-64"}`}
        >
          <div className="flex h-full w-64 flex-col">
            <SidebarToolbar onNew={startConversation} onCollapse={() => showSidebar(false)} />
            <p className="px-5 pt-3 pb-3 font-mono text-[10px] tracking-wider text-faint uppercase">
              Conversations
            </p>
            <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
              <ConversationList
                items={items}
                activeId={conversationId}
                onOpen={(id) => void openConversation(id)}
                onRename={(id, title) => void rename(id, title)}
                onPin={(id, pinned) => void pin(id, pinned)}
                onDelete={(id) => {
                  if (id === conversationId) newConversation();
                  void remove(id);
                }}
              />
            </div>
          </div>
        </aside>

        <main className="flex min-w-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-6">
            {turns.length === 0 ? (
              <p className="max-w-md font-serif text-[22px] leading-snug text-muted">
                Light that left a long time ago, arriving one token at a time.
              </p>
            ) : (
              turns.map((turn) =>
                turn.role === "user" ? (
                  <Question key={turn.id} text={turn.text} />
                ) : (
                  <article key={turn.id} className="flex gap-4">
                    <Rail status={railState(turn, runId, status)} className="mt-1 mb-1" />
                    <div className="min-w-0 flex-1">
                      <Answer turn={turn} />
                    </div>
                  </article>
                ),
              )
            )}
          </div>

          <div className="relative shrink-0 border-t border-white/6 px-6 py-4">
            {picking && model ? (
              <ModelMenu
                className="absolute right-6 bottom-full mb-2"
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

            <Attachments
              className="pb-3"
              items={files}
              onRemove={(path) => setFiles((current) => current.filter((file) => file.path !== path))}
            />

            <div className="flex items-end gap-3">
              <button
                type="button"
                onClick={() => void attach()}
                title="Attach files"
                className="shrink-0 rounded-md px-1 pb-1 text-[19px] leading-none text-faint hover:text-ink"
              >
                +
              </button>

              <Composer
                value={draft}
                placeholder={turns.length > 0 ? "Ask a follow-up" : "Ask anything"}
                onChange={setDraft}
                onSubmit={submit}
                maxRows={8}
                marker={false}
              />

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
            </div>
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
    </div>
  );
}
