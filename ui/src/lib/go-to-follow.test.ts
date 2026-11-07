/**
 * The go sitting (2026-11-07, finding 3): Start on a profile started it in
 * seconds, but "Starting Claude app · aij-test" stayed for minutes and its
 * row kept Start. A start is now followed: "… is running" as soon as it
 * runs (and the places read again), or after 30 s that it didn't start.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { KnownPlaceInfo } from "./ipc/places";

const rows: { now: KnownPlaceInfo[] } = { now: [] };
vi.mock("./ipc/places", async (actual) => ({
  ...(await actual<typeof import("./ipc/places")>()),
  placesList: vi.fn(async () => rows.now),
}));
const refreshed = vi.fn(async () => undefined);
vi.mock("./places", async (actual) => ({ ...(await actual<typeof import("./places")>()), refreshKnownPlaces: () => refreshed() }));

const { showGoTo, FOLLOW_FOR } = await import("./go-to");
const { resetToasts, toastEntries } = await import("../components/Toast");

const place = (pid: number | null, cannot: string | null = null): KnownPlaceInfo =>
  ({ id: "app:claude-app:/p/aij-test", app_id: "claude-app", app: "Claude", instance: "aij-test", profile: "aij-test", label: "Claude · aij-test", app_pid: pid, lost: false, keep_alive: false, paused: null, account_id: null, seen_ms: 0, cannot_start: cannot, recent: [], found: false });
const toast = () => toastEntries()[0]!.options;

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  resetToasts();
  refreshed.mockClear();
});

describe("a start is followed to its end", () => {
  it("says Starting…, then that it runs as soon as it does, and reads the places again", async () => {
    rows.now = [place(null)];
    showGoTo({ outcome: "Starting Claude app · aij-test", reached: "started", place_id: "app:claude-app:/p/aij-test" });
    expect(toast().message).toBe("Starting Claude app · aij-test");
    expect(toast().tone).toBeUndefined();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(toast().message).toBe("Starting Claude app · aij-test");
    rows.now = [place(4242)];
    await vi.advanceTimersByTimeAsync(1_000);
    expect(toast()).toMatchObject({ message: "Claude app · aij-test is running", tone: "success" });
    expect(refreshed).toHaveBeenCalledTimes(1);
    // Followed no further.
    await vi.advanceTimersByTimeAsync(10_000);
    expect(refreshed).toHaveBeenCalledTimes(1);
  });

  it("after 30 seconds not running, says it didn't start, with why when it's known", async () => {
    rows.now = [place(null, "Its folder was removed")];
    showGoTo({ outcome: "Starting Claude app · aij-test", reached: "started", place_id: "app:claude-app:/p/aij-test" });
    await vi.advanceTimersByTimeAsync(FOLLOW_FOR * 1_000);
    expect(toast()).toMatchObject({ message: "Claude app · aij-test didn't start", detail: "Its folder was removed", tone: "error" });
    expect(refreshed).toHaveBeenCalledTimes(1);
  });

  it("an outcome without the place's id is said as it is", () => {
    showGoTo({ outcome: "Starting Claude app · aij-test", reached: "started" });
    expect(toast().testId).toBeUndefined();
  });
});
