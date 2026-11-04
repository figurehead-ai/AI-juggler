/**
 * AJ-E3510: a door on the Floor (or a notification) opens the right place
 * in the main window.
 */
import { describe, expect, it } from "vitest";

import { routeOpen, type OpenRoutes } from "./outside";

function routes(): OpenRoutes & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    remote: (m, s) => calls.push(`remote ${m} ${s}`),
    session: (id) => calls.push(`session ${id}`),
    centre: () => calls.push("centre"),
    page: (id) => calls.push(`page ${id}`),
    app: (id) => calls.push(`app ${id}`),
    conversation: (a, id) => calls.push(`conversation ${a} ${id}`),
    pool: (id) => calls.push(`pool ${id}`),
  };
}

describe("doors into the main window (AJ-E3510)", () => {
  it.each([
    [{ session_id: "9", page: null }, "session 9"],
    [{ session_id: "9", page: null, machine_id: "mac-2" }, "remote mac-2 9"],
    [{ session_id: null, page: "notifications" as const }, "centre"],
    [{ session_id: null, page: "app" as const, id: "cli:work" }, "app cli:work"],
    [{ session_id: null, page: "conversation" as const, id: "acct/77" }, "conversation acct 77"],
    [{ session_id: null, page: "subagents" as const, id: "s:9" }, "session 9"],
    [{ session_id: null, page: "subagents" as const, id: "c:acct/77" }, "conversation acct 77"],
    [{ session_id: null, page: "usage" as const, id: "acct:1" }, "page usage"],
    [{ session_id: null, page: "this_mac" as const }, "page thismac"],
    [{ session_id: null, page: "pool" as const, id: "pool-1" }, "pool pool-1"],
  ])("%j → %s", (open, expected) => {
    const to = routes();
    routeOpen(open, to);
    expect(to.calls).toEqual([expected]);
  });

  it("an unknown page falls back to This Mac, never nowhere", () => {
    const to = routes();
    routeOpen({ session_id: null, page: null }, to);
    expect(to.calls).toEqual(["page thismac"]);
  });
});
