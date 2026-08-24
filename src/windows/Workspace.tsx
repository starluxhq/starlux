import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import AgentMode from "../components/AgentMode";
import ArtifactViewer from "../components/ArtifactViewer";
import Attachments from "../components/Attachments";
import Composer from "../components/Composer";
import ContextMeter from "../components/ContextMeter";
import { ModelMenu, ModelTrigger } from "../components/ModelPicker";
import { ProviderMenu, ProviderTrigger } from "../components/ProviderPicker";
import ProviderHint from "../components/ProviderHint";
import SettingsModal from "../components/SettingsModal";
import Sidebar from "../components/Sidebar";
import Turn from "../components/Turn";
import WindowControls from "../components/WindowControls";
import { PICKER } from "../lib/models";
import { platform } from "../lib/platform";
import type { Attachment } from "../lib/types";
import { onConversationsChanged, onFocusConversation } from "../lib/events";
import { useMirroredWindow } from "../lib/mirror";
import { activeConversation, saveSidebarCollapsed, sidebarCollapsed } from "../lib/ipc";
import { useArtifact } from "../stores/useArtifact";
import { currentContext, useChat } from "../stores/useChat";
import { useConversations } from "../stores/useConversations";
import { useSettings } from "../stores/useSettings";

export default function Workspace() {
  const [draft, setDraft] = useState("");
  const [files, setFiles] = useState<Attachment[]>([]);
  const [picking, setPicking] = useState<"provider" | "model" | null>(null);
  const [settings, setSettings] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
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
    selectProvider,
    selectModel,
    setAgentDir,
    send,
    retry,
    edit,
    stop,
  } = useChat();
  const context = currentContext(turns);
  const { tools, loadTools } = useSettings();
  const { items, load, rename, pin, remove } = useConversations();
  const { expanded, collapse } = useArtifact();

  useEffect(() => {
    void loadProviders();
    void load();
    void loadTools();
    void activeConversation().then((id) => {
      if (id) void openConversation(id);
    });
    void sidebarCollapsed().then(setCollapsed);
  }, [loadProviders, load, loadTools, openConversation]);

  useMirroredWindow();
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
      if (!(event.target as HTMLElement).closest(`[${PICKER}]`)) setPicking(null);
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
      // Nearest thing first: the settings sit over everything, then a menu,
      // then an artifact, then the run itself.
      if (event.key === "Escape" && settings) setSettings(false);
      else if (event.key === "Escape" && picking) setPicking(null);
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
  }, [status, stop, newConversation, expanded, collapse, picking, settings]);


  const showSidebar = (shown: boolean) => {
    setAnimate(true);
    setCollapsed(!shown);
    void saveSidebarCollapsed(!shown);
  };

  const submit = () => {
    void send(draft, files);
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
      {/* A modal `<dialog>`, so it is in the top layer rather than positioned
          over the page. Escape is handled below all the same: the keystroke
          still reaches this window, and the ladder must not also stop a run. */}
      {settings ? <SettingsModal onClose={() => setSettings(false)} /> : null}
      {/* The window has no toolkit chrome, so this strip is both the handle it
          is moved by and the corner its controls sit in. Indented on macOS,
          where the traffic lights are drawn over the page's own top-left. */}
      <div
        data-tauri-drag-region
        className={`flex h-10 shrink-0 items-center gap-4 border-b border-white/6 pr-2 ${
          platform === "macos" ? "pl-[82px]" : "pl-5"
        }`}
      >
        <span data-tauri-drag-region className="font-serif text-[15px] tracking-tight">
          Starlux
        </span>

        <div className="ml-auto flex min-w-0 items-center gap-4">
          <AgentMode
            dir={agentDir}
            tools={tools}
            onPick={() => void pickFolder()}
            onClear={() => void setAgentDir(null)}
          />
          {platform === "macos" ? null : <WindowControls />}
        </div>
      </div>

      <div className="flex min-h-0 flex-1">
        <Sidebar
          items={items}
          activeId={conversationId}
          collapsed={collapsed}
          animate={animate}
          onToggle={() => showSidebar(collapsed)}
          onNew={startConversation}
          onOpenSettings={() => setSettings(true)}
          onOpen={(id) => void openConversation(id)}
          onRename={(id, title) => void rename(id, title)}
          onPin={(id, pinned) => void pin(id, pinned)}
          onDelete={(id) => {
            if (id === conversationId) newConversation();
            void remove(id);
          }}
        />

        <main className="flex min-w-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-6">
            {turns.length === 0 ? (
              <p className="max-w-md font-serif text-[22px] leading-snug text-muted">
                Light that left a long time ago, arriving one token at a time.
              </p>
            ) : (
              turns.map((turn) => (
                <Turn
                  key={turn.id}
                  turn={turn}
                  status={status}
                  runId={runId}
                  onRetry={(id) => void retry(id)}
                  onEdit={(id, text) => void edit(id, text)}
                />
              ))
            )}
          </div>

          <div className="relative shrink-0 border-t border-white/6 px-6 py-4">
            {picking === "provider" ? (
              <ProviderMenu
                className="absolute right-6 bottom-full mb-2"
                providers={providers}
                providerId={providerId}
                limits={limits}
                onSelect={(next) => {
                  selectProvider(next);
                  setPicking(null);
                }}
              />
            ) : null}

            {picking === "model" && model ? (
              <ModelMenu
                className="absolute right-6 bottom-full mb-2"
                provider={providers.find((provider) => provider.id === providerId)}
                model={model}
                onSelect={(next) => {
                  selectModel(next);
                  setPicking(null);
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
                <>
                  <ProviderTrigger
                    providers={providers}
                    providerId={providerId}
                    open={picking === "provider"}
                    onToggle={() => setPicking((was) => (was === "provider" ? null : "provider"))}
                  />
                  <ModelTrigger
                    model={model}
                    open={picking === "model"}
                    onToggle={() => setPicking((was) => (was === "model" ? null : "model"))}
                  />
                </>
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
