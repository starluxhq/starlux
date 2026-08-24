import { useEffect, useRef, useState } from "react";
import ToolSettings from "./ToolSettings";
import { useChat } from "../stores/useChat";
import { useSettings } from "../stores/useSettings";

interface SettingsModalProps {
  onClose: () => void;
}

/** One entry per pane. The list is here rather than in a shared module because
 *  a component file that exports anything else gives up Fast Refresh for
 *  everything in it. */
const SECTIONS = [{ id: "tools", name: "Tools" }];

const STROKE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** Settings that belong to the app rather than to a conversation.
 *
 *  A real `<dialog>`, opened modally: the top layer, the focus trap and Escape
 *  are the platform's rather than three things to reimplement. Its `close`
 *  event is what tells the Workspace, since the element can close itself. */
export default function SettingsModal({ onClose }: SettingsModalProps) {
  const [section, setSection] = useState(SECTIONS[0].id);
  const dialog = useRef<HTMLDialogElement>(null);
  const providers = useChat((state) => state.providers);
  const { tools, loadTools, setTool } = useSettings();

  // Read on open rather than trusted from the last time this window looked:
  // the other one may have granted or given something back since.
  useEffect(() => {
    dialog.current?.showModal();
    void loadTools();
  }, [loadTools]);

  return (
    <dialog
      ref={dialog}
      aria-label="Settings"
      onClose={onClose}
      className="fixed inset-0 m-0 hidden h-full max-h-none w-full max-w-none items-center justify-center bg-void/70 p-8 text-ink open:flex"
    >
      {/* Clicking off the panel closes it. A submit button rather than a
          handler on the backdrop: `method="dialog"` is the platform's own way
          to close one, and it needs no JavaScript to be right. Out of the tab
          order, where the close button in the corner already is. */}
      <form method="dialog" className="absolute inset-0">
        <button type="submit" tabIndex={-1} aria-hidden className="size-full cursor-default" />
      </form>

      <div className="relative flex h-[34rem] max-h-full w-[48rem] max-w-full overflow-hidden rounded-xl border border-rule bg-dust shadow-2xl shadow-black/50">
        <nav className="w-48 shrink-0 border-r border-rule bg-void/40 p-3">
          <p className="px-2 pt-1 pb-2 font-mono text-[10px] tracking-wider text-faint uppercase">
            Settings
          </p>
          {SECTIONS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              onClick={() => setSection(entry.id)}
              aria-current={entry.id === section}
              className={`block w-full rounded-md px-2 py-1.5 text-left text-[13px] ${
                entry.id === section ? "bg-white/6 text-ink" : "text-muted hover:text-ink"
              }`}
            >
              {entry.name}
            </button>
          ))}
        </nav>

        <div className="flex min-w-0 flex-1 flex-col">
          <div className="flex h-12 shrink-0 items-center border-b border-rule px-5">
            <h2 className="font-serif text-[17px] tracking-tight">
              {SECTIONS.find((entry) => entry.id === section)?.name}
            </h2>
            <button
              type="button"
              onClick={() => dialog.current?.close()}
              aria-label="Close settings"
              className="ml-auto flex size-7 items-center justify-center rounded-md text-muted hover:bg-white/6 hover:text-ink"
            >
              <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
                <path d="M4 4l8 8M12 4l-8 8" {...STROKE} />
              </svg>
            </button>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto px-5 py-5">
            <ToolSettings
              tools={tools}
              providers={providers}
              onChange={(id, on) => void setTool(id, on)}
            />
          </div>
        </div>
      </div>
    </dialog>
  );
}
