export interface Usage {
  inputTokens: number;
  outputTokens: number;
  costUsd?: number;
  context?: Context;
}

/** How much of the model's window the conversation now occupies. Both halves
 *  are the provider's own numbers, so this is arithmetic rather than a guess. */
export interface Context {
  used: number;
  window: number;
}

/** The provider's view of the user's whole subscription window — every session
 *  they have run, terminal included — not Starlux's share of it. */
export interface RateLimit {
  providerId: string;
  /** The provider's own name for the window (`five_hour`, `weekly`, ...). */
  kind: string;
  status: string;
  resetsAt: number | null;
  usingOverage: boolean;
  /** When Starlux saw this, not when the provider computed it. */
  observedAt: number;
}

/** What was attached to a question — a description, never the contents. The
 *  core reads the files itself, under its own size cap. */
export interface Attachment {
  path: string;
  name: string;
  mime?: string | null;
  bytes?: number | null;
}

export type StreamEvent =
  | {
      kind: "start";
      runId: string;
      conversationId: string;
      providerId: string;
      prompt: string;
      attachments: Attachment[];
    }
  | { kind: "chunk"; runId: string; delta: string }
  | { kind: "meta"; runId: string; sessionId: string | null; model: string | null }
  | { kind: "end"; runId: string; text: string; sessionId: string | null; usage: Usage | null }
  | { kind: "error"; runId: string; message: string; stderrTail: string }
  | { kind: "rateLimit"; runId: string; limit: RateLimit };

export interface RunRequest {
  runId: string;
  conversationId: string;
  providerId: string;
  prompt: string;
  sessionId?: string | null;
  model?: string | null;
  effort?: string | null;
  agentDir?: string | null;
  attachments?: string[];
}

/** What every run may reach beyond the model itself. One answer for the whole
 *  app, so a question asked from the bar reaches exactly what one asked from
 *  the Workspace does. Searching and fetching are granted separately because
 *  the providers grant them separately. */
export interface Tools {
  webSearch: boolean;
  webFetch: boolean;
}

export type ToolId = keyof Tools;

export interface Conversation {
  id: string;
  title: string;
  providerId: string;
  sessionId: string | null;
  model: string | null;
  agentDir: string | null;
  updatedAt: number;
  pinned: boolean;
}

export interface Message {
  id: string;
  role: "user" | "assistant";
  text: string;
  model: string | null;
  usage: Usage | null;
  error: string | null;
  attachments: Attachment[];
}

export interface Thread {
  conversation: Conversation;
  messages: Message[];
}

export interface Selection {
  providerId: string;
  model: string;
  effort?: string | null;
}

/** A model and how hard it can be asked to think. The levels are the model's
 *  own: `opencode-go/gpt-5.6-luna` offers six, `opencode-go/kimi-k3` offers
 *  one, and `opencode-go/minimax-m3` offers `none` and `thinking`, which is
 *  not a ladder. Empty where the model offers no choice, or the CLI has no
 *  flag to carry one. */
export interface Model {
  id: string;
  efforts: string[];
}

/** Installed and signed in are different problems with different fixes, so a
 *  provider nobody is signed in to is not reported as absent. */
export type Availability =
  | { state: "missing" }
  | { state: "signedOut" }
  | { state: "ready"; plan: string | null };

export interface Provider {
  id: string;
  name: string;
  binary: string;
  /** What to run to sign in, in full — `opencode login` is not a command. */
  login: string;
  availability: Availability;
  models: Model[];
  /** Which of Starlux's tools this CLI has to offer. Not every provider has
   *  every one, so a tool granted app-wide is still only reached where it
   *  exists. */
  tools: ToolId[];
}

export const isReady = (provider: Provider) => provider.availability.state === "ready";

/** What the chosen model can be asked for, which is nothing at all for most of
 *  them. Reached by id rather than by index: the picker holds the id it was
 *  given, and the list under it is refetched. */
export const effortsOf = (provider: Provider | undefined, model: string | null): string[] =>
  provider?.models.find((known) => known.id === model)?.efforts ?? [];

export type SpectralClass = "a" | "f" | "g" | "k" | "m";

const SPECTRAL_CLASSES: Record<string, SpectralClass> = {
  "claude-cli": "k",
  "opencode-cli": "g",
  "gemini-cli": "a",
  "openai-api": "f",
  "ollama-local": "g",
};

export const spectralClass = (providerId: string): SpectralClass =>
  SPECTRAL_CLASSES[providerId] ?? "f";
