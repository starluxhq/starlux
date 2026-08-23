import { useState } from "react";
import { shortAge } from "../lib/time";
import type { Conversation } from "../lib/types";
import ContextMenu from "./ContextMenu";
import SpectralDot from "./SpectralDot";

interface ConversationListProps {
  items: Conversation[];
  activeId: string | null;
  onOpen: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onPin: (id: string, pinned: boolean) => void;
  onDelete: (id: string) => void;
}

interface OpenMenu {
  id: string;
  x: number;
  y: number;
}

export default function ConversationList({
  items,
  activeId,
  onOpen,
  onRename,
  onPin,
  onDelete,
}: ConversationListProps) {
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);

  if (items.length === 0) {
    return (
      <p className="px-3 py-2 text-[12.5px] text-faint">
        Conversations you start will collect here.
      </p>
    );
  }

  const open = items.find((item) => item.id === menu?.id);

  const commit = (item: Conversation, title: string) => {
    setRenaming(null);
    const trimmed = title.trim();
    if (trimmed && trimmed !== item.title) onRename(item.id, trimmed);
  };

  return (
    <ul>
      {items.map((item) => (
        <li key={item.id} className="group relative">
          {renaming === item.id ? (
            <input
              autoFocus
              defaultValue={item.title}
              onBlur={(event) => commit(item, event.target.value)}
              onKeyDown={(event) => {
                // The field is dropped from the tree once the key is handled,
                // and losing focus that way still fires `onBlur`. Putting the
                // old title back is what makes that second pass a no-op.
                if (event.key === "Enter") commit(item, event.currentTarget.value);
                if (event.key === "Enter" || event.key === "Escape") {
                  event.currentTarget.value = item.title;
                  setRenaming(null);
                }
              }}
              aria-label={`Rename ${item.title}`}
              className="w-full rounded-md bg-haze px-3 py-2 text-[12.5px] text-ink outline-none"
            />
          ) : (
            <button
              type="button"
              onClick={() => onOpen(item.id)}
              onContextMenu={(event) => setMenu({ id: item.id, x: event.clientX, y: event.clientY })}
              aria-current={item.id === activeId}
              className="flex w-full items-baseline gap-2 rounded-md px-3 py-2 text-left hover:bg-haze/60 aria-[current=true]:bg-haze"
            >
              <SpectralDot providerId={item.providerId} className="translate-y-[-1px] opacity-70" />
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink/85">{item.title}</span>
              {item.pinned ? (
                <span aria-label="Pinned" className="shrink-0 text-[10px] text-class-k">
                  ●
                </span>
              ) : null}
              <span className="shrink-0 font-mono text-[10px] text-faint group-hover:invisible">
                {shortAge(item.updatedAt)}
              </span>
            </button>
          )}

          {renaming === item.id ? null : (
            <button
              type="button"
              onClick={(event) =>
                setMenu({ id: item.id, x: event.clientX, y: event.clientY })
              }
              aria-label={`Actions for ${item.title}`}
              className="absolute top-1/2 right-2 hidden -translate-y-1/2 px-1 font-mono text-[13px] leading-none text-faint group-hover:block hover:text-ink focus-visible:block"
            >
              ⋯
            </button>
          )}
        </li>
      ))}

      {menu && open ? (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          items={[
            {
              label: open.pinned ? "Unpin" : "Pin",
              onSelect: () => onPin(open.id, !open.pinned),
            },
            { label: "Rename", onSelect: () => setRenaming(open.id) },
            { label: "Delete", onSelect: () => onDelete(open.id), destructive: true },
          ]}
        />
      ) : null}
    </ul>
  );
}
