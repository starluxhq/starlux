interface AgentModeProps {
  dir: string | null;
  /** Both are absent in the Quick Bar, which shows the grant but cannot change
   *  it: a hotkey question should never be one click from filesystem access. */
  onPick?: () => void;
  onClear?: () => void;
}

const LABEL = "font-mono text-[10px] tracking-wide uppercase";

const nameOf = (dir: string) => dir.split(/[\\/]/).filter(Boolean).pop() ?? dir;

/** Whether the assistant can reach the filesystem, and where. */
export default function AgentMode({ dir, onPick, onClear }: AgentModeProps) {
  if (!dir) {
    return onPick ? (
      <button type="button" onClick={onPick} className={`${LABEL} text-muted hover:text-ink`}>
        Work in a folder
      </button>
    ) : null;
  }

  // A diamond rather than the round provider dot: this one is a grant, not a
  // label, and has to read differently at the size both are shown.
  const folder = (
    <span className="inline-flex min-w-0 items-center gap-1.5">
      <span aria-hidden className="size-[5px] shrink-0 rotate-45 bg-class-k" />
      <span className="truncate">{nameOf(dir)}</span>
    </span>
  );

  return (
    <span className={`inline-flex min-w-0 items-center gap-2.5 ${LABEL}`}>
      {onPick ? (
        <button
          type="button"
          onClick={onPick}
          title={dir}
          aria-label={`Working in ${dir}. Choose another folder`}
          className="min-w-0 text-class-k hover:text-ink"
        >
          {folder}
        </button>
      ) : (
        <span title={dir} className="min-w-0 text-class-k">
          {folder}
        </span>
      )}
      {onClear ? (
        <button type="button" onClick={onClear} className="shrink-0 text-faint hover:text-ink">
          Chat only
        </button>
      ) : null}
    </span>
  );
}
