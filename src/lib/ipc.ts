import { Channel, invoke } from "@tauri-apps/api/core";
import type { Conversation, Provider, RunRequest, StreamEvent, Thread } from "./types";

export const hideQuickBar = () => invoke<void>("hide_quickbar");
export const toggleQuickBar = () => invoke<void>("toggle_quickbar");
export const openWorkspace = () => invoke<void>("open_workspace");

/** The bar measures its own content; Rust bounds what it will accept. */
export const setQuickbarHeight = (height: number) =>
  invoke<void>("set_quickbar_height", { height });
export const listProviders = () => invoke<Provider[]>("list_providers");
export const storeArtifact = (html: string) => invoke<string>("store_artifact", { html });
export const cancelRun = (runId: string) => invoke<boolean>("cancel_run", { runId });

export const activeConversation = () => invoke<string | null>("active_conversation");
export const listConversations = () => invoke<Conversation[]>("list_conversations");
export const loadConversation = (id: string) => invoke<Thread | null>("load_conversation", { id });
export const deleteConversation = (id: string) => invoke<void>("delete_conversation", { id });

/** `null` returns the conversation to chat-only. */
export const setAgentDir = (id: string, dir: string | null) =>
  invoke<void>("set_agent_dir", { id, dir });

export const renameConversation = (id: string, title: string) =>
  invoke<void>("rename_conversation", { id, title });

export const setBlurHideSuppressed = (suppressed: boolean) =>
  invoke<void>("set_blur_hide_suppressed", { suppressed });

export function runPrompt(request: RunRequest, onEvent: (event: StreamEvent) => void) {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("run_prompt", { request, onEvent: channel });
}
