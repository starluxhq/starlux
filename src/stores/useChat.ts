import { create } from "zustand";
import { cancelRun, listProviders, runPrompt } from "../lib/ipc";
import type { Provider, Usage } from "../lib/types";

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
  sessionId: string | null;
  turns: Turn[];
  status: Status;
  runId: string | null;
  loadProviders: () => Promise<void>;
  selectModel: (model: string | null) => void;
  send: (prompt: string) => Promise<void>;
  stop: () => Promise<void>;
  reset: () => void;
}

const newId = () => crypto.randomUUID();

export const useChat = create<ChatState>((set, get) => ({
  providers: [],
  providerId: "claude-cli",
  model: null,
  sessionId: null,
  turns: [],
  status: "idle",
  runId: null,

  loadProviders: async () => {
    const providers = await listProviders();
    set({ providers });
  },

  selectModel: (model) => set({ model }),

  reset: () => set({ turns: [], sessionId: null, status: "idle", runId: null }),

  stop: async () => {
    const { runId } = get();
    if (runId) await cancelRun(runId);
    set({ status: "idle", runId: null });
  },

  send: async (prompt) => {
    const trimmed = prompt.trim();
    if (!trimmed || get().status === "streaming") return;

    const runId = newId();
    const answerId = newId();
    const { providerId, model, sessionId } = get();

    set((state) => ({
      status: "streaming",
      runId,
      turns: [
        ...state.turns,
        { id: newId(), role: "user", text: trimmed },
        { id: answerId, role: "assistant", text: "", model },
      ],
    }));

    const patch = (change: Partial<Turn>) =>
      set((state) => ({
        turns: state.turns.map((turn) =>
          turn.id === answerId ? { ...turn, ...change } : turn,
        ),
      }));

    const append = (delta: string) =>
      set((state) => ({
        turns: state.turns.map((turn) =>
          turn.id === answerId ? { ...turn, text: turn.text + delta } : turn,
        ),
      }));

    try {
      await runPrompt(
        { runId, providerId, prompt: trimmed, sessionId, model },
        (event) => {
          switch (event.kind) {
            case "chunk":
              append(event.delta);
              break;
            case "meta":
              // The provider's own session id is what makes the next turn a
              // continuation rather than a fresh one-shot.
              if (event.sessionId) set({ sessionId: event.sessionId });
              if (event.model) patch({ model: event.model });
              break;
            case "end":
              patch({ text: event.text, usage: event.usage });
              if (event.sessionId) set({ sessionId: event.sessionId });
              set({ status: "idle", runId: null });
              break;
            case "error":
              patch({ error: event.message, stderrTail: event.stderrTail });
              set({ status: "error", runId: null });
              break;
          }
        },
      );
    } catch (error) {
      patch({ error: String(error) });
      set({ status: "error", runId: null });
    }
  },
}));
