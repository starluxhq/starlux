import { cleanup, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ContextMeter from "./ContextMeter";

const meter = (used: number, window = 200_000) => {
  cleanup();
  return render(<ContextMeter context={{ used, window }} />);
};

describe("ContextMeter", () => {
  it("reports the share of the window the conversation occupies", () => {
    meter(26_829);
    expect(screen.getByText("13%")).toBeTruthy();
  });

  it("puts the exact figures one hover away", () => {
    meter(26_829);
    const title = screen.getByText("13%").title;
    expect(title).toContain((26_829).toLocaleString());
    expect(title).toContain((200_000).toLocaleString());
  });

  // A compacted conversation can report more carried than the window holds;
  // "104%" reads as a bug rather than as a full window.
  it("never claims more than a full window", () => {
    meter(260_000);
    expect(screen.getByText("100%")).toBeTruthy();
  });

  it("marks a crowded window and leaves a roomy one quiet", () => {
    const quiet = meter(100_000).container.querySelector("span")!.className;
    const crowded = meter(170_000).container.querySelector("span")!.className;
    expect(quiet).toContain("text-faint");
    expect(crowded).toContain("text-class-m");
  });
});
