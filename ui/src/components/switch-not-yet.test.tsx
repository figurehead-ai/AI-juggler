/**
 * The rc33 M4 walk: a session with no conversation yet had its account's
 * out-of-room notice offer "Continue this session in another app…" and
 * its header "Continue on another account…", both live, while its switch
 * card said "No other app can take it now" and the dialog refused
 * ("This session hasn't started a conversation yet…"). When the switch
 * would refuse, every door says so up front, in the switch's own words.
 */
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as notices from "../lib/ipc/notices";
import * as focus from "../lib/handover-focus";
import { switchTargets, type SwitchTargetData } from "../lib/ipc/switch-targets";
import { noteSwitchTargets, resetSwitchNotYet, useSwitchNotYet } from "../lib/switch-not-yet";
import { clearSeams, instance, setSeam } from "../test/fixtures";
import { SessionNotices } from "./SessionNotices";
import { handoverTeaser } from "./HandoverSection";
import { ResumeButton } from "../pages/session/ResumeButton";
import { resetToasts } from "./Toast";

const NO_TURN = "This session hasn't started a conversation yet, so there's nothing to switch. Send it a first message, then switch.";
const row = (over: Partial<SwitchTargetData> = {}): SwitchTargetData => ({
  recipe_id: "r-codex",
  provider: "codex",
  account_id: "acc-personal",
  account_label: "Personal · default",
  badge: "Full story, fresh start · Beta",
  validated: false,
  cross_provider: true,
  drops: [],
  score: null,
  filtered_reason: NO_TURN,
  not_yet: NO_TURN,
  top_factors: [],
  factors: [],
  confidence: "Estimate",
  tte_min: null,
  suggested: false,
  ...over,
});
const outOfRoom = {
  id: 1, kind: "quota", needs: true, title: "Work has run out of room", detail: "0% of its 5-hour window left", project_id: null, project: null,
  target: { session_id: null, conversation_id: null, account_id: "acc-work", project_id: null, page: null, task_id: null, pool_id: null },
  facts: { window: "5-hour", resets_at_ms: Date.now() + 70 * 60_000, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Work" },
  action: "hand-over", meter: 0, created_ms: Date.now(), unread: true, subject: "n:1", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false,
};
const WRITER = instance({ id: "writer", account_id: "acc-work", status: "finished", conversation_id: null });

beforeEach(() => {
  resetToasts();
  resetSwitchNotYet();
  notices.resetNoticeStandIn();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  clearSeams("__AIJ_NOTICES__", "__AIJ_SWITCH_TARGETS__");
  resetSwitchNotYet();
  notices.resetNoticeStandIn();
});

describe("rc33 M4: a switch that would refuse says so on every door", () => {
  it("the out-of-room notice's Continue is greyed with the switch's words once its list has asked", async () => {
    setSeam("__AIJ_NOTICES__", { app: [outOfRoom] });
    setSeam("__AIJ_SWITCH_TARGETS__", [row()]);
    const opened = vi.spyOn(focus, "focusHandoverSoon").mockImplementation(() => undefined);
    render(<SessionNotices accountId="acc-work" switchKey="writer" reminderKey="writer" parkedId="writer" sessions={[WRITER]} />);
    const go = await screen.findByTestId("session-notice-hand-over");
    expect(go.getAttribute("aria-disabled")).toBeNull();
    // The page's switch list asks (HandoverSection, ContinueOn).
    await act(async () => void (await switchTargets("writer")));
    await waitFor(() => expect(screen.getByTestId("session-notice-hand-over").getAttribute("aria-disabled")).toBe("true"));
    expect(screen.getByTestId("session-notice-hand-over").getAttribute("aria-description")).toBe(NO_TURN);
    screen.getByTestId("session-notice-hand-over").click();
    expect(opened).not.toHaveBeenCalled();
  });

  it("a session that has talked keeps the notice's Continue live", async () => {
    setSeam("__AIJ_NOTICES__", { app: [outOfRoom] });
    setSeam("__AIJ_SWITCH_TARGETS__", [row({ filtered_reason: null, not_yet: null })]);
    await switchTargets("writer");
    render(<SessionNotices accountId="acc-work" switchKey="writer" reminderKey="writer" parkedId="writer" sessions={[WRITER]} />);
    const go = await screen.findByTestId("session-notice-hand-over");
    expect(go.getAttribute("aria-disabled")).toBeNull();
  });

  it("the header's Continue on another account… greys with the same words", async () => {
    const { quotaDisplay, windowDisplay } = await import("../lib/wire");
    setSeam("__AIJ_QUOTA_DISPLAY__", [{ ...quotaDisplay("acc-work"), five_hour: windowDisplay("tracking", { percentage: 100, resets_at_ms: Date.now() + 2 * 3_600_000 }) }]);
    const ended = instance({ id: "writer", provider: "claude-code", status: "finished", conversation_id: null, account_id: "acc-work" });
    const onContinue = vi.fn();
    render(<ResumeButton session={ended} openIn={null} onContinue={onContinue} continueWhy={NO_TURN} />);
    const go = await screen.findByTestId("header-resume-continue");
    expect(go.getAttribute("aria-disabled")).toBe("true");
    expect(go.getAttribute("aria-description")).toBe(NO_TURN);
    go.click();
    expect(onContinue).not.toHaveBeenCalled();
    clearSeams("__AIJ_QUOTA_DISPLAY__");
  });

  it("the switch card's line gives the same reason, not “No other app can take it now”", () => {
    expect(handoverTeaser([row()])).toBe(NO_TURN);
    // Greyed for their own reasons, it still says none can take it.
    expect(handoverTeaser([row({ not_yet: null, filtered_reason: "Not signed in" })])).toBe("No other app can take it now.");
  });

  it("keeps the answer by the session it was asked for", () => {
    noteSwitchTargets("a", [row()]);
    noteSwitchTargets("b", [row({ not_yet: null })]);
    function Probe({ id }: { id: string }) {
      return <span data-testid="probe">{useSwitchNotYet(id) ?? "none"}</span>;
    }
    render(<Probe id="a" />);
    expect(screen.getByTestId("probe").textContent).toBe(NO_TURN);
    cleanup();
    render(<Probe id="b" />);
    expect(screen.getByTestId("probe").textContent).toBe("none");
  });
});
