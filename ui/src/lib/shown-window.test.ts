import { afterEach, describe, expect, it, vi } from "vitest";
import { coalesced, everyWhileShown, nextDelay } from "./shown-window";

/** The backend's events as the page subscribes to them (one list of the
 * listeners in place). */
const live: string[] = [];
const handlers = new Map<number, { event: string; handler: (message: { payload: unknown }) => void }>();
let nextId = 0;
const emit = (event: string, payload: unknown) => {
  for (const entry of handlers.values()) if (entry.event === event) entry.handler({ payload });
};
vi.mock("./ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/transport")>()),
  listen: (event: string, handler: (message: { payload: unknown }) => void) => {
    const id = ++nextId;
    return Promise.resolve().then(() => {
      handlers.set(id, { event, handler });
      live.push(event);
      return () => {
        if (!handlers.delete(id)) return;
        live.splice(live.indexOf(event), 1);
      };
    });
  },
}));
import { keepIfSame, same } from "./same";

const setHidden = (hidden: boolean) => {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => (hidden ? "hidden" : "visible") });
  document.dispatchEvent(new Event("visibilitychange"));
};

afterEach(() => {
  vi.useRealTimers();
  setHidden(false);
});

describe("the window's own polling", () => {
  it("runs on its cadence when shown, slower unfocused, never hidden", () => {
    expect(nextDelay(2000, true, true, 4)).toBe(2000);
    expect(nextDelay(2000, true, false, 4)).toBe(8000);
    expect(nextDelay(2000, false, true, 4)).toBeNull();
  });

  it("pauses while hidden and runs once at once when shown again", () => {
    vi.useFakeTimers();
    vi.spyOn(document, "hasFocus").mockReturnValue(true);
    const run = vi.fn();
    const stop = everyWhileShown(run, 1000);
    vi.advanceTimersByTime(3000);
    expect(run).toHaveBeenCalledTimes(3);
    setHidden(true);
    vi.advanceTimersByTime(10_000);
    expect(run).toHaveBeenCalledTimes(3);
    setHidden(false);
    expect(run).toHaveBeenCalledTimes(4);
    vi.advanceTimersByTime(1000);
    expect(run).toHaveBeenCalledTimes(5);
    stop();
    vi.advanceTimersByTime(5000);
    expect(run).toHaveBeenCalledTimes(5);
  });

  // SK-24 (the 0.4.7 persona runs): a session page said "started 1s ago"
  // for a session minutes old. The shell's clock is this cadence (15 s,
  // 30 s unfocused), and each focus or blur started its wait again: a
  // window whose focus came and went every 10 s never ticked at all.
  it("keeps its cadence while focus comes and goes faster than it", () => {
    vi.useFakeTimers();
    let focused = false;
    const focus = vi.spyOn(document, "hasFocus").mockImplementation(() => focused);
    const run = vi.fn();
    const stop = everyWhileShown(run, 15_000, { unfocusedTimes: 2 });
    for (let step = 0; step < 12; step += 1) {
      vi.advanceTimersByTime(10_000);
      focused = !focused;
      window.dispatchEvent(new Event(focused ? "focus" : "blur"));
    }
    // Two minutes: at least one run every 30 s (the unfocused cadence).
    expect(run.mock.calls.length).toBeGreaterThanOrEqual(4);
    // A run is never more often than the focused cadence, however often focus moves.
    const count = run.mock.calls.length;
    for (let step = 0; step < 10; step += 1) {
      vi.advanceTimersByTime(1000);
      window.dispatchEvent(new Event(step % 2 === 0 ? "blur" : "focus"));
    }
    expect(run.mock.calls.length - count).toBeLessThanOrEqual(1);
    stop();
    focus.mockRestore();
  });
});

describe("a read asked for on every change (AJ-E805, round 3)", () => {
  // With 10 sessions at work the window read each list on every event;
  // a burst now makes one read at once and at most one more a second.
  it("runs the first ask at once and a burst's others once, a second later", () => {
    vi.useFakeTimers();
    const run = vi.fn();
    const reads = coalesced(run, 1000);
    reads.ask();
    expect(run).toHaveBeenCalledTimes(1);
    for (let i = 0; i < 9; i += 1) {
      vi.advanceTimersByTime(100);
      reads.ask();
    }
    expect(run).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(100);
    expect(run).toHaveBeenCalledTimes(2);
    vi.advanceTimersByTime(5000);
    expect(run).toHaveBeenCalledTimes(2);
    // After a quiet spell, at once again.
    reads.ask();
    expect(run).toHaveBeenCalledTimes(3);
    reads.ask();
    reads.stop();
    vi.advanceTimersByTime(5000);
    expect(run).toHaveBeenCalledTimes(3);
  });
});

describe("an equal answer changes nothing", () => {
  it("compares the backend's answers by value", () => {
    expect(same({ a: [1, { b: "x" }], c: null }, { c: null, a: [1, { b: "x" }] })).toBe(true);
    expect(same([1, 2], [1, 2, 3])).toBe(false);
    expect(same({ a: 1 }, { a: 1, b: undefined })).toBe(false);
    const was = [{ id: "s1", state: "working" }];
    expect(keepIfSame(was, [{ id: "s1", state: "working" }])).toBe(was);
    const next = [{ id: "s1", state: "asks" }];
    expect(keepIfSame(was, next)).toBe(next);
  });
});

describe("a backend event while the window is hidden (AJ-E805)", () => {
  // Every event reaches the page as a script WebKit runs at foreground
  // priority: a hidden window's processes were raised and dropped for each
  // one (about 60 a minute with 10 sessions at work). Hidden, the page
  // holds no listener, so the backend sends it nothing; shown, it listens
  // again and then reads what it missed, once.
  it("holds no listener while hidden and catches up once it listens again", async () => {
    // The setup file loaded this module with the real transport: again,
    // with the stand-in above.
    vi.resetModules();
    const { listenWhileShown } = await import("./shown-window");
    const heard: string[] = [];
    let caught = 0;
    const stop = listenWhileShown<string>("conversations://changed", (payload) => heard.push(payload), () => (caught += 1));
    await vi.waitFor(() => expect(live).toEqual(["conversations://changed"]));
    emit("conversations://changed", "live");
    expect(heard).toEqual(["live"]);
    setHidden(true);
    expect(live).toEqual([]);
    emit("conversations://changed", "index");
    expect(heard).toEqual(["live"]);
    expect(caught).toBe(0);
    setHidden(false);
    await vi.waitFor(() => expect(caught).toBe(1));
    expect(live).toEqual(["conversations://changed"]);
    emit("conversations://changed", "index");
    expect(heard).toEqual(["live", "index"]);
    // Hidden and shown again before the listen lands: one listener, one catch-up.
    setHidden(true);
    setHidden(false);
    setHidden(true);
    setHidden(false);
    await vi.waitFor(() => expect(caught).toBe(2));
    await Promise.resolve();
    expect(live).toEqual(["conversations://changed"]);
    stop();
    expect(live).toEqual([]);
    setHidden(true);
    setHidden(false);
    await Promise.resolve();
    expect(caught).toBe(2);
    expect(live).toEqual([]);
  });
});
