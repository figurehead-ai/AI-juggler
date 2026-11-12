/**
 * D-87 (2026-11-06; the maintainer's finding, from 0.3 and again in 0.4):
 * the band over a page carries only what concerns the whole app (this
 * Mac's alerts, another Mac's, an update); an account's or a session's
 * notice is never there. A session's page shows what is about that
 * session alone, inline under its header, "This session's account…",
 * whose Continue opens that session's own switch.
 *
 * Kept from 0.3.2 (the maintainer's 0.3.0 walk: stacked banners cut the
 * page off, a close took about a minute): one banner, "N more" for the
 * rest; a close or a snooze hides it in the same frame and puts it back
 * if the backend refuses; a reminder shows only when it's due.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { NoticeSummaryInfo } from "../lib/ipc/notices";
import * as notices from "../lib/ipc/notices";
import * as focus from "../lib/handover-focus";
import { NotificationRegion } from "./NotificationRegion";
import { SessionNotices } from "./SessionNotices";
import { resetToasts } from "./Toast";

const seams = window as never as { __AIJ_NOTICES__?: unknown; __AIJ_RESET_QUEUE__?: unknown; __AIJ_FLEET_REMOTE__?: unknown };
const target = { session_id: null, conversation_id: null, account_id: "acc-west", project_id: null, page: null, task_id: null, pool_id: null };
const facts = { window: "5-hour", resets_at_ms: Date.now() + 70 * 60_000, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "West" };
const quota = (id: number, account: string, name: string) => ({ id, kind: "quota", needs: true, title: `${name} is almost out of room`, detail: "7% of its 5-hour window left", project_id: null, project: null, target: { ...target, account_id: account }, facts: { ...facts, account: name }, action: "hand-over", meter: 7, created_ms: Date.now() - id, unread: true, subject: `n:${id}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false });
const LIVE = { id: "s-1", account_id: "acc-west", status: "running", started_at_ms: 1, last_activity_ms: 2, git_branch: "login-flow", cwd: "/w/app" } as unknown as InstanceInfo;

/** West's session's page: its own notices. */
const page = () => render(<SessionNotices accountId="acc-west" switchKey="s-1" reminderKey="s-1" parkedId="s-1" sessions={[LIVE]} />);

beforeEach(() => {
  resetToasts();
  notices.resetNoticeStandIn();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete seams.__AIJ_NOTICES__;
  delete seams.__AIJ_RESET_QUEUE__;
  notices.resetNoticeStandIn();
});

describe("D-87 the band over a page", () => {
  it("never shows an account's or a session's notice, nor a due reminder", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "acc-west", "West"), quota(2, "acc-work", "Work")] };
    seams.__AIJ_RESET_QUEUE__ = [{ id: "rq-s-1", session: "s-1", account: "acc-west", resume_at_ms: Date.now() - 1, state: "ready" }];
    render(<NotificationRegion />);
    await waitFor(() => expect(screen.getByTestId("notification-region")).toBeTruthy());
    await act(async () => new Promise((done) => setTimeout(done, 20)));
    expect(screen.getByTestId("notification-region").textContent).toBe("");
  });

  it("shows what concerns the whole app, one at a time, the rest behind “N more”", async () => {
    // Another Mac's two alerts (D-87: this Mac's own are in the bell and Needs you).
    const alertOf = (kind: string, text: string) => ({ kind, account: "", text, at_ms: Date.now() });
    seams.__AIJ_FLEET_REMOTE__ = [
      { machine_id: "m-2", label: "Studio", connection: "connected", freshness: "live", last_heard_ms: Date.now(), permission: "view", resources: null, instances: [], alerts: [alertOf("actionable", "A session needs you"), alertOf("exhausted", "Work is out of room")], processes: [], clock_skew_ms: null, synced_at_ms: null },
    ];
    render(<NotificationRegion />);
    await waitFor(() => expect(screen.getAllByTestId(/^remote-alert-/)).toHaveLength(1));
    const more = screen.getByTestId("banners-more");
    expect(more.textContent).toBe("1 more notice");
    fireEvent.click(more);
    expect(screen.getAllByTestId(/^remote-alert-/)).toHaveLength(2);
    delete seams.__AIJ_FLEET_REMOTE__;
  });
});

describe("D-87 a session's page: its own notices, inline", () => {
  it("its own account, in its words, whose Continue opens its own switch; never another account's", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "acc-west", "West"), quota(2, "acc-work", "Work")] };
    const opened = vi.spyOn(focus, "focusHandoverSoon").mockImplementation(() => undefined);
    page();
    const own = await screen.findByTestId("session-notice-quota");
    expect(screen.getAllByTestId("session-notice-quota")).toHaveLength(1);
    expect(own.textContent).toContain("This session's account, West, is almost out of room");
    expect(own.textContent).not.toContain("Work");
    expect(screen.getByTestId("session-notice-hand-over").textContent).toBe("Continue this session in another app…");
    fireEvent.click(screen.getByTestId("session-notice-hand-over"));
    expect(opened).toHaveBeenCalledWith("s-1");
    expect(screen.getByTestId("session-notice-remind").textContent).toMatch(/^Remind me at .+$/);
  });

  it("another account's session's page shows nothing of it", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(2, "acc-work", "Work")] };
    render(<SessionNotices accountId="acc-west" switchKey="s-1" reminderKey="s-1" parkedId="s-1" sessions={[LIVE]} />);
    await act(async () => new Promise((done) => setTimeout(done, 20)));
    expect(screen.queryByTestId("session-notices")).toBeNull();
  });

  it("a close hides it in the same frame; a refused one comes back", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "acc-west", "West")] };
    let refuse!: (cause: unknown) => void;
    vi.spyOn(notices, "notificationsDismiss").mockReturnValue(new Promise<NoticeSummaryInfo>((_, no) => (refuse = no)));
    page();
    const banner = await screen.findByTestId("session-notice-quota");
    const start = performance.now();
    fireEvent.click(banner.querySelector<HTMLButtonElement>("button[aria-label^='Close']")!);
    expect(performance.now() - start).toBeLessThan(100);
    expect(screen.queryByTestId("session-notice-quota")).toBeNull();
    await act(async () => refuse(new Error("database is locked")));
    expect(await screen.findByTestId("session-notice-quota")).toBeTruthy();
  });

  it("snooze hides it at once too", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "acc-west", "West")] };
    vi.spyOn(notices, "notificationsSnooze").mockReturnValue(new Promise<NoticeSummaryInfo>(() => {}));
    page();
    await screen.findByTestId("session-notice-quota");
    fireEvent.click(screen.getByTestId("session-notice-snooze"));
    expect(screen.queryByTestId("session-notice-quota")).toBeNull();
  });

  it("its reminder shows nothing while it waits, and once due; another session's never", async () => {
    const later = { id: "rq-later", session: "s-1", account: "acc-west", resume_at_ms: Date.now() + 3_600_000, state: "queued" };
    seams.__AIJ_RESET_QUEUE__ = [later, { ...later, id: "rq-s-2", session: "s-2", state: "ready" }];
    page();
    await act(async () => new Promise((done) => setTimeout(done, 20)));
    expect(screen.queryByTestId("session-notice-reminder")).toBeNull();
    cleanup();
    seams.__AIJ_RESET_QUEUE__ = [{ ...later, id: "rq-s-1", resume_at_ms: Date.now() - 1, state: "ready" }];
    page();
    const due = await screen.findByTestId("session-notice-reminder");
    expect(due.textContent).toContain("This session's account has room again: it can carry on");
    expect(due.textContent).toContain("Nothing restarted by itself");
  });
});
