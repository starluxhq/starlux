import { useEffect, useState } from "react";

const BUTTON =
  "flex size-6 shrink-0 items-center justify-center rounded-md text-faint hover:bg-white/6 hover:text-ink";

const STROKE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

interface TurnActionsProps {
  /** `null` when there is nothing to copy, as on a run that only errored. */
  onCopy: (() => void) | null;
  onEdit?: () => void;
  onRetry?: () => void;
}

const COPIED_FOR = 1400;

export default function TurnActions({ onCopy, onEdit, onRetry }: TurnActionsProps) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), COPIED_FOR);
    return () => clearTimeout(timer);
  }, [copied]);

  return (
    <div
      // Always occupying its row, so a turn does not shift under the pointer
      // the moment it is hovered. Faded rather than hidden for the same reason.
      className="mt-1 flex h-6 gap-0.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
    >
      {onEdit ? (
        <button type="button" onClick={onEdit} aria-label="Edit" className={BUTTON}>
          <svg viewBox="0 0 16 16" aria-hidden className="size-3">
            <path d="M11 2.5l2.5 2.5-7 7H4v-2.5z" {...STROKE} />
          </svg>
        </button>
      ) : null}

      {onRetry ? (
        <button type="button" onClick={onRetry} aria-label="Retry" className={BUTTON}>
          <svg viewBox="0 0 16 16" aria-hidden className="size-3">
            <path d="M13 8a5 5 0 1 1-1.6-3.7M13 2.5V5h-2.5" {...STROKE} />
          </svg>
        </button>
      ) : null}

      {onCopy ? (
        <button
          type="button"
          onClick={() => {
            onCopy();
            setCopied(true);
          }}
          aria-label={copied ? "Copied" : "Copy"}
          className={BUTTON}
        >
          <svg viewBox="0 0 16 16" aria-hidden className="size-3">
            {copied ? (
              <path d="M3.5 8.5l3 3 6-6.5" {...STROKE} />
            ) : (
              <path d="M6 2.5h7.5V10M2.5 6h7.5v7.5H2.5z" {...STROKE} />
            )}
          </svg>
        </button>
      ) : null}
    </div>
  );
}
