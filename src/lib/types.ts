export interface Usage {
  inputTokens: number;
  outputTokens: number;
  costUsd?: number;
}

export type StreamEvent =
  | { kind: "start"; runId: string; conversationId: string; providerId: string; prompt: string }
  | { kind: "chunk"; runId: string; delta: string }
  | { kind: "meta"; runId: string; sessionId: string | null; model: string | null }
  | { kind: "end"; runId: string; text: string; sessionId: string | null; usage: Usage | null }
  | { kind: "error"; runId: string; message: string; stderrTail: string };

export interface RunRequest {
  runId: string;
  conversationId: string;
  providerId: string;
  prompt: string;
  sessionId?: string | null;
  model?: string | null;
  agentDir?: string | null;
}

export interface Conversation {
  id: string;
  title: string;
  providerId: string;
  sessionId: string | null;
  model: string | null;
  agentDir: string | null;
  updatedAt: number;
}

export interface Message {
  id: string;
  role: "user" | "assistant";
  text: string;
  model: string | null;
  usage: Usage | null;
  error: string | null;
}

export interface Thread {
  conversation: Conversation;
  messages: Message[];
}

export interface Provider {
  id: string;
  name: string;
  binary: string;
  available: boolean;
  models: string[];
}

export type SpectralClass = "a" | "f" | "g" | "k" | "m";

const SPECTRAL_CLASSES: Record<string, SpectralClass> = {
  "claude-cli": "k",
  "gemini-cli": "a",
  "openai-api": "f",
  "ollama-local": "g",
};

export const spectralClass = (providerId: string): SpectralClass =>
  SPECTRAL_CLASSES[providerId] ?? "f";
