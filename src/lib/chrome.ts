import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

/** Called per use rather than once: at module scope it runs before Tauri has
 *  put its metadata on the page. */
const self = () => getCurrentWebviewWindow();

export const minimiseWindow = () => self().minimize();
export const toggleMaximiseWindow = () => self().toggleMaximize();

/** A close request, not a destroy: Rust turns it into a hide, which is what the
 *  toolkit's own button did. */
export const closeWindow = () => self().close();

export const windowIsMaximised = () => self().isMaximized();

/** The window changes between maximised and not without the button: a drag to
 *  the top edge, a keyboard shortcut, the window manager's own menu. */
export function onWindowResized(handle: () => void) {
  const pending = self().onResized(handle);
  return () => void pending.then((off) => off());
}
