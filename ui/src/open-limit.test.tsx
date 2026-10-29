/**
 * 0.3 — a session that hits its limit and stays open (an interactive
 * tool's), and the account that has room again: its terminal stays live
 * (only one that ended there reads Ended), its words say it hit its limit,
 * the limit dialog says it is still open, its page's switch list opens
 * when the limit arrives while the page is open, and the "has room again"
 * notice's Switch back opens the switch back for the work that left — on
 * the click, never by itself.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { processGone } from "./lib/session-process";
import { LimitDialog } from "./components/LimitDialog";
import { HandoverSection } from "./components/HandoverSection";
import { NotificationCentre } from "./components/NotificationCentre";
import { attentionWords } from "./lib/attention";
import { primaryLabel } from "./lib/notice-words";
import { notificationsSummary, resetNoticeStandIn, type NoticeInfo, type NoticeSeed } from "./lib/ipc/notices";
import { resetNotices } from "./lib/notices-store";
import type { InstanceInfo } from "./lib/bridge";

const NOW = 1_728_800_000_000;
const w = () => window as unknown as Record<string, unknown>;

const atLimit = (rule: string, status = "quota_blocked") =>
  ({
    id: "s-1",
    status,
    account_id: "acct-work",
    account_label: "Work A",
    attention: { rule, inputs: ["5-hour limit"], since_ms: NOW - 60_000 },
    quota: { used_percentage: 100, resets_at_ms: NOW + 130 * 60_000, confidence: "measured" },
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
  }) as unknown as InstanceInfo;

const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };
const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
function notice(over: Partial<NoticeInfo>): NoticeInfo {
  return { id: 1, kind: "reset", needs: false, title: "", detail: "", project_id: null, project: null, target, facts, action: null, meter: null, created_ms: Date.now(), unread: true, subject: `n:${over.id ?? 1}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false, ...over };
}

beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
  resetNoticeStandIn();
  resetNotices();
});
afterEach(cleanup);

describe("an open session at its limit", () => {
  it("its terminal stays live; only one that ended at its limit reads Ended", () => {
    expect(processGone("quota_blocked", "limit_hit")).toBe(false);
    expect(processGone("quota_blocked", "quota_exit")).toBe(true);
    expect(processGone("quota_blocked", null)).toBe(true);
    expect(processGone("running", null)).toBe(false);
    expect(processGone("finished", null)).toBe(true);
    expect(processGone(undefined, "limit_hit")).toBe(false);
  });

  it("says it hit its usage limit, never the rule's code", () => {
    expect(attentionWords("limit_hit")).toBe("hit its usage limit");
    expect(attentionWords("limit_hit")).not.toContain("limit_hit");
  });

  it("the limit dialog says the session is still open, and Park keeps it there", () => {
    const session = atLimit("limit_hit");
    const { getByTestId } = render(
      <LimitDialog session={session} sessions={[session]} now={NOW} onContinue={() => null} onChooseAnother={() => null} onParked={() => null} onClose={() => null} />,
    );
    const dialog = getByTestId("limit-dialog");
    expect(dialog.textContent).toContain("The session is still open where it stopped; nothing is lost.");
    expect(getByTestId("limit-wait").textContent).toContain("Keeps it open where it stopped.");
    expect(getByTestId("limit-wait").textContent).toContain("nothing runs on its own");
    expect(within(getByTestId("limit-wait")).getByRole("button", { name: /^Park until/ }).hasAttribute("disabled")).toBe(false);
  });

  it("one that ended at its limit still reads as paused", () => {
    const session = atLimit("quota_exit");
    const { getByTestId } = render(
      <LimitDialog session={session} sessions={[session]} now={NOW} onContinue={() => null} onChooseAnother={() => null} onParked={() => null} onClose={() => null} />,
    );
    expect(getByTestId("limit-dialog").textContent).toContain("The session is paused where it stopped; nothing is lost.");
    expect(getByTestId("limit-wait").textContent).toContain("Parks the session.");
  });

  it("its page's switch list opens when the limit arrives while the page is open", async () => {
    w().__AIJ_SWITCH_TARGETS__ = [];
    const running = { ...atLimit("limit_hit"), status: "running", attention: null } as InstanceInfo;
    const onHandover = vi.fn();
    const { rerender } = render(<HandoverSection session={running} onHandover={onHandover} />);
    const section = screen.getByTestId("handover-section") as HTMLDetailsElement;
    expect(section.open).toBe(false);
    rerender(<HandoverSection session={atLimit("limit_hit")} onHandover={onHandover} />);
    await waitFor(() => expect((screen.getByTestId("handover-section") as HTMLDetailsElement).open).toBe(true));
    // Opening the list hands nothing over by itself.
    expect(onHandover).not.toHaveBeenCalled();
  });
});

describe("the account has room again", () => {
  const back = notice({
    id: 5,
    title: "Work A has room again — its 5-hour window reset",
    detail: "“fix-auth-tests” can switch back to it",
    action: "switch-back",
    target: { ...target, session_id: "s-away", account_id: "acct-work" },
    facts: { ...facts, window: "5-hour", sessions: 1, account: "Work A" },
  });

  it("its one action is Switch back, in plain words", () => {
    expect(primaryLabel(back)).toBe("Switch back…");
    expect(primaryLabel(notice({ action: "open" }))).toBe("Open");
  });

  it("Switch back opens the switch back for that work, on the click only", async () => {
    w().__AIJ_NOTICES__ = { app: [], earlier: [back] } as NoticeSeed;
    const opened = vi.fn();
    const on = (event: Event) => opened((event as CustomEvent<{ sessionId?: string }>).detail?.sessionId);
    window.addEventListener("aij:switch-back", on);
    try {
      const summary = await notificationsSummary();
      render(<NotificationCentre summary={summary} sessions={[]} onClose={() => null} />);
      const row = await screen.findByTestId("notice-earlier");
      expect(row.textContent).toContain("“fix-auth-tests” can switch back to it");
      expect(row.textContent).not.toContain("s-away");
      expect(opened).not.toHaveBeenCalled();
      fireEvent.click(within(row).getByRole("button", { name: /^Switch back…/ }));
      expect(opened).toHaveBeenCalledWith("s-away");
      expect(opened).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener("aij:switch-back", on);
    }
  });
});
