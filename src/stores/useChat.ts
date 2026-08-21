import { create } from "zustand";
import { cancelRun, listProviders, loadConversation, runPrompt } from "../lib/ipc";
import type { Message, Provider, StreamEvent, Usage } from "../lib/types";

export type Status = "idle" | "streaming" | "error";

export interface Turn {
  id: string;
  role: "user" | "assistant";
  text: string;
  model?: string | null;
  usage?: Usage | null;
  error?: string;
  stderrTail?: string;
}

interface ChatState {
  providers: Provider[];
  providerId: string;
  model: string | null;
  conversationId: string | null;
  sessionId: string | null;
  turns: Turn[];
  status: Status;
  runId: string | null;
  loadProviders: () => Promise<void>;
  selectModel: (model: string | null) => void;
  apply: (event: StreamEvent) => void;
  openConversation: (id: string) => Promise<void>;
  newConversation: () => void;
  send: (prompt: string) => Promise<void>;
  stop: () => Promise<void>;
}

/** Ids are shared with the database so a loaded thread and a live run merge. */
const questionId = (runId: string) => `${runId}:u`;

const upsert = (turns: Turn[], turn: Turn): Turn[] =>
  turns.some((existing) => existing.id === turn.id)
    ? turns.map((existing) => (existing.id === turn.id ? { ...existing, ...turn } : existing))
    : [...turns, turn];

const patch = (turns: Turn[], id: string, change: Partial<Turn>): Turn[] =>
  turns.map((turn) => (turn.id === id ? { ...turn, ...change } : turn));

const toTurn = (message: Message): Turn => ({
  id: message.id,
  role: message.role,
  text: message.text,
  model: message.model,
  usage: message.usage,
  error: message.error ?? undefined,
});

export const useChat = create<ChatState>((set, get) => ({
  providers: [],
  providerId: "claude-cli",
  model: null,
  conversationId: null,
  sessionId: null,
  turns: [],
  status: "idle",
  runId: null,

  loadProviders: async () => {
    const providers = await listProviders();
    set({ providers });
  },

  selectModel: (model) => set({ model }),

  newConversation: () =>
    set({ conversationId: null, sessionId: null, turns: [], status: "idle", runId: null }),

  apply: (event) =>
    set((state) => {
      switch (event.kind) {
        case "start": {
          const withQuestion = upsert(state.turns, {
            id: questionId(event.runId),
            role: "user",
            text: event.prompt,
          });
          return {
            conversationId: event.conversationId,
            providerId: event.providerId,
            runId: event.runId,
            status: "streaming",
            turns: upsert(withQuestion, {
              id: event.runId,
              role: "assistant",
              text: "",
              model: state.model,
            }),
          };
        }
        case "chunk": {
          const answer = state.turns.find((turn) => turn.id === event.runId);
          return {
            turns: patch(state.turns, event.runId, { text: (answer?.text ?? "") + event.delta }),
          };
        }
        case "meta":
          // The provider's own session id is what makes the next turn a
          // continuation rather than a fresh one-shot.
          return {
            sessionId: event.sessionId ?? state.sessionId,
            model: event.model ?? state.model,
            turns: event.model ? patch(state.turns, event.runId, { model: event.model }) : state.turns,
          };
        case "end":
          return {
            sessionId: event.sessionId ?? state.sessionId,
            status: "idle",
            runId: null,
            turns: patch(state.turns, event.runId, { text: event.text, usage: event.usage }),
          };
        case "error":
          return {
            status: "error",
            runId: null,
            turns: upsert(state.turns, {
              id: event.runId,
              role: "assistant",
              text: "",
              error: event.message,
              stderrTail: event.stderrTail,
            }),
          };
      }
    }),

  openConversation: async (id) => {
    const thread = await loadConversation(id);
    if (!thread) return;
    set((state) => {
      const turns = thread.messages.map(toTurn);
      // Keep anything the database has not caught up with yet, which is how a
      // run still streaming survives its own history load.
      if (state.conversationId === id) {
        for (const turn of state.turns) {
          if (!turns.some((loaded) => loaded.id === turn.id)) turns.push(turn);
        }
      }
      return {
        conversationId: id,
        providerId: thread.conversation.providerId,
        sessionId: thread.conversation.sessionId,
        model: thread.conversation.model ?? state.model,
        turns,
      };
    });
  },

  stop: async () => {
    const { runId } = get();
    if (runId) await cancelRun(runId);
    set({ status: "idle" });
  },

  send: async (prompt) => {
    const trimmed = prompt.trim();
    if (!trimmed || get().status === "streaming") return;

    const runId = crypto.randomUUID();
    const conversationId = get().conversationId ?? crypto.randomUUID();
    const { providerId, model, sessionId, apply } = get();

    apply({ kind: "start", runId, conversationId, providerId, prompt: trimmed });

    try {
      await runPrompt(
        { runId, conversationId, providerId, prompt: trimmed, sessionId, model },
        apply,
      );
    } catch (error) {
      apply({ kind: "error", runId, message: String(error), stderrTail: "" });
    }
  },
}));

/** Applies a run owned by the other window, pulling in its history if it is a
 *  thread this window was not already showing. */
export function applyMirrored(event: StreamEvent) {
  const store = useChat.getState();
  if (event.kind === "start" && store.conversationId !== event.conversationId) {
    // The thread on screen is dropped before the new turns land, so the old one
    // is never briefly shown with the new question appended to it.
    store.newConversation();
    useChat.getState().apply(event);
    void useChat.getState().openConversation(event.conversationId);
    return;
  }
  store.apply(event);
}
