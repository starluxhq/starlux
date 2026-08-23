const BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-md text-muted hover:bg-white/6 hover:text-ink";

const STROKE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

interface SidebarToolbarProps {
  collapsed: boolean;
  onNew: () => void;
  onToggle: () => void;
}

/** The one part of the sidebar that survives collapsing. The list closes behind
 *  it and the row becomes a column, so the buttons stay on the same left edge
 *  and the new-conversation one only ever moves down a slot. */
export default function SidebarToolbar({ collapsed, onNew, onToggle }: SidebarToolbarProps) {
  const toggle = (
    <button
      type="button"
      onClick={onToggle}
      aria-label={collapsed ? "Show conversations" : "Hide conversations"}
      className={BUTTON}
    >
      <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
        <path d={collapsed ? "M6.5 4l4 4-4 4" : "M9.5 4l-4 4 4 4"} {...STROKE} />
      </svg>
    </button>
  );

  const create = (
    <button type="button" onClick={onNew} aria-label="New conversation" className={BUTTON}>
      <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
        <path d="M8 3.5v9M3.5 8h9" {...STROKE} />
      </svg>
    </button>
  );

  return collapsed ? (
    <div className="flex w-12 flex-col items-start gap-1 px-3 pt-3">
      {toggle}
      {create}
    </div>
  ) : (
    <div className="flex items-center justify-between px-3 pt-3">
      {create}
      {toggle}
    </div>
  );
}
