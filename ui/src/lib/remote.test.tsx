import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

/** What the backend is asked, and the change listeners it was given. */
const calls: Array<{ name: string; args: unknown }> = [];
const heard = new Map<string, Array<(message: { payload: null }) => void>>();
let flags: Record<string, boolean> = { notices_on: true };

vi.mock("./ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/transport")>()),
  liveBackend: true,
  call: async (name: string, args: unknown) => {
    calls.push({ name, args });
    if (name === "remote_config_values") return { section: "flags", flags: { ...flags } };
    throw new Error(`unexpected ${name}`);
  },
  listen: async (event: string, handler: (message: { payload: null }) => void) => {
    heard.set(event, [...(heard.get(event) ?? []), handler]);
    return () => heard.set(event, (heard.get(event) ?? []).filter((h) => h !== handler));
  },
}));
vi.mock("./bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("./bridge")>()), hasLiveBackend: () => true }));

beforeEach(() => {
  calls.length = 0;
  heard.clear();
  flags = { notices_on: true };
  vi.useRealTimers();
});

describe("useRemote(section) (ADR-0035 §9, AJ-E2023)", () => {
  it("reads nothing until a view asks", async () => {
    vi.resetModules();
    await import("./remote");
    expect(calls).toEqual([]);
  });

  it("answers a section typed by its name, one read shared by every view", async () => {
    vi.resetModules();
    const { useRemote } = await import("./remote");
    const one = renderHook(() => useRemote("flags"));
    const two = renderHook(() => useRemote("flags"));
    expect(one.result.current).toBeNull();
    await waitFor(() => expect(one.result.current?.flags.notices_on).toBe(true));
    await waitFor(() => expect(two.result.current?.flags.notices_on).toBe(true));
    expect(calls).toEqual([{ name: "remote_config_values", args: { section: "flags" } }]);
    // A view opened again paints the kept answer at once, with no new read.
    const three = renderHook(() => useRemote("flags"));
    expect(three.result.current?.section).toBe("flags");
    expect(calls).toHaveLength(1);
  });

  it("is read again when the backend says it changed, never on a timer", async () => {
    vi.resetModules();
    const { useRemote } = await import("./remote");
    const view = renderHook(() => useRemote("flags"));
    await waitFor(() => expect(view.result.current?.flags.notices_on).toBe(true));
    await waitFor(() => expect(heard.get("remote-config://changed")).toHaveLength(1));
    // Time alone reads nothing.
    vi.useFakeTimers();
    await act(async () => void vi.advanceTimersByTime(24 * 60 * 60 * 1000));
    vi.useRealTimers();
    expect(calls).toHaveLength(1);
    // The change event does.
    flags = { notices_on: false };
    act(() => heard.get("remote-config://changed")?.forEach((on) => on({ payload: null })));
    await waitFor(() => expect(view.result.current?.flags.notices_on).toBe(false));
    expect(calls).toHaveLength(2);
  });
});
