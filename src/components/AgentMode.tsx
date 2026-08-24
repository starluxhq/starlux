import type { Tools } from "../lib/types";

interface AgentModeProps {
  dir: string | null;
  /** Absent where the settings are, which is the Workspace: a badge repeating
   *  what the panel two clicks away already says is chrome. The Quick Bar has
   *  no way in there and passes them, because a hotkey question reaching the
   *  network is worth saying out loud. Never changed either way. */
  tools?: Tools;
  /** Both are absent in the Quick Bar, which shows the folder but cannot change
   *  it: a hotkey question should never be one click from the filesystem. */
  onPick?: () => void;
  onClear?: () => void;
}

const LABEL = "font-mono text-[10px] tracking-wide uppercase";
/** Preflight resets `text-transform` on buttons, so a button under `LABEL`
 *  quietly opts out of the caps every other micro-label in the chrome wears. */
const CAPS = "uppercase";

const nameOf = (dir: string) => dir.split(/[\\/]/).filter(Boolean).pop() ?? dir;

/** A diamond rather than the round provider dot: this one is a grant, not a
 *  label, and has to read differently at the size both are shown. */
const folder = (dir: string) => (
  <span className="inline-flex min-w-0 items-center gap-1.5">
    <span aria-hidden className="size-[5px] shrink-0 rotate-45 bg-class-k" />
    <span className="truncate">{nameOf(dir)}</span>
  </span>
);

const reach = (tools: Tools) => {
  const on = [tools.webSearch && "search", tools.webFetch && "fetch"].filter(Boolean);
  return on.length === 2 ? "Searching and fetching are on" : `${on[0]} is on`;
};

/** What the assistant may reach, and where. Two grants side by side rather than
 *  one ladder: looking something up should not cost a folder, and opening a
 *  folder should not quietly buy the network. */
export default function AgentMode({ dir, tools, onPick, onClear }: AgentModeProps) {
  return (
    <span className={`inline-flex min-w-0 items-center gap-3 ${LABEL}`}>
      {dir ? (
        <span className="inline-flex min-w-0 items-center gap-2.5">
          {onPick ? (
            <button
              type="button"
              onClick={onPick}
              title={dir}
              aria-label={`Working in ${dir}. Choose another folder`}
              className={`min-w-0 text-class-k hover:text-ink ${CAPS}`}
            >
              {folder(dir)}
            </button>
          ) : (
            <span title={dir} className="min-w-0 text-class-k">
              {folder(dir)}
            </span>
          )}
          {onClear ? (
            <button
              type="button"
              onClick={onClear}
              className={`shrink-0 text-faint hover:text-ink ${CAPS}`}
            >
              Chat only
            </button>
          ) : null}
        </span>
      ) : onPick ? (
        <button type="button" onClick={onPick} className={`shrink-0 text-muted hover:text-ink ${CAPS}`}>
          Work in a folder
        </button>
      ) : null}

      {/* An open ring for the one grant that points away from the machine,
          against the folder's solid diamond. */}
      {tools && (tools.webSearch || tools.webFetch) ? (
        <span title={reach(tools)} className="inline-flex shrink-0 items-center gap-1.5 text-class-a">
          <span aria-hidden className="size-[6px] shrink-0 rounded-full border border-class-a bg-class-a/40" />
          Web
        </span>
      ) : null}
    </span>
  );
}
