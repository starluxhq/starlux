import ConversationList from "./ConversationList";
import SidebarToolbar from "./SidebarToolbar";
import SidebarTray from "./SidebarTray";
import type { Conversation } from "../lib/types";

interface SidebarProps {
  items: Conversation[];
  activeId: string | null;
  collapsed: boolean;
  /** The stored state has to land without sliding: restoring a hidden sidebar
   *  is not a move the user made, so only their own toggles are animated. */
  animate: boolean;
  onToggle: () => void;
  onNew: () => void;
  onOpenSettings: () => void;
  onOpen: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onPin: (id: string, pinned: boolean) => void;
  onDelete: (id: string) => void;
}

/** Closing it leaves a strip rather than nothing: the toolbar and the tray stay
 *  put, and the list is drawn back behind them at its full width throughout so
 *  the rows never reflow on the way out. */
export default function Sidebar({
  items,
  activeId,
  collapsed,
  animate,
  onToggle,
  onNew,
  onOpenSettings,
  onOpen,
  onRename,
  onPin,
  onDelete,
}: SidebarProps) {
  return (
    <aside
      className={`shrink-0 overflow-hidden border-r border-white/6 bg-dust/60 ${
        animate ? "transition-[width] duration-200 ease-out motion-reduce:transition-none" : ""
      } ${collapsed ? "w-12" : "w-64"}`}
    >
      <div className="flex h-full w-64 flex-col">
        <SidebarToolbar collapsed={collapsed} onNew={onNew} onToggle={onToggle} />

        <div
          className={`flex min-h-0 flex-1 flex-col ${
            animate ? "transition-opacity duration-150 motion-reduce:transition-none" : ""
          } ${collapsed ? "pointer-events-none opacity-0" : "opacity-100"}`}
        >
          <p className="px-5 pt-3 pb-3 font-mono text-[10px] tracking-wider text-faint uppercase">
            Conversations
          </p>
          <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
            <ConversationList
              items={items}
              activeId={activeId}
              onOpen={onOpen}
              onRename={onRename}
              onPin={onPin}
              onDelete={onDelete}
            />
          </div>
        </div>

        <SidebarTray collapsed={collapsed} onOpenSettings={onOpenSettings} />
      </div>
    </aside>
  );
}
