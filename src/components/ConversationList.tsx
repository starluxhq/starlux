import { shortAge } from "../lib/time";
import type { Conversation } from "../lib/types";
import SpectralDot from "./SpectralDot";

interface ConversationListProps {
  items: Conversation[];
  activeId: string | null;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
}

export default function ConversationList({
  items,
  activeId,
  onOpen,
  onDelete,
}: ConversationListProps) {
  if (items.length === 0) {
    return (
      <p className="px-3 py-2 text-[12.5px] text-faint">
        Conversations you start will collect here.
      </p>
    );
  }

  return (
    <ul>
      {items.map((item) => (
        <li key={item.id} className="group relative">
          <button
            type="button"
            onClick={() => onOpen(item.id)}
            aria-current={item.id === activeId}
            className="flex w-full items-baseline gap-2 rounded-md px-3 py-2 text-left hover:bg-haze/60 aria-[current=true]:bg-haze"
          >
            <SpectralDot providerId={item.providerId} className="translate-y-[-1px] opacity-70" />
            <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink/85">{item.title}</span>
            <span className="shrink-0 font-mono text-[10px] text-faint group-hover:invisible">
              {shortAge(item.updatedAt)}
            </span>
          </button>
          <button
            type="button"
            onClick={() => onDelete(item.id)}
            aria-label={`Delete ${item.title}`}
            className="absolute top-1/2 right-2 hidden -translate-y-1/2 px-1 font-mono text-[11px] text-faint group-hover:block hover:text-class-m focus-visible:block"
          >
            ✕
          </button>
        </li>
      ))}
    </ul>
  );
}
