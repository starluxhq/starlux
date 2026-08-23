import type { Attachment } from "../lib/types";

interface AttachmentsProps {
  items: Attachment[];
  /** Absent once the question has been asked: what was sent was sent. */
  onRemove?: (path: string) => void;
  className?: string;
}

/** Above the input rather than inside it, so a second and third file wrap into
 *  the row instead of squeezing the question. */
export default function Attachments({ items, onRemove, className = "" }: AttachmentsProps) {
  if (items.length === 0) return null;

  return (
    <ul className={`flex flex-wrap gap-2 ${className}`}>
      {items.map((item) => (
        <li key={item.path}>
          {onRemove ? (
            <button
              type="button"
              title={item.path}
              onClick={() => onRemove(item.path)}
              className="group flex max-w-44 items-center gap-1.5 rounded-md border border-rule bg-haze px-2 py-1 text-[11.5px] text-muted hover:border-class-m/60 hover:text-ink"
            >
              <span className="truncate">{item.name}</span>
              <span aria-hidden className="text-faint group-hover:text-class-m">
                ×
              </span>
            </button>
          ) : (
            <span
              title={item.path}
              className="flex max-w-44 items-center rounded-md border border-rule bg-haze px-2 py-1 text-[11.5px] text-muted"
            >
              <span className="truncate">{item.name}</span>
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}
