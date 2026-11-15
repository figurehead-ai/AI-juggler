import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

/** The events the page listens to, as it asks for them. */
const listened: string[] = [];
vi.mock("./ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/transport")>()),
  liveBackend: true,
  call: async () => ({ processes: 0, sessions: 0, attention: 0, to_clean: 2, left_behind: 2, zombies: 0, failed: 0 }),
  listen: async (event: string) => {
    listened.push(event);
    return () => undefined;
  },
}));
vi.mock("./bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("./bridge")>()), hasLiveBackend: () => true }));

describe("the top bar's count of what is left to clean up (AJ-E805)", () => {
  // The graph's event goes out on nearly every 2 s tick while sessions
  // work; the count listened to it, so the window was sent every change.
  // It listens to the counts' own event, told when a count moves.
  it("listens to the counts, not to every change of the graph", async () => {
    // The setup file loaded these with the real transport: again, with the stand-in.
    vi.resetModules();
    const { useCleanupCount } = await import("./cleanup-count");
    const { result } = renderHook(() => useCleanupCount());
    await waitFor(() => expect(result.current).toBe(2));
    await waitFor(() => expect(listened).toEqual(["processes://counts"]));
  });
});
