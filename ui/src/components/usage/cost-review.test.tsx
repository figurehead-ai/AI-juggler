/**
 * AJ-E2902 vitest — cost you can act on: the per-finished-task and
 * crashed-or-stuck tiles (their words when there's nothing, never $0),
 * the account table (top project, how sure the price is, sortable), the
 * weekly review's week in two locales, and its export of just that week.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CostByAccount, CostCutTiles, perTask, priceListWords } from "./CostCuts";
import { CostReview } from "./CostReview";
import { WastedSheet } from "./WastedSheet";
import { ActionHostProvider } from "../ItemMenu";
import { firstDayOfWeek, lastFullWeek, weekWords } from "../../lib/usage/week";
import type { UsageCostCutsInfo, UsageQueryInput } from "../../lib/ipc/usageHistory";
import * as costs from "../../lib/ipc/costs";
import * as usage from "../../lib/ipc/usageHistory";
import { pageOf } from "../../lib/paged";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const INPUT: UsageQueryInput = { edges_ms: [0, 1], group_by: "profile", profile: null, project: null, top: 0 };
const PRICES = { version: "v4", captured: "2026-11-03", stale: false, overridden: false };
const cuts = (overrides: Partial<UsageCostCutsInfo> = {}): UsageCostCutsInfo => ({
  finished: { count: 44, io: 44 * 120_000, usd: 84.48, unpriced: false },
  wasted: { count: 2, io: 90_000, usd: 6.3, unpriced: false },
  crashed: 1,
  stuck: 1,
  unknown_endings: 0,
  by_account: [
    { key: "work", account_id: "acc-work", label: "Work", tool: "claude-code", io: 4_500_000, cache: 0, usd: 80, unpriced: false, sessions: 30, top_project: { id: "p-aij", label: "AIJuggler", usd: 60, io: 3_000_000 } },
    { key: "personal", account_id: "acc-p", label: "Personal", tool: "codex", io: 9_000_000, cache: 0, usd: 20, unpriced: true, sessions: 12, top_project: null },
  ],
  prices: PRICES,
  ...overrides,
});

describe("AJ-E2902 the tiles", () => {
  it("per finished task: the average, its tokens and the count", () => {
    const view = render(<CostCutTiles data={cuts()} input={INPUT} />);
    const tile = view.getByTestId("tile-per-task");
    expect(tile.textContent).toContain("~$1.92");
    expect(tile.textContent).toContain("120k tokens a task · 44 finished");
    expect(perTask(cuts().finished)).toBeCloseTo(1.92);
  });

  it("nothing finished, nothing wasted: words, never $0", () => {
    const view = render(<CostCutTiles data={cuts({ finished: { count: 0, io: 0, usd: null, unpriced: false }, wasted: { count: 0, io: 0, usd: null, unpriced: false }, crashed: 0, stuck: 0 })} input={INPUT} />);
    expect(view.getByTestId("tile-per-task").textContent).toContain("No finished tasks in this range");
    expect(view.getByTestId("tile-wasted").textContent).toContain("Nothing went on crashed or stuck sessions");
    expect(view.container.textContent).not.toContain("$0");
  });

  it("wasted: the estimate, what happened (stuck says probably), and the sessions a click away", () => {
    const view = render(<CostCutTiles data={cuts()} input={INPUT} />);
    const tile = view.getByTestId("tile-wasted");
    expect(tile.textContent).toContain("~$6.30");
    expect(view.getByTestId("usage-wasted-open").textContent).toBe("On 2 sessions: 1 crashed, 1 probably stuck — see them");
  });
});

describe("the 0.4 review, 37: the crashed-or-stuck sheet", () => {
  it("plain rows under its headings, what happened in a column of its own", async () => {
    const session = (id: string) => ({ profile: "acc-x", id, session_id: null, tool: "claude-code", title: `Task ${id}`, project_id: null, io: 1000, cache: 0, usd: 1, first_ms: 0, last_ms: 1, messages: 3 });
    vi.spyOn(usage, "usageHistoryWasted").mockImplementation(async (_input, page) =>
      pageOf([{ session: session("a"), why: "crashed" as const }, { session: session("b"), why: "stuck" as const }], page),
    );
    const view = render(<WastedSheet input={INPUT} count={2} onClose={() => null} />);
    const rows = await view.findAllByTestId("usage-wasted-row");
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) {
      const state = within(row).getByTestId("usage-wasted-state");
      expect(state.textContent).toBe(row.dataset.why === "crashed" ? "Crashed" : "Probably stuck");
      // Its own column: beside the session's, never a line under it.
      expect(state.parentElement).toBe(row);
      expect(row.querySelectorAll("p")).toHaveLength(0);
    }
    // No box inside the sheet's box.
    expect(view.getByRole("list", { name: "Sessions that crashed or were probably stuck" }).className).not.toContain("border");
  });
});

describe("AJ-E2902 cost by account", () => {
  it("each account with ~cost, its top project and how sure the price is; sorts by tokens", () => {
    const openProjectById = vi.fn();
    const view = render(
      <ActionHostProvider host={{ openProjectById }}>
        <CostByAccount data={cuts()} busy={false} />
      </ActionHostProvider>,
    );
    expect(view.getByTestId("cost-by-account").textContent).toContain("List prices of 3 Nov 2026");
    const rows = () => view.getAllByTestId("cost-by-account-row");
    expect(rows().map((row) => row.querySelector("th")!.textContent)).toEqual(["WorkClaude Code", "PersonalCodex"]);
    expect(rows()[0]!.textContent).toContain("~$80");
    expect(rows()[1]!.textContent).toContain("list price, but one model has none");
    expect(rows()[1]!.textContent).toContain("No project");
    fireEvent.click(within(rows()[0]!).getByText("AIJuggler"));
    expect(openProjectById).toHaveBeenCalledWith("p-aij");
    fireEvent.click(view.getByTestId("cost-by-account-sort-tokens"));
    expect(rows()[0]!.querySelector("th")!.textContent).toBe("PersonalCodex");
  });

  it("has no Price column while every row is at list price, and shows it when one differs (the 0.4 review, 58)", () => {
    const listed = cuts({ by_account: cuts().by_account.map((row) => ({ ...row, unpriced: false })) });
    const plain = render(<CostByAccount data={listed} busy={false} />);
    expect(plain.queryByRole("columnheader", { name: "Price" })).toBeNull();
    expect(plain.getByTestId("cost-by-account").textContent).not.toContain("list price");
    cleanup();
    const differs = render(<CostByAccount data={cuts()} busy={false} />);
    expect(differs.getByRole("columnheader", { name: "Price" })).toBeTruthy();
    expect(differs.getAllByTestId("cost-by-account-row")[0]!.textContent).toContain("list price");
  });

  it("names the price list, and says so when some prices are the user's", () => {
    expect(priceListWords(PRICES)).toBe("List prices of 3 Nov 2026");
    expect(priceListWords({ ...PRICES, overridden: true })).toBe("List prices of 3 Nov 2026, with your own");
  });
});

describe("AJ-E2902 the weekly review", () => {
  // Wednesday 4 Nov 2026, noon local time.
  const WED = new Date(2026, 10, 4, 12).getTime();

  it("the last full week starts on the locale's first day", () => {
    const monday = lastFullWeek(WED, 1);
    expect(new Date(monday.start).getDay()).toBe(1);
    expect(weekWords(monday.start, monday.end)).toBe("26 Oct – 1 Nov");
    expect(monday.end - monday.start).toBe(new Date(2026, 10, 2).getTime() - new Date(2026, 9, 26).getTime());
    const sunday = lastFullWeek(WED, 7);
    expect(new Date(sunday.start).getDay()).toBe(0);
    expect(weekWords(sunday.start, sunday.end)).toBe("25 Oct – 31 Oct");
    // On the first day itself, the week just begun isn't full: the one before is.
    const onMonday = lastFullWeek(new Date(2026, 10, 2, 9).getTime(), 1);
    expect(weekWords(onMonday.start, onMonday.end)).toBe("26 Oct – 1 Nov");
    expect(firstDayOfWeek("en-GB")).toBe(1);
    expect(firstDayOfWeek("en-US")).toBe(7);
  });

  it("opens on that week against the one before, and exports just that week", async () => {
    const exported = vi.spyOn(costs, "costCsvExport").mockResolvedValue("/tmp/week.csv");
    const view = render(<CostReview onClose={() => {}} now={WED} firstDay={1} />);
    expect(view.getByTestId("cost-review").textContent).toContain("26 Oct – 1 Nov, against the week before");
    await waitFor(() => expect(view.getByTestId("review-cost").textContent).toContain("~$"));
    fireEvent.click(view.getByTestId("cost-review-export"));
    const week = lastFullWeek(WED, 1);
    expect(exported).toHaveBeenCalledWith(undefined, { fromMs: week.start, toMs: week.end });
    await waitFor(() => expect(view.getByTestId("cost-review-csv").textContent).toBe("CSV written: /tmp/week.csv"));
  });
});
