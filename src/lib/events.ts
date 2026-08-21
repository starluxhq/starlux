import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { StreamEvent } from "./types";

type Off = () => void;

// Scoped to this window rather than the global `listen`, which registers an
// `EventTarget::Any` listener that Tauri delivers to whatever the emitter's
// target or filter says.
function subscribe<T>(name: string, handle: (payload: T) => void): Off {
  const pending = getCurrentWebviewWindow().listen<T>(name, (event) => handle(event.payload));
  return () => {
    void pending.then((off) => off());
  };
}

/** Runs started in the other window, so expanding mid-stream keeps the answer. */
export const onStream = (handle: (event: StreamEvent) => void) =>
  subscribe<StreamEvent>("starlux://stream", handle);

export const onConversationsChanged = (handle: () => void) =>
  subscribe<null>("starlux://conversations", handle);

export const onFocusConversation = (handle: (id: string | null) => void) =>
  subscribe<string | null>("starlux://focus", handle);

export const onAsk = (handle: (prompt: string) => void) =>
  subscribe<string>("starlux://ask", handle);
