interface AgentModeProps {
  dir: string | null;
  web: boolean;
  /** All three are absent in the Quick Bar, which shows the grants but cannot
   *  change them: a hotkey question should never be one click from the
   *  filesystem or the network. */
  onPick?: () => void;
  onClear?: () => void;
  onWeb?: (web: boolean) => void;
}

const LABEL = "font-mono text-[10px] tracking-wide uppercase";
/** Preflight resets `text-transform` on buttons, so a button under `LABEL`
 *  quietly opts out of the caps every other micro-label in the chrome wears. */
const CAPS = "uppercase";

const nameOf = (dir: string) => dir.split(/[\\/]/).filter(Boolean).pop() ?? dir;

/** What the assistant may reach, and where. Two grants side by side rather than
 *  one ladder: looking something up should not cost a folder, and opening a
 *  folder should not quietly buy the network. */
export default function AgentMode({ dir, web, onPick, onClear, onWeb }: AgentModeProps) {
  // A diamond rather than the round provider dot: this one is a grant, not a
  // label, and has to read differently at the size both are shown.
  const folder = (name: string) => (
    <span className="inline-flex min-w-0 items-center gap-1.5">
      <span aria-hidden className="size-[5px] shrink-0 rotate-45 bg-class-k" />
      <span className="truncate">{nameOf(name)}</span>
    </span>
  );

  // An open ring for the one grant that points away from the machine, against
  // the folder's solid diamond.
  const ring = (
    <span
      aria-hidden
      className={`size-[6px] shrink-0 rounded-full border ${
        web ? "border-class-a bg-class-a/40" : "border-faint"
      }`}
    />
  );

  const webLabel = web ? "Searching and fetching are on" : "Let this conversation search the web";

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
            <button type="button" onClick={onClear} className={`shrink-0 text-faint hover:text-ink ${CAPS}`}>
              Chat only
            </button>
          ) : null}
        </span>
      ) : onPick ? (
        <button type="button" onClick={onPick} className={`shrink-0 text-muted hover:text-ink ${CAPS}`}>
          Work in a folder
        </button>
      ) : null}

      {onWeb ? (
        <button
          type="button"
          onClick={() => onWeb(!web)}
          aria-pressed={web}
          title={webLabel}
          className={`inline-flex shrink-0 items-center gap-1.5 ${CAPS} ${
            web ? "text-class-a hover:text-ink" : "text-muted hover:text-ink"
          }`}
        >
          {ring}
          Web
        </button>
      ) : web ? (
        <span
          title={webLabel}
          className="inline-flex shrink-0 items-center gap-1.5 text-class-a"
        >
          {ring}
          Web
        </span>
      ) : null}
    </span>
  );
}
