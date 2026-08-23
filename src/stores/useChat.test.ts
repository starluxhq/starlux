import { beforeEach, describe, expect, it } from "vitest";
import { useChat } from "./useChat";
import type { RateLimit } from "../lib/types";

const RUN = "run-1";

const limit = (over: Partial<RateLimit> = {}): RateLimit => ({
  providerId: "claude-cli",
  kind: "five_hour",
  status: "allowed",
  resetsAt: 1_800_000_000,
  usingOverage: false,
  observedAt: 1_799_000_000,
  ...over,
});

const start = () =>
  useChat.getState().apply({
    kind: "start",
    runId: RUN,
    conversationId: "conv-1",
    providerId: "claude-cli",
    prompt: "hello",
  });

beforeEach(() => {
  useChat.setState({ turns: [], limits: {}, status: "idle", runId: null, sessionId: null });
});

describe("apply", () => {
  it("opens a run as a question and an empty answer", () => {
    start();
    const { turns, status, runId, conversationId } = useChat.getState();
    expect(turns.map((turn) => turn.role)).toEqual(["user", "assistant"]);
    expect(turns[0].text).toBe("hello");
    expect(turns[1].id).toBe(RUN);
    expect({ status, runId, conversationId }).toEqual({
      status: "streaming",
      runId: RUN,
      conversationId: "conv-1",
    });
  });

  it("settles the answering turn on end", () => {
    start();
    useChat.getState().apply({
      kind: "end",
      runId: RUN,
      text: "hi there",
      sessionId: "sess-1",
      usage: { inputTokens: 3, outputTokens: 4 },
    });
    const { turns, status, runId, sessionId } = useChat.getState();
    expect(turns[1].text).toBe("hi there");
    expect(turns[1].usage?.outputTokens).toBe(4);
    expect({ status, runId, sessionId }).toEqual({
      status: "idle",
      runId: null,
      sessionId: "sess-1",
    });
  });

  it("keeps the session when a run reports none", () => {
    useChat.setState({ sessionId: "sess-1" });
    start();
    useChat
      .getState()
      .apply({ kind: "end", runId: RUN, text: "hi", sessionId: null, usage: null });
    expect(useChat.getState().sessionId).toBe("sess-1");
  });

  it("puts the failure on the turn that was answering", () => {
    start();
    useChat
      .getState()
      .apply({ kind: "error", runId: RUN, message: "boom", stderrTail: "trace" });
    const { turns, status, runId } = useChat.getState();
    expect(turns).toHaveLength(2);
    expect(turns[1]).toMatchObject({ id: RUN, error: "boom", stderrTail: "trace" });
    expect({ status, runId }).toEqual({ status: "error", runId: null });
  });

  // Volunteered partway through a run, so it must not disturb the answer that
  // is still arriving.
  it("stores a rate limit under its provider and leaves the turns alone", () => {
    start();
    const before = useChat.getState().turns;
    useChat.getState().apply({ kind: "rateLimit", runId: RUN, limit: limit() });
    const { turns, limits, status } = useChat.getState();
    expect(turns).toBe(before);
    expect(status).toBe("streaming");
    expect(limits["claude-cli"].kind).toBe("five_hour");
  });

  it("replaces a provider's window rather than accumulating them", () => {
    const { apply } = useChat.getState();
    apply({ kind: "rateLimit", runId: RUN, limit: limit() });
    apply({ kind: "rateLimit", runId: RUN, limit: limit({ status: "rejected_hard" }) });
    apply({ kind: "rateLimit", runId: RUN, limit: limit({ providerId: "gemini-cli" }) });
    const { limits } = useChat.getState();
    expect(Object.keys(limits).sort()).toEqual(["claude-cli", "gemini-cli"]);
    expect(limits["claude-cli"].status).toBe("rejected_hard");
  });

  it("records what actually ran on the turn, not on the selection", () => {
    useChat.setState({ model: "opus" });
    start();
    useChat
      .getState()
      .apply({ kind: "meta", runId: RUN, sessionId: "sess-1", model: "claude-opus-5" });
    expect(useChat.getState().turns[1].model).toBe("claude-opus-5");
    expect(useChat.getState().model).toBe("opus");
  });
});
