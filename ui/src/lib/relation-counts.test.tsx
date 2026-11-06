import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

/** AJ-E3102: a page's counts come from the backend's relation counts,
 * once per entity, and are read again after a change settles. */
type Counts = { kind: string; id: string; sessions: { all: number; open: number; ended: number; archived: number } };
let all = 12;
const invoke = vi.fn(async (_command: string, args: { entities: { kind: string; id: string }[] }) =>
  args.entities.map((entity): Counts => ({ ...entity, sessions: { all, open: 1, ended: all - 1, archived: 0 } })),
);
const told: (() => void)[] = [];
const hear = (handler: () => void) => {
  told.push(handler);
  return () => undefined;
};
vi.mock("./ipc/transport", () => ({ call: (command: string, args: never) => invoke(command, args) }));
vi.mock("./bridge", () => ({ hasLiveBackend: () => true }));
vi.mock("./ipc/instances", () => ({ onSessionsUpdated: hear }));
vi.mock("./ipc/conversations", () => ({ onConversationsChanged: hear }));
vi.mock("./ipc/places", () => ({ onPlacesChanged: hear }));

afterEach(() => {
  invoke.mockClear();
  told.length = 0;
});

describe("useRelationCounts", () => {
  it("reads an entity's counts and again after a change", async () => {
    const { useRelationCounts } = await import("./relation-counts");
    const { result } = renderHook(() => useRelationCounts("project", "p-1"));
    expect(result.current).toBeUndefined();
    await waitFor(() => expect(result.current?.sessions.all).toBe(12));
    expect(invoke).toHaveBeenCalledWith("relations_counts", { entities: [{ kind: "project", id: "p-1" }] });
    expect(told).toHaveLength(3);
    all = 13;
    vi.useFakeTimers();
    act(() => {
      told[0]!();
      told[1]!();
    });
    act(() => void vi.advanceTimersByTime(300));
    vi.useRealTimers();
    await waitFor(() => expect(result.current?.sessions.all).toBe(13));
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("asks nothing without an entity", async () => {
    const { useRelationCounts } = await import("./relation-counts");
    const { result } = renderHook(() => useRelationCounts(null, "p-1"));
    expect(result.current).toBeUndefined();
    expect(invoke).not.toHaveBeenCalled();
  });
});
