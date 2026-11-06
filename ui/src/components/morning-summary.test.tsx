/**
 * AJ-E1004 vitest — the morning summary: severity groups in order with
 * words + an icon, every row a button into its Instance, unmeasured cost
 * says Unavailable, leftovers named, "Mark all seen" clears it; nothing
 * renders when nothing happened.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MorningSummary } from "./MorningSummary";
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
  it("orders by severity, names cost Unavailable and leftovers, opens a row", async () => {
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
      "1 crashed · 1 needs you · 1 finished · 2 leftover processes to clean up",
    );
    const rows = container.querySelectorAll<HTMLButtonElement>("[data-testid='summary-row']");
    expect(Array.from(rows).map((r) => r.dataset.group)).toEqual(["crashed", "needs_you", "finished"]);
    expect(rows[0]!.textContent).toContain("2 leftover processes");
    expect(rows[0]!.textContent).toContain("cost: Unavailable");
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
    expect(names).toEqual(["Claude app · Work · Work Max", "Claude Code · Work · work"]);
  });
});
