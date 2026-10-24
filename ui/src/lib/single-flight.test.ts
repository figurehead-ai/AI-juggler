import { describe, expect, it } from "vitest";
import { singleFlight } from "./live-deltas";

describe("singleFlight", () => {
  it("a burst of calls while one runs gives one more run, not one per call", async () => {
    let runs = 0;
    let release: () => void = () => {};
    const gate = () => new Promise<void>((resolve) => (release = resolve));
    const refresh = singleFlight(async () => {
      runs += 1;
      if (runs === 1) await gate();
    });
    const first = refresh();
    for (let i = 0; i < 10; i += 1) void refresh();
    expect(runs).toBe(1);
    release();
    await first;
    expect(runs).toBe(2);
  });

  it("a call after the run ended starts a new one", async () => {
    let runs = 0;
    const refresh = singleFlight(async () => {
      runs += 1;
    });
    await refresh();
    await refresh();
    expect(runs).toBe(2);
  });

  it("a failing run doesn't keep it stuck", async () => {
    let runs = 0;
    const refresh = singleFlight(async () => {
      runs += 1;
      if (runs === 1) throw new Error("boom");
    });
    await expect(refresh()).rejects.toThrow("boom");
    await refresh();
    expect(runs).toBe(2);
  });
});
