import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  Conversation,
  Provider,
  RateLimit,
  RunRequest,
  Selection,
  StreamEvent,
  Thread,
  ToolId,
  Tools,
} from "./types";

export const hideQuickBar = () => invoke<void>("hide_quickbar");
export const toggleQuickBar = () => invoke<void>("toggle_quickbar");
export const openWorkspace = () => invoke<void>("open_workspace");

/** The bar measures its own content; Rust bounds what it will accept. */
export const setQuickbarHeight = (height: number) =>
  invoke<void>("set_quickbar_height", { height });
export const listProviders = () => invoke<Provider[]>("list_providers");
export const selectedModel = () => invoke<Selection | null>("selected_model");

/** What each provider was last asked for, so switching back returns to it. */
export const rememberedModels = () => invoke<Selection[]>("remembered_models");

/** The last window each provider reported, so the bar has something to show
 *  before the first run of this launch refreshes it. */
export const rateLimits = () => invoke<RateLimit[]>("rate_limits");
export const saveSelectedModel = (providerId: string, model: string, effort: string | null) =>
  invoke<void>("set_selected_model", { providerId, model, effort });
export const sidebarCollapsed = () => invoke<boolean>("sidebar_collapsed");
export const saveSidebarCollapsed = (collapsed: boolean) =>
  invoke<void>("set_sidebar_collapsed", { collapsed });
export const storeArtifact = (html: string) => invoke<string>("store_artifact", { html });
export const cancelRun = (runId: string) => invoke<boolean>("cancel_run", { runId });

export const activeConversation = () => invoke<string | null>("active_conversation");
export const listConversations = () => invoke<Conversation[]>("list_conversations");
export const loadConversation = (id: string) => invoke<Thread | null>("load_conversation", { id });
export const deleteConversation = (id: string) => invoke<void>("delete_conversation", { id });

/** `null` takes the folder back. The tools are granted app-wide and unaffected. */
export const setAgentDir = (id: string, dir: string | null) =>
  invoke<void>("set_agent_dir", { id, dir });

/** What every run may reach. Not a property of a conversation, which is why it
 *  is read and written without one. */
export const tools = () => invoke<Tools>("tools");

/** Answers with the whole grant rather than the one bit that changed, so the
 *  window never has to reconstruct what it should now be. */
export const setTool = (id: ToolId, on: boolean) => invoke<Tools>("set_tool", { id, on });

export const renameConversation = (id: string, title: string) =>
  invoke<void>("rename_conversation", { id, title });

export const setPinned = (id: string, pinned: boolean) =>
  invoke<void>("set_pinned", { id, pinned });

/** Everything after this message goes, so a retry or an edit rewrites the
 *  thread rather than adding to it. */
export const truncateAfter = (conversationId: string, messageId: string) =>
  invoke<void>("truncate_after", { conversationId, messageId });

export const setBlurHideSuppressed = (suppressed: boolean) =>
  invoke<void>("set_blur_hide_suppressed", { suppressed });

export function runPrompt(request: RunRequest, onEvent: (event: StreamEvent) => void) {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("run_prompt", { request, onEvent: channel });
}
