/** Walk K2 (AJ-E3306, rc24): the Project filter read "web · 167" but the
 * State filter's Open now, Recent and Ended said no "· N". Live, they
 * come from the list's counts: Open now and Ended over the whole list in
 * its search, Recent over its rows since the 7 days before it opened. */
import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SessionsQuery } from "../../lib/ipc/sessions";

const asked: SessionsQuery[] = [];
vi.mock("../../lib/bridge", async (real) => ({ ...(await real<object>()), hasLiveBackend: () => true }));
vi.mock("../../lib/ipc/sessions", async (real) => ({
  ...(await real<object>()),
  sessionsCounts: async (query: SessionsQuery) => {
    asked.push(query);
    return query.since_ms ? { all: 91, open: 0, ended: 91, archived: 0 } : { all: 333, open: 0, ended: 333, archived: 0 };
  },
}));

describe("the State filter's counts", () => {
  it("reads Open now and Ended from the list, Recent since 7 days before it opened", async () => {
    const { useStateCounts } = await import("./useFacets");
    const { RECENT_MS } = await import("./order");
    const opened = 1_800_000_000_000;
    const base = { project_id: null, account_id: null, app_id: "cli:work", which: "ended", since_ms: opened - RECENT_MS, text: "fix", utc_offset_min: 0 } as SessionsQuery;
    const { result } = renderHook(() => useStateCounts("k2|fix", base, opened, () => ({ open: -1, recent: -1, ended: -1 }), true, 333));
    await waitFor(() => expect(result.current).toEqual({ open: 0, recent: 91, ended: 333 }));
    // The list's own state isn't in what's asked: each value's count is over the list in its search.
    expect(asked).toEqual([
      { ...base, which: null, since_ms: null },
      { ...base, which: null, since_ms: opened - RECENT_MS },
    ]);
  });
});
