import { create } from "zustand";
import {
  cancelRun,
  listProviders,
  loadConversation,
  rateLimits as loadRateLimits,
  runPrompt,
  saveSelectedModel,
  selectedModel,
  setAgentDir as saveAgentDir,
  truncateAfter,
} from "../lib/ipc";
import {
  isReady,
  type Attachment,
  type Context,
  type Message,
  type Provider,
  type RateLimit,
  type StreamEvent,
  type Usage,
} from "../lib/types";

export type Status = "idle" | "streaming" | "error";

export interface Turn {
  id: string;
  role: "user" | "assistant";
  text: string;
  model?: string | null;
  usage?: Usage | null;
  error?: string;
  stderrTail?: string;
  /** Carried on the question so asking it again sends the same files. A retry
   *  that quietly dropped a screenshot would change what the answer means. */
  attachments?: Attachment[];
}

interface ChatState {
  providers: Provider[];
  /** Keyed by provider: each reports its own window, and some report none. */
  limits: Record<string, RateLimit>;
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
  send: (prompt: string, files?: Attachment[]) => Promise<void>;
  retry: (id: string) => Promise<void>;
  edit: (id: string, prompt: string) => Promise<void>;
  stop: () => Promise<void>;
}

/** Ids are shared with the database so a loaded thread and a live run merge. */
const questionId = (runId: string) => `${runId}:u`;
const runOf = (id: string) => id.replace(/:u$/, "");

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
  attachments: message.attachments,
});

export const useChat = create<ChatState>((set, get) => ({
  providers: [],
  limits: {},
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
    const [providers, saved, limits] = await Promise.all([
      listProviders(),
      selectedModel(),
      loadRateLimits(),
    ]);
    const usable = providers.filter(isReady);
    const chosen =
      usable.find(
        (provider) => provider.id === saved?.providerId && provider.models.includes(saved.model),
      ) ?? usable[0];

    set({
      providers,
      limits: Object.fromEntries(limits.map((limit) => [limit.providerId, limit])),
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
            attachments: event.attachments,
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
        // Volunteered by the provider partway through a run, so it lands
        // whether or not the answer does.
        case "rateLimit":
          return { limits: { ...state.limits, [event.limit.providerId]: event.limit } };
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

  send: async (prompt, files = []) => {
    const trimmed = prompt.trim();
    if (!trimmed || get().status === "streaming") return;
    await dispatch(crypto.randomUUID(), trimmed, files);
  },

  // Asking again under the answer's own id. `add_message` upserts on that id
  // without touching `created_at`, so the answer is rewritten where it stands
  // rather than appended below the turns that came after it.
  //
  // The provider's session is kept. A CLI session can only be abandoned, never
  // rewound, and abandoning it would cost the whole conversation's context —
  // so the model still remembers the discarded exchange, and reads this as
  // being asked to answer again.
  retry: async (id) => {
    const { turns, conversationId } = get();
    const at = turns.findIndex((turn) => turn.id === id);
    const question = turns[at - 1];
    if (at < 1 || !conversationId || question.role !== "user") return;

    await get().stop();
    await truncateAfter(conversationId, id);
    set({ turns: turns.slice(0, at + 1) });
    await dispatch(id, question.text, question.attachments ?? []);
  },

  // The question keeps its id too: `run_prompt` writes it as `{runId}:u`, the
  // same id it was minted with, so a re-run rewrites it in place.
  edit: async (id, prompt) => {
    const trimmed = prompt.trim();
    const { turns, conversationId } = get();
    const at = turns.findIndex((turn) => turn.id === id);
    if (!trimmed || at < 0 || !conversationId) return;

    await get().stop();
    await truncateAfter(conversationId, id);
    set({ turns: turns.slice(0, at + 1) });
    await dispatch(runOf(id), trimmed, turns[at].attachments ?? []);
  },
}));

/** One run, under an id the caller chooses — new for a question just asked,
 *  the old one for a question being asked again. */
async function dispatch(runId: string, prompt: string, files: Attachment[]) {
  const { conversationId, providerId, model, sessionId, agentDir, apply } = useChat.getState();
  const id = conversationId ?? crypto.randomUUID();

  // Shown right away; the run replaces them with what the core actually read,
  // which is where a file's size and type come from.
  apply({ kind: "start", runId, conversationId: id, providerId, prompt, attachments: files });

  try {
    await runPrompt(
      {
        runId,
        conversationId: id,
        providerId,
        prompt,
        sessionId,
        model,
        agentDir,
        attachments: files.map((file) => file.path),
      },
      apply,
    );
  } catch (error) {
    apply({ kind: "error", runId, message: String(error), stderrTail: "" });
  }
}

/** How full the conversation is now: the most recent answer that said so. An
 *  older turn's reading is not stale, it is simply about fewer turns than the
 *  thread now holds. */
export function currentContext(turns: Turn[]): Context | null {
  for (let at = turns.length - 1; at >= 0; at--) {
    const context = turns[at].usage?.context;
    if (context) return context;
  }
  return null;
}

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
