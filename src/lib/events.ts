import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Selection, StreamEvent, Tools } from "./types";

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

/** What the other window chose for the next run. The choice outlives any one
 *  conversation, so both windows hold it and both have to hear about it. */
export const onSelection = (handle: (chosen: Selection) => void) =>
  subscribe<Selection>("starlux://selection", handle);

/** What the other window granted or gave back. One answer for the whole app, so
 *  neither window may sit showing a grant that no longer stands. */
export const onTools = (handle: (tools: Tools) => void) =>
  subscribe<Tools>("starlux://tools", handle);
