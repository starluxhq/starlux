import { describe, expect, it } from "vitest";
import { resetLabel, shortAge, windowLabel } from "./time";

const AT = Date.UTC(2026, 7, 23, 12, 0, 0);
const seconds = (ms: number) => ms / 1000;

describe("windowLabel", () => {
  it("names the windows Anthropic reports", () => {
    expect(windowLabel("five_hour")).toBe("5-hour");
    expect(windowLabel("seven_day")).toBe("weekly");
  });

  it("shows a kind it has never seen rather than dropping it", () => {
    expect(windowLabel("lunar_cycle")).toBe("lunar cycle");
  });
});

describe("resetLabel", () => {
  it("is null once the reset is behind us", () => {
    expect(resetLabel(seconds(AT - 60_000), AT)).toBeNull();
    expect(resetLabel(seconds(AT), AT)).toBeNull();
  });

  it("gives a time for a reset still ahead", () => {
    expect(resetLabel(seconds(AT + 90 * 60_000), AT)).toBeTruthy();
  });

  it("adds a weekday only when the reset is not today", () => {
    const today = resetLabel(seconds(AT + 60 * 60_000), AT)!;
    const tomorrow = resetLabel(seconds(AT + 26 * 60 * 60_000), AT)!;
    expect(today.split(" ").length).toBeLessThan(tomorrow.split(" ").length);
  });
});

describe("shortAge", () => {
  it("counts up through the units", () => {
    expect(shortAge(AT - 30_000, AT)).toBe("now");
    expect(shortAge(AT - 5 * 60_000, AT)).toBe("5m");
    expect(shortAge(AT - 3 * 3_600_000, AT)).toBe("3h");
    expect(shortAge(AT - 2 * 86_400_000, AT)).toBe("2d");
  });

  it("never reports a future timestamp as an age", () => {
    expect(shortAge(AT + 60_000, AT)).toBe("now");
  });
});
