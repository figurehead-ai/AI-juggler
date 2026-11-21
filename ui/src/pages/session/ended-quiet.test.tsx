/**
 * D-103.3 (M7, twice: the amber notice "shouting" on a page they only
 * wanted to read): reading an ended session stays quiet. D-143 amends it:
 * its account out of room opens the list by itself; low, the folded line
 * says what's possible first. Its account's
 * out-of-room notice is one quiet line under "Continue in another app",
 * and the search for room starts only when the user chooses an app. A
 * live session keeps today's notice and its list at once.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as notices from "../../lib/ipc/notices";
import * as targets from "../../lib/ipc/switch-targets";
import { SessionNotices } from "../../components/SessionNotices";
import { ContinueOn } from "./ContinueOn";
import { HandoverSection } from "../../components/HandoverSection";
import { focusHandover } from "../../lib/handover-focus";
import { fromInstance } from "./model";
import { statusLive } from "../../lib/session-process";
import { instance, setSeam } from "../../test/fixtures";
import { resetToasts } from "../../components/Toast";

const target = { session_id: null, conversation_id: null, account_id: "acc-west", project_id: null, page: null, task_id: null, pool_id: null };
const RESETS = Date.now() + 70 * 60_000;
const facts = { window: "5-hour", resets_at_ms: RESETS, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "West" };
const OUT = { id: 1, kind: "quota", needs: true, title: "West is out of room", detail: "0% of its 5-hour window left", project_id: null, project: null, target, facts, action: "hand-over", meter: 0, created_ms: Date.now() - 1, unread: true, subject: "n:1", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false };

const ENDED = instance({ id: "s-1", account_id: "acc-west", account_label: "West", status: "finished", conversation_id: "c-1" });
const LIVE = instance({ id: "s-1", account_id: "acc-west", account_label: "West", status: "running", conversation_id: "c-1" });

beforeEach(() => {
  resetToasts();
  notices.resetNoticeStandIn();
  setSeam("__AIJ_NOTICES__", { app: [OUT] });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_NOTICES__");
  notices.resetNoticeStandIn();
});

const settle = () => act(async () => new Promise((done) => setTimeout(done, 20)));

/** Opens a folded card, as a click on its title does. */
/** Presses a card's Continue fold (the app's disclosure, a button). */
function openFold(testId: string): void {
  fireEvent.click(within(screen.getByTestId(testId)).getByRole("button", { name: "Continue in another app" }));
}

describe("D-103.3 an ended session's page stays quiet", () => {
  it("shows no notice under its header; a live one keeps it", async () => {
    const ended = render(<SessionNotices accountId="acc-west" quiet switchKey="s-1" reminderKey="s-1" parkedId="s-1" sessions={[ENDED]} />);
    await settle();
    expect(screen.queryByTestId("session-notice-quota")).toBeNull();
    ended.unmount();
    render(<SessionNotices accountId="acc-west" switchKey="s-1" reminderKey="s-1" parkedId="s-1" sessions={[LIVE]} />);
    expect((await screen.findByTestId("session-notice-quota")).textContent).toContain("This session's account, West, is out of room");
  });

  it("D-143: its account out of room, the list opens by itself and searches at once", async () => {
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<ContinueOn model={fromInstance(ENDED)} accounts={[]} displays={[]} onPick={vi.fn()} />);
    await waitFor(() => expect(screen.getByTestId("continue-fold-toggle").getAttribute("aria-expanded")).toBe("true"));
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
    expect(await screen.findByTestId("continue-none")).toBeTruthy();
    // No banner on the page: the list itself is the answer.
    expect(screen.queryByTestId("session-notice-quota")).toBeNull();
    // Its user can still fold it away.
    openFold("continue-fold");
    expect(screen.getByTestId("continue-fold-toggle").getAttribute("aria-expanded")).toBe("false");
  });

  it("D-143: low, it stays folded, and its line says what's possible before the account's fact", async () => {
    setSeam("__AIJ_NOTICES__", { app: [{ ...OUT, meter: 6, title: "West is almost out of room" }] });
    notices.resetNoticeStandIn();
    vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<ContinueOn model={fromInstance(ENDED)} accounts={[]} displays={[]} onPick={vi.fn()} />);
    const teaser = await screen.findByTestId("continue-teaser");
    await waitFor(() => expect(teaser.textContent).toBe("No other app can take it now."));
    expect(screen.getByTestId("continue-fold-toggle").getAttribute("aria-expanded")).toBe("false");
    expect((await screen.findByTestId("continue-account-room")).textContent).toMatch(/almost out of room/);
  });

  it("with room, an ended one stays quiet (D-103.3): folded, no search until it's opened", async () => {
    setSeam("__AIJ_NOTICES__", { app: [] });
    notices.resetNoticeStandIn();
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<ContinueOn model={fromInstance(ENDED)} accounts={[]} displays={[]} onPick={vi.fn()} />);
    await settle();
    expect(asked).not.toHaveBeenCalled();
    expect(screen.queryByTestId("continue-teaser")).toBeNull();
    expect(screen.getByTestId("continue-fold-toggle").getAttribute("aria-expanded")).toBe("false");
    openFold("continue-fold");
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
  });

  it("⌘⇧H (or Switch account…) opens its list and the search too", async () => {
    setSeam("__AIJ_NOTICES__", { app: [] });
    notices.resetNoticeStandIn();
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<ContinueOn model={fromInstance(ENDED)} accounts={[]} displays={[]} onPick={vi.fn()} />);
    await settle();
    expect(asked).not.toHaveBeenCalled();
    await act(async () => void focusHandover("s-1"));
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
  });

  it("a live session lists its apps at once, as before", async () => {
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<ContinueOn model={fromInstance(LIVE)} accounts={[]} displays={[]} onPick={vi.fn()} />);
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
    expect(screen.queryByTestId("continue-fold")).toBeNull();
    expect(screen.queryByTestId("continue-account-room")).toBeNull();
  });

  it("a terminal session's card, ended, its account out of room: open by itself (D-143)", async () => {
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<HandoverSection session={ENDED} onHandover={vi.fn()} />);
    await waitFor(() => expect(screen.getByTestId("handover-section-toggle").getAttribute("aria-expanded")).toBe("true"));
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
  });

  it("a terminal session's card, ended, with room: searches nothing until opened", async () => {
    setSeam("__AIJ_NOTICES__", { app: [] });
    notices.resetNoticeStandIn();
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    render(<HandoverSection session={ENDED} onHandover={vi.fn()} />);
    await settle();
    expect(asked).not.toHaveBeenCalled();
    expect(screen.queryByTestId("handover-teaser")).toBeNull();
    openFold("handover-section");
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
  });

  it("a live terminal session's card ranks its apps at once, as before; its account out of room, open (D-143)", async () => {
    const asked = vi.spyOn(targets, "switchTargets").mockResolvedValue([]);
    const out = render(<HandoverSection session={LIVE} onHandover={vi.fn()} />);
    await waitFor(() => expect(screen.getByTestId("handover-section-toggle").getAttribute("aria-expanded")).toBe("true"));
    out.unmount();
    setSeam("__AIJ_NOTICES__", { app: [] });
    notices.resetNoticeStandIn();
    render(<HandoverSection session={LIVE} onHandover={vi.fn()} />);
    await waitFor(() => expect(asked).toHaveBeenCalledWith("s-1"));
    expect(await screen.findByTestId("handover-teaser")).toBeTruthy();
    expect(screen.queryByTestId("continue-account-room")).toBeNull();
  });

  it("one at its limit is still live (Continue matters most there)", () => {
    expect(statusLive("quota_blocked")).toBe(true);
    expect(statusLive("waiting_user")).toBe(true);
    for (const ended of ["finished", "stopped", "failed", "handed_over"]) expect(statusLive(ended)).toBe(false);
  });
});
