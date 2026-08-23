import { beforeEach, describe, expect, it, vi } from "vitest";
import { currentContext, useChat } from "./useChat";
import type { Attachment, RateLimit } from "../lib/types";

const ipc = vi.hoisted(() => ({
  truncateAfter: vi.fn(() => Promise.resolve()),
  runPrompt: vi.fn(
    (_request: { runId: string; prompt: string; attachments?: string[] }) => Promise.resolve(),
  ),
  cancelRun: vi.fn(() => Promise.resolve(true)),
  listProviders: vi.fn(() => Promise.resolve([])),
  loadConversation: vi.fn(() => Promise.resolve(null)),
  rateLimits: vi.fn(() => Promise.resolve([])),
  saveSelectedModel: vi.fn(),
  selectedModel: vi.fn(() => Promise.resolve(null)),
  setAgentDir: vi.fn(() => Promise.resolve()),
  setWeb: vi.fn(() => Promise.resolve()),
}));

vi.mock("../lib/ipc", () => ipc);

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

const file = (name: string): Attachment => ({ path: `/home/a/${name}`, name });

const start = (attachments: Attachment[] = []) =>
  useChat.getState().apply({
    kind: "start",
    runId: RUN,
    conversationId: "conv-1",
    providerId: "claude-cli",
    prompt: "hello",
    attachments,
  });

beforeEach(() => {
  vi.clearAllMocks();
  useChat.setState({
    turns: [],
    limits: {},
    status: "idle",
    runId: null,
    sessionId: null,
    conversationId: null,
    web: false,
    agentDir: null,
  });
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

  it("hangs what was attached on the question, not the answer", () => {
    start([file("blue.png")]);
    const { turns } = useChat.getState();
    expect(turns[0].attachments).toEqual([file("blue.png")]);
    expect(turns[1].attachments).toBeUndefined();
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

describe("currentContext", () => {
  const turn = (id: string, context?: { used: number; window: number }) => ({
    id,
    role: "assistant" as const,
    text: "",
    usage: { inputTokens: 1, outputTokens: 1, ...(context && { context }) },
  });

  it("is null until an answer has reported one", () => {
    expect(currentContext([])).toBeNull();
    expect(currentContext([turn("a")])).toBeNull();
  });

  it("takes the most recent reading, which covers the most turns", () => {
    const turns = [
      turn("a", { used: 10, window: 100 }),
      turn("b", { used: 40, window: 100 }),
      turn("c"),
    ];
    expect(currentContext(turns)?.used).toBe(40);
  });
});

describe("grants", () => {
  // Stored against the conversation, not the window, so a grant revoked in one
  // window cannot be spent in the other.
  it("writes the web grant against the conversation and sends it with the run", async () => {
    useChat.setState({ conversationId: "conv-1" });
    await useChat.getState().setWeb(true);

    expect(ipc.setWeb).toHaveBeenCalledWith("conv-1", true);
    expect(useChat.getState().web).toBe(true);

    await useChat.getState().send("what shipped this week?");
    expect(ipc.runPrompt.mock.calls[0][0]).toMatchObject({ web: true });
  });

  // Nowhere to write it yet, so it travels with the run that opens the thread.
  it("carries the grant into a conversation that does not exist yet", async () => {
    await useChat.getState().setWeb(true);
    expect(ipc.setWeb).not.toHaveBeenCalled();

    await useChat.getState().send("hello");
    expect(ipc.runPrompt.mock.calls[0][0]).toMatchObject({ web: true });
  });

  it("starts a new conversation with neither grant", async () => {
    useChat.setState({ conversationId: "conv-1", web: true, agentDir: "/work" });
    useChat.getState().newConversation();
    const { web, agentDir } = useChat.getState();
    expect({ web, agentDir }).toEqual({ web: false, agentDir: null });
  });
});

describe("retry and edit", () => {
  const thread = () => {
    useChat.setState({
      conversationId: "conv-1",
      turns: [
        { id: "run-1:u", role: "user", text: "hello" },
        { id: "run-1", role: "assistant", text: "hi there" },
        { id: "run-2:u", role: "user", text: "and again" },
        { id: "run-2", role: "assistant", text: "still here" },
      ],
    });
  };

  const asked = () => {
    const { calls } = ipc.runPrompt.mock;
    return calls[calls.length - 1][0];
  };

  // The answer keeps its id, so the run that replaces it rewrites the row
  // rather than adding one below the turns that were dropped.
  it("asks the same question again under the answer's own id", async () => {
    thread();
    await useChat.getState().retry("run-1");

    expect(ipc.truncateAfter).toHaveBeenCalledWith("conv-1", "run-1");
    expect(asked()).toMatchObject({ runId: "run-1", prompt: "hello" });
    expect(useChat.getState().turns.map((turn) => turn.id)).toEqual(["run-1:u", "run-1"]);
  });

  it("re-runs an edited question in the place the old one held", async () => {
    thread();
    await useChat.getState().edit("run-1:u", "  ask it differently  ");

    expect(ipc.truncateAfter).toHaveBeenCalledWith("conv-1", "run-1:u");
    expect(asked()).toMatchObject({ runId: "run-1", prompt: "ask it differently" });
    const { turns } = useChat.getState();
    expect(turns.map((turn) => turn.id)).toEqual(["run-1:u", "run-1"]);
    expect(turns[0].text).toBe("ask it differently");
  });

  it("leaves the thread alone when there is nothing to act on", async () => {
    thread();
    await useChat.getState().edit("run-1:u", "   ");
    await useChat.getState().retry("nobody");

    expect(ipc.truncateAfter).not.toHaveBeenCalled();
    expect(ipc.runPrompt).not.toHaveBeenCalled();
    expect(useChat.getState().turns).toHaveLength(4);
  });

  // A retry that quietly dropped the screenshot would be asking a different
  // question and getting a fair answer to it.
  it("re-sends what was attached to the question it asks again", async () => {
    useChat.setState({
      conversationId: "conv-1",
      turns: [
        { id: "run-1:u", role: "user", text: "what is this?", attachments: [file("blue.png")] },
        { id: "run-1", role: "assistant", text: "blue" },
      ],
    });
    await useChat.getState().retry("run-1");

    expect(asked().attachments).toEqual(["/home/a/blue.png"]);
    expect(useChat.getState().turns[0].attachments).toEqual([file("blue.png")]);
  });

  it("keeps the files when the question itself is rewritten", async () => {
    useChat.setState({
      conversationId: "conv-1",
      turns: [
        { id: "run-1:u", role: "user", text: "what is this?", attachments: [file("blue.png")] },
        { id: "run-1", role: "assistant", text: "blue" },
      ],
    });
    await useChat.getState().edit("run-1:u", "and what shade?");

    expect(asked()).toMatchObject({
      prompt: "and what shade?",
      attachments: ["/home/a/blue.png"],
    });
  });

  // `send` refuses to start a second run while one is going, so a retry that
  // did not cancel first would simply do nothing.
  it("stops a run still going before starting the replacement", async () => {
    thread();
    useChat.setState({ status: "streaming", runId: "run-2" });
    await useChat.getState().retry("run-1");

    expect(ipc.cancelRun).toHaveBeenCalledWith("run-2");
    expect(ipc.runPrompt).toHaveBeenCalledOnce();
  });
});
