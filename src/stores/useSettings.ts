import { create } from "zustand";
import { setTool as saveTool, tools as loadTools } from "../lib/ipc";
import type { ToolId, Tools } from "../lib/types";

interface SettingsState {
  tools: Tools;
  loadTools: () => Promise<void>;
  setTool: (id: ToolId, on: boolean) => Promise<void>;
  adoptTools: (tools: Tools) => void;
}

/** App settings, kept apart from the conversation. What a run may reach is one
 *  answer for the whole app: a question asked from the bar reaches exactly what
 *  one asked from the Workspace does, and neither window is the one that
 *  decides. */
export const useSettings = create<SettingsState>((set) => ({
  tools: { webSearch: false, webFetch: false },

  loadTools: async () => set({ tools: await loadTools() }),

  // The core answers with the whole grant rather than the bit that changed, so
  // what lands here is what the next run will actually be given.
  setTool: async (id, on) => set({ tools: await saveTool(id, on) }),

  /** Applied without writing it back, so the window told about a grant does not
   *  tell the other one in turn. */
  adoptTools: (tools) => set({ tools }),
}));
