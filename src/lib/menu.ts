import { useEffect, useLayoutEffect, useRef } from "react";

/** Marks an open menu, so a click inside it is not a dismissal. Lives here
 *  rather than beside the component: a component file that exports anything
 *  else gives up Fast Refresh for everything in it. */
export const MENU = "data-context-menu";

/** The webview's own menu is a browser's — Back, Reload, Inspect Element — in
 *  an app that is not a browser. Taken away everywhere, including text fields,
 *  where we serve the clipboard items ourselves. A sandboxed artifact keeps
 *  its own: the event never leaves that iframe's document. */
export function suppressNativeMenu() {
  document.addEventListener("contextmenu", (event) => event.preventDefault());
}

/** Closes an open menu on anything that means "not this": a click elsewhere,
 *  Escape, or the surface underneath moving. */
export function useDismiss(open: boolean, onClose: () => void) {
  // Held in a ref so the listeners are registered once per opening, rather
  // than torn down and rebuilt on every render of whatever opened them.
  const close = useRef(onClose);
  useLayoutEffect(() => {
    close.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!open) return;
    const away = (event: MouseEvent) => {
      if (!(event.target as HTMLElement).closest(`[${MENU}]`)) close.current();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") close.current();
    };
    const moved = () => close.current();
    document.addEventListener("mousedown", away, true);
    document.addEventListener("keydown", key, true);
    window.addEventListener("resize", moved);
    window.addEventListener("scroll", moved, true);
    return () => {
      document.removeEventListener("mousedown", away, true);
      document.removeEventListener("keydown", key, true);
      window.removeEventListener("resize", moved);
      window.removeEventListener("scroll", moved, true);
    };
  }, [open]);
}
