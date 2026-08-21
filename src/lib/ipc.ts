import { Channel, invoke } from "@tauri-apps/api/core";
import type { Provider, RunRequest, StreamEvent } from "./types";

export const hideQuickBar = () => invoke<void>("hide_quickbar");
export const toggleQuickBar = () => invoke<void>("toggle_quickbar");
export const openWorkspace = () => invoke<void>("open_workspace");
export const listProviders = () => invoke<Provider[]>("list_providers");
export const cancelRun = (runId: string) => invoke<boolean>("cancel_run", { runId });

export const setBlurHideSuppressed = (suppressed: boolean) =>
  invoke<void>("set_blur_hide_suppressed", { suppressed });

export function runPrompt(request: RunRequest, onEvent: (event: StreamEvent) => void) {
  const channel = new Channel<StreamEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("run_prompt", { request, onEvent: channel });
}
