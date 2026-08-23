import { create } from "zustand";
import {
  deleteConversation,
  listConversations,
  renameConversation,
  setPinned,
} from "../lib/ipc";
import type { Conversation } from "../lib/types";

interface ConversationsState {
  items: Conversation[];
  load: () => Promise<void>;
  rename: (id: string, title: string) => Promise<void>;
  pin: (id: string, pinned: boolean) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useConversations = create<ConversationsState>((set, get) => ({
  items: [],

  load: async () => {
    set({ items: await listConversations() });
  },

  rename: async (id, title) => {
    await renameConversation(id, title);
    await get().load();
  },

  pin: async (id, pinned) => {
    await setPinned(id, pinned);
    await get().load();
  },

  remove: async (id) => {
    await deleteConversation(id);
    await get().load();
  },
}));
