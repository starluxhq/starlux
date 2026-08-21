import { create } from "zustand";
import { deleteConversation, listConversations } from "../lib/ipc";
import type { Conversation } from "../lib/types";

interface ConversationsState {
  items: Conversation[];
  load: () => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useConversations = create<ConversationsState>((set, get) => ({
  items: [],

  load: async () => {
    set({ items: await listConversations() });
  },

  remove: async (id) => {
    await deleteConversation(id);
    await get().load();
  },
}));
