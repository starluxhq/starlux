interface SidebarTrayProps {
  collapsed: boolean;
  onOpenSettings: () => void;
}

const STROKE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** The foot of the sidebar. It survives collapsing for the same reason the
 *  toolbar above it does: the settings are still reachable from the strip, on
 *  the same left edge, without opening the list to get at them. */
export default function SidebarTray({ collapsed, onOpenSettings }: SidebarTrayProps) {
  return (
    <div className={`shrink-0 border-t border-white/6 p-3 ${collapsed ? "w-12" : ""}`}>
      <button
        type="button"
        onClick={onOpenSettings}
        aria-label="Settings"
        className={`flex h-7 items-center gap-2 rounded-md text-muted hover:bg-white/6 hover:text-ink ${
          collapsed ? "w-7 justify-center" : "w-full px-1.5"
        }`}
      >
        {/* Faders rather than a gear: a gear's teeth turn to mush at this
            size, and what is behind the button is a panel of switches. */}
        <svg viewBox="0 0 16 16" aria-hidden className="size-3.5 shrink-0">
          <path d="M2 4.5h12M2 11.5h12" {...STROKE} />
          <circle cx="5.5" cy="4.5" r="1.7" fill="currentColor" />
          <circle cx="10.5" cy="11.5" r="1.7" fill="currentColor" />
        </svg>
        {collapsed ? null : <span className="text-[12.5px] whitespace-nowrap">Settings</span>}
      </button>
    </div>
  );
}
