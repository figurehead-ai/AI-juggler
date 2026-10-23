import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BudgetCaps, spentWords } from "./BudgetCaps";
import { capHeadline, CapNeedItems, useCapNeeds } from "./BudgetNeeds";
import { dollars, monthStartMs, type BudgetInfo } from "../lib/ipc/budgets";
import type { AccountInfo, ProjectInfo } from "../lib/bridge";

/**
 * AJ-E1514: monthly caps. Usage lists each cap's month so far; the form
 * sets one for an account or a group of projects; Needs you warns at
 * 80 % and 100 % until the user has seen it. Nothing anywhere stops a
 * session: the items offer "Got it" and "See usage", nothing else.
 */

type Seams = { __AIJ_BUDGETS__?: BudgetInfo[]; __AIJ_BUDGET_CALLS__?: Array<{ call: string; args: unknown }> };
const seams = () => window as unknown as Seams;

const ACCOUNTS = [
  { id: "claude-code-1", label: "Work" },
  { id: "codex-1", label: "Personal" },
] as unknown as AccountInfo[];
const PROJECTS = [
  { id: "p-acme", name: "acme" },
  { id: "p-web", name: "acme-web" },
  { id: "p-other", name: "other" },
] as unknown as ProjectInfo[];

const cap = (over: Partial<BudgetInfo>): BudgetInfo => ({
  id: "cap-1",
  name: "Acme",
  monthly_usd: 300,
  account_id: null,
  project_ids: ["p-acme", "p-web"],
  spent_usd: 243.4,
  unpriced: false,
  level: "near",
  seen: false,
  ...over,
});

beforeEach(() => {
  delete seams().__AIJ_BUDGETS__;
  delete seams().__AIJ_BUDGET_CALLS__;
});
afterEach(cleanup);

describe("AJ-E1514 caps on Usage", () => {
  it("says what a cap is for, and that nothing is stopped, before there is one", async () => {
    const view = render(<BudgetCaps accounts={ACCOUNTS} projects={PROJECTS} />);
    await waitFor(() => expect(view.getByTestId("usage-caps").textContent).toContain("Nothing is ever stopped."));
    expect(view.queryByTestId("cap-row")).toBeNull();
  });

  it("shows a cap's month so far, what it counts, and the estimate's basis", async () => {
    seams().__AIJ_BUDGETS__ = [cap({}), cap({ id: "cap-2", name: "Work", account_id: "claude-code-1", project_ids: [], spent_usd: 12.5, level: "under", monthly_usd: 50 })];
    const view = render(<BudgetCaps accounts={ACCOUNTS} projects={PROJECTS} />);
    await waitFor(() => expect(view.getAllByTestId("cap-row")).toHaveLength(2));
    const [acme, work] = view.getAllByTestId("cap-row");
    expect(acme!.textContent).toContain("≈$243 of $300 this month");
    expect(acme!.textContent).toContain("acme, acme-web");
    expect(acme!.textContent).toContain("81%");
    expect(work!.textContent).toContain("≈$12.50 of $50.00 this month");
    expect(work!.textContent).toContain("Work");
    expect(view.getByTestId("usage-caps").textContent).toContain("not your bill");
  });

  it("sets a cap for a group of projects from the form", async () => {
    const view = render(<BudgetCaps accounts={ACCOUNTS} projects={PROJECTS} />);
    fireEvent.click(view.getByTestId("cap-add"));
    const sheet = within(view.getByTestId("cap-sheet"));
    fireEvent.change(sheet.getByTestId("cap-name"), { target: { value: "Acme" } });
    fireEvent.change(sheet.getByTestId("cap-amount"), { target: { value: "$1,200" } });
    fireEvent.click(sheet.getByLabelText("A group of projects, on any account"));
    fireEvent.click(sheet.getByLabelText("acme"));
    fireEvent.click(sheet.getByLabelText("acme-web"));
    fireEvent.click(sheet.getByTestId("cap-save"));
    await waitFor(() => expect(view.queryByTestId("cap-sheet")).toBeNull());
    expect(seams().__AIJ_BUDGET_CALLS__).toEqual([
      {
        call: "save",
        args: { id: null, name: "Acme", monthly_usd: 1200, account_id: null, project_ids: ["p-acme", "p-web"] },
      },
    ]);
    await waitFor(() => expect(view.getAllByTestId("cap-row")).toHaveLength(1));
  });

  it("removes a cap", async () => {
    seams().__AIJ_BUDGETS__ = [cap({})];
    const view = render(<BudgetCaps accounts={ACCOUNTS} projects={PROJECTS} />);
    await waitFor(() => expect(view.getByTestId("cap-row")).toBeTruthy());
    fireEvent.click(view.getByRole("button", { name: /Remove — Acme/ }));
    await waitFor(() => expect(view.queryByTestId("cap-row")).toBeNull());
    expect(seams().__AIJ_BUDGET_CALLS__).toEqual([{ call: "remove", args: { id: "cap-1" } }]);
  });

  it("never shows usage it couldn't price as $0", () => {
    expect(spentWords(cap({ spent_usd: null }))).toBe("Not priced yet · cap $300 a month");
    expect(dollars(1234.5)).toBe("$1,235");
    expect(dollars(7)).toBe("$7.00");
  });

  it("starts the month at this screen's local midnight on the 1st", () => {
    const start = new Date(monthStartMs(new Date(2026, 9, 23, 15, 30)));
    expect([start.getFullYear(), start.getMonth(), start.getDate(), start.getHours()]).toEqual([2026, 9, 1, 0]);
  });
});

function NeedsHarness() {
  const { caps, reload } = useCapNeeds();
  return (
    <ul>
      <CapNeedItems caps={caps} accounts={ACCOUNTS} projects={PROJECTS} onSeen={reload} />
    </ul>
  );
}

describe("AJ-E1514 caps in Needs you", () => {
  it("warns at 80 % and 100 %, not under, and not once seen", async () => {
    seams().__AIJ_BUDGETS__ = [
      cap({}),
      cap({ id: "cap-2", name: "Work", level: "over", spent_usd: 320 }),
      cap({ id: "cap-3", name: "Quiet", level: "under", spent_usd: 10 }),
      cap({ id: "cap-4", name: "Seen", level: "near", seen: true }),
    ];
    const view = render(<NeedsHarness />);
    await waitFor(() => expect(view.getAllByTestId("home-cap")).toHaveLength(2));
    const [near, over] = view.getAllByTestId("home-cap");
    expect(near!.textContent).toContain("Acme is at 81% of its monthly cap");
    expect(near!.textContent).toContain("Nothing is stopped.");
    expect(over!.textContent).toContain("Work is over its monthly cap");
    // The only actions: seen, and Usage.
    expect(within(near!).getAllByRole("button").map((button) => button.textContent)).toEqual(["Got it — Acme", "See usage"]);
  });

  it("Got it records the level seen and the item goes", async () => {
    seams().__AIJ_BUDGETS__ = [cap({})];
    const view = render(<NeedsHarness />);
    await waitFor(() => expect(view.getByTestId("home-cap")).toBeTruthy());
    fireEvent.click(view.getByRole("button", { name: /Got it/ }));
    await waitFor(() => expect(view.queryByTestId("home-cap")).toBeNull());
    expect(seams().__AIJ_BUDGET_CALLS__).toEqual([{ call: "seen", args: { id: "cap-1", level: "near" } }]);
  });

  it("See usage opens Usage", async () => {
    seams().__AIJ_BUDGETS__ = [cap({})];
    const opened = vi.fn();
    window.addEventListener("aij:open-screen", opened);
    const view = render(<NeedsHarness />);
    await waitFor(() => expect(view.getByTestId("home-cap")).toBeTruthy());
    fireEvent.click(view.getByRole("button", { name: "See usage" }));
    window.removeEventListener("aij:open-screen", opened);
    expect((opened.mock.calls[0]![0] as CustomEvent).detail).toEqual({ id: "usage" });
  });

  it("gives Home one headline for one cap or several", () => {
    expect(capHeadline([cap({})])).toBe("Acme is at 81% of its monthly cap");
    expect(capHeadline([cap({}), cap({ id: "b" })])).toBe("2 monthly caps need a look");
  });
});
