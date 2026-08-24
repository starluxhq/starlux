import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSettings } from "./useSettings";

const ipc = vi.hoisted(() => ({
  tools: vi.fn(() => Promise.resolve({ webSearch: true, webFetch: false })),
  setTool: vi.fn(() => Promise.resolve({ webSearch: true, webFetch: true })),
}));

vi.mock("../lib/ipc", () => ipc);

beforeEach(() => {
  vi.clearAllMocks();
  useSettings.setState({ tools: { webSearch: false, webFetch: false } });
});

describe("useSettings", () => {
  it("starts with nothing granted", () => {
    expect(useSettings.getState().tools).toEqual({ webSearch: false, webFetch: false });
  });

  it("reads what the core says the grant is", async () => {
    await useSettings.getState().loadTools();
    expect(useSettings.getState().tools).toEqual({ webSearch: true, webFetch: false });
  });

  // The core answers with the whole grant, not the bit that changed, so what
  // lands here is what the next run will actually be given.
  it("takes the core's answer rather than assuming its own change stuck", async () => {
    await useSettings.getState().setTool("webFetch", true);

    expect(ipc.setTool).toHaveBeenCalledWith("webFetch", true);
    expect(useSettings.getState().tools).toEqual({ webSearch: true, webFetch: true });
  });

  // Applied without writing it back, or the window told about a grant would
  // tell the other one in turn and the two would answer each other forever.
  it("adopts the other window's grant without writing it back", () => {
    useSettings.getState().adoptTools({ webSearch: false, webFetch: true });

    expect(useSettings.getState().tools).toEqual({ webSearch: false, webFetch: true });
    expect(ipc.setTool).not.toHaveBeenCalled();
  });
});
