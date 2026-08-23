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
  onNew: () => void;
  onCollapse: () => void;
}

export default function SidebarToolbar({ onNew, onCollapse }: SidebarToolbarProps) {
  return (
    <div className="flex items-center justify-between px-3 pt-3">
      <button type="button" onClick={onNew} aria-label="New conversation" className={BUTTON}>
        <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
          <path d="M8 3.5v9M3.5 8h9" {...STROKE} />
        </svg>
      </button>

      <button type="button" onClick={onCollapse} aria-label="Hide conversations" className={BUTTON}>
        <svg viewBox="0 0 16 16" aria-hidden className="size-3.5">
          <path d="M9.5 4l-4 4 4 4" {...STROKE} />
        </svg>
      </button>
    </div>
  );
}
