/**
 * AJ-E1004 vitest — the morning summary: severity groups in order with
 * words + an icon, every row a button into its Instance, unmeasured cost
 * says Unavailable, leftovers named, "Mark all seen" clears it; nothing
 * renders when nothing happened.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MorningSummary, sinceHeading, whenLine } from "./MorningSummary";
import { summaryHeadline } from "../lib/ended-how";
import type { SummaryRowInfo } from "../types/generated/SummaryRowInfo";

const w = () => window as unknown as Record<string, unknown>;
const row = (id: string, group: string, words: string, over: Partial<SummaryRowInfo> = {}): SummaryRowInfo => ({
  session_id: id,
  account_label: "Work Max",
  provider: "claude-code",
  kind: "terminal",
  project: "storefront",
  group,
  status: group === "crashed" ? "failed" : "running",
  words,
  transitions: [{ from: "running", to: "failed", at_ms: 1_800_000_000_000 }],
  at_ms: null,
  detail: null,
  tokens: null,
  leftovers: 0,
  ...over,
});

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_SUMMARY__;
  delete w().__AIJ_SUMMARY_SEEN__;
});

describe("AJ-E1004 morning summary", () => {
  it("orders by severity, names leftovers, says tokens only when known, opens a row", async () => {
    w().__AIJ_SUMMARY__ = {
      since_ms: 1_800_000_000_000 - 8 * 3_600_000,
      generated_ms: 1_800_000_000_000,
      rows: [
        row("s1", "crashed", "Crashed", { leftovers: 2 }),
        row("s2", "needs_you", "Needs you — quota reached", { tokens: 12_400 }),
        row("s3", "finished", "Finished"),
      ],
      counts: { crashed: 1, needs_you: 1, finished: 1 },
      leftovers: 2,
    };
    const opened: string[] = [];
    const { container } = render(<MorningSummary onOpenSession={(id) => opened.push(id)} />);
    await waitFor(() => expect(container.querySelector("[data-testid='morning-summary']")).not.toBeNull());
    expect(container.querySelector("[data-testid='summary-headline']")!.textContent).toBe(
      "1 needs you · 2 ended: 1 finished, 1 crashed · 2 leftover processes to clean up",
    );
    const rows = container.querySelectorAll<HTMLButtonElement>("[data-testid='summary-row']");
    expect(Array.from(rows).map((r) => r.dataset.group)).toEqual(["crashed", "needs_you", "finished"]);
    expect(rows[0]!.textContent).toContain("2 leftover processes");
    // An unknown value stays silent: no "Unavailable" on every row, never a 0.
    expect(rows[0]!.textContent).not.toMatch(/Unavailable|cost|0 tokens/);
    expect(rows[1]!.textContent).toContain("12,400 tokens");
    fireEvent.click(rows[1]!);
    expect(opened).toEqual(["s2"]);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='summary-seen']")!);
    });
    expect(w().__AIJ_SUMMARY_SEEN__).toBe(1);
    expect(container.querySelector("[data-testid='morning-summary']")).toBeNull();
  });

  it("shows nothing when nothing happened", async () => {
    const { container } = render(<MorningSummary onOpenSession={() => {}} />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector("[data-testid='morning-summary']")).toBeNull();
  });

  it("names a session that ran in a desktop profile by its App (walk A1)", async () => {
    w().__AIJ_SUMMARY__ = {
      since_ms: 1_800_000_000_000 - 8 * 3_600_000,
      generated_ms: 1_800_000_000_000,
      rows: [
        row("s1", "ended", "Ended", { kind: "desktop", account_label: "Work · Work Max" }),
        row("s2", "ended", "Ended", { account_label: "Work · work" }),
      ],
      counts: { ended: 2 },
      leftovers: 0,
    };
    const { container } = render(<MorningSummary onOpenSession={() => undefined} />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='summary-row']").length).toBe(2));
    const names = [...container.querySelectorAll("[data-testid='summary-row']")].map((el) => el.firstElementChild!.textContent);
    expect(names).toEqual(["Claude app · Work · Work Max", "Claude Code CLI · Work · work"]);
  });

  it("shows five rows a group, the rest a click away (walk K1)", async () => {
    const many = Array.from({ length: 12 }, (_, i) => row(`s${i}`, "finished", "Finished"));
    w().__AIJ_SUMMARY__ = { since_ms: 1_800_000_000_000 - 8 * 3_600_000, generated_ms: 1_800_000_000_000, rows: many, counts: { finished: 12 }, leftovers: 0 };
    const { container } = render(<MorningSummary onOpenSession={() => undefined} />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='summary-row']").length).toBe(5));
    const more = container.querySelector<HTMLButtonElement>("[data-testid='summary-show-all']")!;
    expect(more.textContent).toBe("Show all 12");
    fireEvent.click(more);
    expect(container.querySelectorAll("[data-testid='summary-row']").length).toBe(12);
    expect(container.querySelector("[data-testid='summary-show-all']")).toBeNull();
  });

  it("says how long a waiting one has waited, the longest first, and when an ended one ended (AJ-E3307 D2)", async () => {
    const now = Date.now();
    w().__AIJ_SUMMARY__ = {
      since_ms: now - 12 * 3_600_000,
      generated_ms: now,
      rows: [
        row("w1", "needs_you", "Needs you — waiting for an answer", { status: "waiting_user", at_ms: now - 7 * 3_600_000, transitions: [{ from: "running", to: "waiting_user", at_ms: now - 60_000 }] }),
        row("e1", "finished", "Finished", { status: "finished", at_ms: new Date(2026, 10, 10, 6, 38).getTime() }),
      ],
      counts: { needs_you: 1, finished: 1 },
      leftovers: 0,
    };
    const { container } = render(<MorningSummary onOpenSession={() => undefined} />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='summary-row']").length).toBe(2));
    const [waiting, ended] = [...container.querySelectorAll("[data-testid='summary-row']")];
    expect(waiting!.textContent).toContain("waiting 7 h");
    expect(waiting!.textContent).not.toContain("last change");
    expect(ended!.textContent).toMatch(/ended .*6:38/);
  });

  it("leaves the time out of a row that has none: never NaN or Invalid Date (0.4.1 baseline pass)", () => {
    const now = 1_800_000_000_000;
    // A read that left its time off (the 54 capture's rows did), and a null one.
    const off = (group: string, over: Partial<SummaryRowInfo> = {}) => {
      const r = row("x", group, "", { transitions: [], ...over });
      Reflect.deleteProperty(r, "at_ms");
      return r;
    };
    for (const group of ["needs_you", "crashed", "finished", "ended", "running"]) {
      expect(whenLine(off(group), now)).toBeNull();
      expect(whenLine(row("x", group, "", { transitions: [] }), now)).toBeNull();
    }
    // It falls back to its last change that has a time.
    const timeless = { from: "running", to: "waiting_user", at_ms: 0 };
    Reflect.deleteProperty(timeless, "at_ms");
    const steps = [{ from: null, to: "running", at_ms: now - 3_600_000 }, timeless];
    expect(whenLine(off("needs_you", { transitions: steps }), now)).toMatch(/^last change /);
    for (const group of ["needs_you", "crashed", "finished"]) {
      expect(whenLine(off(group, { transitions: steps }), now) ?? "").not.toMatch(/NaN|Invalid Date/);
    }
  });

  it("shows the last summary in its first frame when Home opens again (AJ-E3307: Home's repaint)", async () => {
    w().__AIJ_SUMMARY__ = { since_ms: 1_800_000_000_000 - 8 * 3_600_000, generated_ms: 1_800_000_000_000, rows: [row("s1", "finished", "Finished")], counts: { finished: 1 }, leftovers: 0 };
    const first = render(<MorningSummary onOpenSession={() => undefined} />);
    await waitFor(() => expect(first.container.querySelector("[data-testid='morning-summary']")).not.toBeNull());
    first.unmount();
    const again = render(<MorningSummary onOpenSession={() => undefined} />);
    // No wait: the kept answer, painted at once; the read goes on behind it.
    expect(again.container.querySelector("[data-testid='morning-summary']")).not.toBeNull();
  });

  it("says how each ended run ended, and counts them (D-96 C)", async () => {
    const ended = [
      ...Array.from({ length: 57 }, (_, i) => row(`f${i}`, "finished", "Finished", { status: "finished" })),
      row("e1", "error", "Error", { status: "failed", detail: "exit code 1" }),
      row("e2", "error", "Error", { status: "failed", detail: "exit code 2" }),
      row("c1", "crashed", "Crashed", { status: "stopped", detail: "signal 11" }),
    ];
    w().__AIJ_SUMMARY__ = { since_ms: 1_800_000_000_000 - 8 * 3_600_000, generated_ms: 1_800_000_000_000, rows: ended, counts: { finished: 57, error: 2, crashed: 1 }, leftovers: 0 };
    const { container } = render(<MorningSummary onOpenSession={() => undefined} />);
    await waitFor(() => expect(container.querySelector("[data-testid='summary-headline']")).not.toBeNull());
    expect(container.querySelector("[data-testid='summary-headline']")!.textContent).toBe("60 ended: 57 finished, 2 errors, 1 crashed");
    const rows = [...container.querySelectorAll<HTMLElement>("[data-testid='summary-row']")];
    expect(rows[0]!.dataset.group).toBe("crashed");
    expect(rows[0]!.textContent).toContain("Crashed · signal 11");
    expect(rows.find((one) => one.textContent?.includes("exit code 1"))!.textContent).toContain("Error · exit code 1");
  });

  it("starts plain: \"Since you last looked, 01:56\" (the locale's clock), the 12 h default said in words (D-97 5)", async () => {
    const look = new Date(2026, 10, 10, 1, 56).getTime();
    const built = new Date(2026, 10, 10, 9, 30).getTime();
    expect(sinceHeading({ since_ms: look, generated_ms: built, default_window: false })).toMatch(/^Since you last looked, 0?1:56( AM)?$/);
    // Another day: its day too, never a bare time that reads as today.
    expect(sinceHeading({ since_ms: look - 24 * 3_600_000, generated_ms: built, default_window: false })).toMatch(/^Since you last looked, \S+ 0?1:56( AM)?$/);
    // No last look kept: the default window, in words.
    expect(sinceHeading({ since_ms: built - 12 * 3_600_000, generated_ms: built, default_window: true })).toBe("In the last 12 hours");
    w().__AIJ_SUMMARY__ = { since_ms: look, generated_ms: built, default_window: false, rows: [row("s1", "finished", "Finished")], counts: { finished: 1 }, leftovers: 0 };
    const { findByTestId, container } = render(<MorningSummary onOpenSession={() => undefined} />);
    await waitFor(async () => expect((await findByTestId("summary-since")).textContent).toMatch(/^Since you last looked, 0?1:56/));
    expect(container.textContent).not.toMatch(/Since you were away|since \w{3} /);
  });

  it("says only \"Ended\" when nothing said how (D-96 C)", () => {
    expect(summaryHeadline({ ended: 3 })).toBe("3 ended");
    expect(summaryHeadline({ needs_you: 1, finished: 2, ended: 1, stopped: 1 })).toBe("1 needs you · 4 ended: 2 finished, 1 stopped by you, 1 not known how");
  });
});
