/**
 * 0.3.2 — the band of banners, from the maintainer's 0.3.0 walk:
 * stacked banners pushed the page down and cut it off, a close took about
 * a minute, and "Queue for the reset" resumed nothing. One banner shows,
 * with "N more"; a close or a snooze hides it in the same frame, whatever
 * the backend takes, and puts it back if the backend refuses; a reminder
 * shows only when due, and opens the session it was set for.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AccountInfo, InstanceInfo } from "../lib/bridge";
import type { NoticeSummaryInfo } from "../lib/ipc/notices";
import * as notices from "../lib/ipc/notices";
import { ActionHostProvider } from "./ItemMenu";

type ActionHost = Parameters<typeof ActionHostProvider>[0]["host"];
import { NotificationRegion } from "./NotificationRegion";
import { resetToasts } from "./Toast";

const seams = window as never as { __AIJ_NOTICES__?: unknown; __AIJ_RESET_QUEUE__?: unknown };
const target = { session_id: null, conversation_id: null, account_id: "acc-mia", project_id: null, page: null, task_id: null, pool_id: null };
const facts = { window: "5-hour", resets_at_ms: Date.now() + 70 * 60_000, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Mia" };
const quota = (id: number, title: string) => ({ id, kind: "quota", needs: true, title, detail: "7% of its 5-hour window left", project_id: null, project: null, target: { ...target, account_id: `acc-${id}` }, facts, action: "hand-over", meter: 7, created_ms: Date.now() - id, unread: true, subject: `n:${id}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false });
const LIVE = { id: "s-1", account_id: "acc-mia", status: "running", started_at_ms: 1, last_activity_ms: 2, git_branch: "login-flow", cwd: "/w/app" } as unknown as InstanceInfo;
const MIA = { id: "acc-mia", label: "Mia" } as unknown as AccountInfo;

function band(host: Partial<ActionHost> = {}) {
  return render(
    <ActionHostProvider host={host as ActionHost}>
      <NotificationRegion sessions={[LIVE]} accounts={[MIA]} />
    </ActionHostProvider>,
  );
}

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

describe("0.3.2 the banner band", () => {
  it("shows one banner, the rest behind “N more”, opened and folded in place", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "Mia is almost out of room"), quota(2, "Work is almost out of room"), quota(3, "Luka is almost out of room")] };
    band();
    await waitFor(() => expect(screen.getAllByTestId("quota-banner")).toHaveLength(1));
    const more = screen.getByTestId("banners-more");
    expect(more.textContent).toBe("2 more notices");
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    expect(screen.getAllByTestId("quota-banner")).toHaveLength(3);
    expect(screen.getByTestId("banners-more").textContent).toBe("Show fewer");
    fireEvent.click(screen.getByTestId("banners-more"));
    expect(screen.getAllByTestId("quota-banner")).toHaveLength(1);
  });

  it("a close hides it in the same frame, while the backend is still answering", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "Mia is almost out of room"), quota(2, "Work is almost out of room")] };
    const pending = vi.spyOn(notices, "notificationsDismiss").mockReturnValue(new Promise<NoticeSummaryInfo>(() => {}));
    band();
    const first = await screen.findByTestId("quota-banner");
    const title = first.textContent ?? "";
    const close = first.querySelector<HTMLButtonElement>("button[aria-label^='Close']")!;
    const start = performance.now();
    fireEvent.click(close);
    const shown = screen.getByTestId("quota-banner");
    expect(performance.now() - start).toBeLessThan(100);
    expect(shown.textContent).not.toBe(title);
    expect(pending).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("banners-more")).toBeNull();
  });

  it("a close the backend refuses comes back, and says why", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "Mia is almost out of room")] };
    let refuse!: (cause: unknown) => void;
    vi.spyOn(notices, "notificationsDismiss").mockReturnValue(new Promise<NoticeSummaryInfo>((_, no) => (refuse = no)));
    band();
    const banner = await screen.findByTestId("quota-banner");
    fireEvent.click(banner.querySelector<HTMLButtonElement>("button[aria-label^='Close']")!);
    expect(screen.queryByTestId("quota-banner")).toBeNull();
    await act(async () => refuse(new Error("database is locked")));
    expect(await screen.findByTestId("quota-banner")).toBeTruthy();
  });

  it("snooze hides it at once too", async () => {
    seams.__AIJ_NOTICES__ = { app: [quota(1, "Mia is almost out of room")] };
    vi.spyOn(notices, "notificationsSnooze").mockReturnValue(new Promise<NoticeSummaryInfo>(() => {}));
    band();
    await screen.findByTestId("quota-banner");
    fireEvent.click(screen.getByTestId("quota-banner-snooze"));
    expect(screen.queryByTestId("quota-banner")).toBeNull();
  });

  it("a reminder shows nothing while it waits; due, it names the session and opens it", async () => {
    const future = { id: "rq-later", session: "s-1", account: "acc-mia", resume_at_ms: Date.now() + 3_600_000, state: "queued" };
    seams.__AIJ_RESET_QUEUE__ = [future];
    const view = band();
    await waitFor(() => expect(view.queryByTestId("notification-region")).toBeTruthy());
    expect(view.queryByTestId("reminder-ready")).toBeNull();
    cleanup();
    seams.__AIJ_RESET_QUEUE__ = [{ ...future, id: "rq-s-1", resume_at_ms: Date.now() - 1, state: "ready" }];
    const openSessionById = vi.fn();
    band({ openSessionById });
    const due = await screen.findByTestId("reminder-ready");
    expect(due.textContent).toContain("Mia has room again: login-flow can carry on");
    expect(due.textContent).toContain("Nothing restarted by itself");
    fireEvent.click(screen.getByTestId("reminder-open"));
    expect(openSessionById).toHaveBeenCalledWith("s-1");
    expect(screen.queryByTestId("reminder-ready")).toBeNull();
  });
});
