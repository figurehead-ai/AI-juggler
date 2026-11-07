/**
 * AJ-E2104 R5: the View's Only › Pool. Once you have pools of your own,
 * the rail shows only one pool's sessions; `pool:` typed in search does
 * the same by the pool's name.
 */
import { cleanup, fireEvent, render, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ViewPopover } from "./ViewPopover";
import { onlyOptions } from "./RailToolbar";
import { DEFAULT_VIEW, matches, parseQuery, resetRailViewCache, setRailView, useRailView } from "../lib/rail-view";
import { itemsOf } from "../lib/rail-items";
import type { PoolOf } from "../lib/rail-places";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../preview/fixtures";

afterEach(() => {
  cleanup();
  localStorage.clear();
  resetRailViewCache();
});

/** Two of the fixture's sessions: the first in "Checkout team", the rest in "Docs". */
function twoPools() {
  const [inPool, other] = itemsOf({ sessions: MOCK_SESSIONS, conversations: [], projects: MOCK_PROJECTS, seen: {} }).filter((item) => item.session);
  if (!inPool || !other) throw new Error("the fixture has two sessions");
  const id = inPool.session?.id;
  const poolOf: PoolOf = (item) => (item.session?.id === id ? { id: "pool-checkout", label: "Checkout team" } : { id: "pool-docs", label: "Docs" });
  return { inPool, other, poolOf };
}

describe("Only › Pool (AJ-E2104 R5)", () => {
  it("lists each pool once by name and keeps only the one picked", () => {
    const { inPool, other, poolOf } = twoPools();
    const options = onlyOptions(MOCK_SESSIONS, MOCK_PROJECTS, [], [], undefined, poolOf);
    expect(options.pool.map((option) => option.label)).toEqual(["Checkout team", "Docs"]);
    const only = { pool: "pool-checkout" };
    expect(matches(inPool, only, parseQuery(""), undefined, poolOf)).toBe(true);
    expect(matches(other, only, parseQuery(""), undefined, poolOf)).toBe(false);
  });

  it("pool: in search finds a pool by the start of its name", () => {
    const { inPool, other, poolOf } = twoPools();
    expect(matches(inPool, {}, parseQuery("pool:check"), undefined, poolOf)).toBe(true);
    expect(matches(other, {}, parseQuery("pool:check"), undefined, poolOf)).toBe(false);
  });

  it("is a line of the View's Only, and picking a pool keeps it in the View", () => {
    setRailView(DEFAULT_VIEW);
    const options = { app: [], profile: [], account: [], project: [], mac: [], pool: [{ value: "pool-checkout", label: "Checkout team" }, { value: "pool-docs", label: "Docs" }] };
    const view = render(<ViewPopover options={options} />);
    fireEvent.click(view.getByTestId("rail-view-button"));
    fireEvent.click(view.getByTestId("rail-view-only-pool"));
    fireEvent.click(view.getByTestId("rail-view-only-pool-pool-checkout"));
    const { result } = renderHook(() => useRailView());
    expect(result.current.only.pool).toBe("pool-checkout");
  });
});
