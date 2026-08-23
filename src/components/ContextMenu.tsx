import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { MENU, useDismiss } from "../lib/menu";

export interface MenuItem {
  label: string;
  onSelect: () => void;
  /** Drawn in the colour the app already reserves for losing something. */
  destructive?: boolean;
}

interface ContextMenuProps {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

const MARGIN = 6;

export default function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState({ x, y });

  useDismiss(true, onClose);

  // The pointer decides where this opens, so the only thing keeping it on
  // screen is folding it back once its size is known.
  useLayoutEffect(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    setAt({
      x: Math.max(MARGIN, Math.min(x, window.innerWidth - box.width - MARGIN)),
      y: Math.max(MARGIN, Math.min(y, window.innerHeight - box.height - MARGIN)),
    });
  }, [x, y, items.length]);

  // Rendered against the body: a menu opened from inside the sidebar would
  // otherwise be clipped away by the overflow that lets the list slide shut.
  return createPortal(
    <div
      ref={ref}
      role="menu"
      {...{ [MENU]: "" }}
      style={{ left: at.x, top: at.y }}
      className="fixed z-50 w-40 overflow-hidden rounded-lg border border-rule bg-haze py-1 shadow-xl shadow-black/40"
    >
      {items.map((item) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          onClick={() => {
            item.onSelect();
            onClose();
          }}
          className={`block w-full px-3 py-1.5 text-left text-[12.5px] hover:bg-white/6 ${
            item.destructive ? "text-class-m" : "text-ink/85"
          }`}
        >
          {item.label}
        </button>
      ))}
    </div>,
    document.body,
  );
}
