import { create } from "zustand";
import {
  cancelRun,
  listProviders,
  loadConversation,
  runPrompt,
  saveSelectedModel,
  selectedModel,
  setAgentDir as saveAgentDir,
} from "../lib/ipc";
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
  agentDir: string | null;
  turns: Turn[];
  status: Status;
  runId: string | null;
  loadProviders: () => Promise<void>;
  selectModel: (providerId: string, model: string) => void;
  setAgentDir: (dir: string | null) => Promise<void>;
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

const pending = new Map<string, string>();
let frame = 0;

function flush() {
  frame = 0;
  if (pending.size === 0) return;
  const batch = new Map(pending);
  pending.clear();
  useChat.setState((state) => ({
    turns: state.turns.map((turn) => {
      const delta = batch.get(turn.id);
      return delta === undefined ? turn : { ...turn, text: turn.text + delta };
    }),
  }));
}

/** Deltas land every few characters; re-parsing the markdown that often is what
 *  makes an answer tear while it streams. */
function queue(runId: string, delta: string) {
  pending.set(runId, (pending.get(runId) ?? "") + delta);
  frame ||= requestAnimationFrame(flush);
}

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
  agentDir: null,
  turns: [],
  status: "idle",
  runId: null,

  // The model outlives the conversation: whatever was asked for last is what
  // the next question asks for, on this launch or the next one. A saved choice
  // whose provider or model has since gone is dropped rather than sent.
  loadProviders: async () => {
    const [providers, saved] = await Promise.all([listProviders(), selectedModel()]);
    const usable = providers.filter((provider) => provider.available);
    const chosen =
      usable.find(
        (provider) => provider.id === saved?.providerId && provider.models.includes(saved.model),
      ) ?? usable[0];

    set({
      providers,
      ...(chosen && {
        providerId: chosen.id,
        model: chosen.models.includes(saved?.model ?? "") ? saved!.model : chosen.models[0],
      }),
    });
  },

  // Provider and model move together: the list spans every provider, so
  // picking one that belongs to another is also a switch of provider.
  selectModel: (providerId, model) => {
    set({ providerId, model });
    void saveSelectedModel(providerId, model);
  },

  // Stored against the conversation, not the window, so the folder a run may
  // touch is the one the user granted rather than the one this window last saw.
  setAgentDir: async (dir) => {
    const { conversationId } = get();
    if (conversationId) await saveAgentDir(conversationId, dir);
    set({ agentDir: dir });
  },

  newConversation: () =>
    set({
      conversationId: null,
      sessionId: null,
      agentDir: null,
      turns: [],
      status: "idle",
      runId: null,
    }),

  apply: (event) => {
    if (event.kind === "chunk") {
      queue(event.runId, event.delta);
      return;
    }
    // The final text is authoritative, so anything still buffered would only
    // append itself a second time.
    if (event.kind === "end" || event.kind === "error") pending.delete(event.runId);

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
        case "meta":
          // The provider's own session id is what makes the next turn a
          // continuation rather than a fresh one-shot.
          //
          // What ran is recorded on the turn, never back onto the selection:
          // the provider answers with an exact build (`claude-opus-5`) and the
          // picker offers aliases (`opus`), so writing one over the other
          // leaves the list with nothing selected.
          return {
            sessionId: event.sessionId ?? state.sessionId,
            turns: event.model
              ? patch(state.turns, event.runId, { model: event.model })
              : state.turns,
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
    });
  },

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
        agentDir: thread.conversation.agentDir,
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
    const { providerId, model, sessionId, agentDir, apply } = get();

    apply({ kind: "start", runId, conversationId, providerId, prompt: trimmed });

    try {
      await runPrompt(
        { runId, conversationId, providerId, prompt: trimmed, sessionId, model, agentDir },
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
