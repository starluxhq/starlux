export interface Attachment {
  path: string;
  name: string;
}

interface AttachmentsProps {
  items: Attachment[];
  onRemove: (path: string) => void;
}

/** Above the input rather than inside it, so a second and third file wrap into
 *  the row instead of squeezing the question. */
export default function Attachments({ items, onRemove }: AttachmentsProps) {
  if (items.length === 0) return null;

  return (
    <ul className="flex flex-wrap gap-2 px-3 pt-3">
      {items.map((item) => (
        <li key={item.path}>
          <button
            type="button"
            title={`${item.path}\n\nNot sent to the provider yet.`}
            onClick={() => onRemove(item.path)}
            className="group flex max-w-44 items-center gap-1.5 rounded-md border border-rule bg-haze px-2 py-1 text-[11.5px] text-muted hover:border-class-m/60 hover:text-ink"
          >
            <span className="truncate">{item.name}</span>
            <span aria-hidden className="text-faint group-hover:text-class-m">
              ×
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
