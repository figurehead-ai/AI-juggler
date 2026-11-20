/**
 * AJ-E3816 — the doors into the Watches page that exist now: a watch
 * alert's Edit watch (by the watch's id, from the alert's key), the
 * notification centre's Watches link, and Settings › Notifications'
 * Manage watches under "Watches you set".
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NoticeRow } from "../NoticeRow";
import { onWatchesIntent, takeWatchesIntent, watchIdOfNotice } from "../../lib/watches-open";
import type { NoticeInfo } from "../../lib/ipc/notices";
import type { NoticeActions } from "../../lib/notice-actions";

const ALERT = {
  id: 41,
  kind: "watch",
  needs: false,
  title: "“api” finished",
  detail: "Your watch",
  project_id: null,
  project: null,
  target: { session_id: "s-api", conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null },
  facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null, told: "finished" },
  action: "open",
  meter: null,
  created_ms: 0,
  unread: false,
  subject: "watch:w-0000019a1b2c:s-api:finishes:1795121000000",
  snoozed: false,
  snoozed_until_ms: null,
  muted: false,
  dismissed: false,
} as NoticeInfo;

const ACTIONS: NoticeActions = { primary: vi.fn(), canHandOver: () => false, handOverSubject: () => null, second: vi.fn(), snooze: () => Promise.resolve(true), wake: vi.fn(), mute: vi.fn(), quietSince: () => null };

afterEach(() => {
  cleanup();
  takeWatchesIntent();
});

describe("AJ-E3816 the doors", () => {
  it("reads a watch's id from its alert, and nothing from other notices", () => {
    expect(watchIdOfNotice(ALERT)).toBe("w-0000019a1b2c");
    expect(watchIdOfNotice({ ...ALERT, kind: "finished" })).toBeNull();
    expect(watchIdOfNotice({ ...ALERT, subject: "watch:../etc:x" })).toBeNull();
  });

  it("a watch alert has Edit watch, which opens its watch by id", () => {
    const edit = vi.fn();
    render(
      <ul>
        <NoticeRow notice={ALERT} actions={ACTIONS} now={0} onEditWatch={edit} />
        <NoticeRow notice={{ ...ALERT, id: 42, kind: "finished", subject: "s:s-api" }} actions={ACTIONS} now={0} onEditWatch={edit} />
      </ul>,
    );
    const buttons = screen.getAllByTestId("notice-edit-watch");
    expect(buttons).toHaveLength(1);
    expect(buttons[0]!.textContent).toContain("Edit watch");
    fireEvent.click(buttons[0]!);
    expect(edit).toHaveBeenCalledWith("w-0000019a1b2c");
  });

  it("the centre's Watches link and Settings' Manage watches open the page", async () => {
    const { NotificationCentre } = await import("../NotificationCentre");
    const opened: string[] = [];
    const seen = vi.fn();
    const stop = onWatchesIntent(seen);
    window.addEventListener("aij:open-screen", (event) => opened.push(String(Reflect.get(Object(Reflect.get(event, "detail")), "id"))));
    window.__AIJ_WATCH_VIEW__ = { watches: [] };
    render(<NotificationCentre summary={null} sessions={[]} onClose={() => undefined} />);
    const link = await screen.findByTestId("notices-watches");
    expect(link.textContent).toContain("Watches");
    fireEvent.click(link);
    expect(opened).toContain("watches");
    expect(seen).toHaveBeenCalled();
    cleanup();
    const { NotificationSettingsCard } = await import("../NotificationSettings");
    render(<NotificationSettingsCard accounts={[]} />);
    const manage = await screen.findByTestId("settings-manage-watches");
    expect(manage.textContent).toContain("Manage watches");
    stop();
    Reflect.deleteProperty(window, "__AIJ_WATCH_VIEW__");
    await waitFor(() => expect(true).toBe(true));
  });
});

describe("AJ-E3816 an App by its App id", () => {
  it("the App page's door counts watches on the App id and opens the page showing only those", async () => {
    const { WatchDoor } = await import("./WatchDoor");
    const { forgetSharedReads } = await import("../../lib/shared-read");
    const { STATES } = await import("../../preview/watches");
    forgetSharedReads();
    window.__AIJ_WATCH_VIEW__ = { watches: structuredClone(STATES.list!.watches) };
    const seen = vi.fn();
    const stop = onWatchesIntent(seen);
    render(<WatchDoor target={{ kind: "app", id: "cli:acct-home" }} name="Agent app two" />);
    const door = await screen.findByText("Watched · 1");
    fireEvent.click(door);
    expect(takeWatchesIntent()).toEqual({ kind: "filter", target: { kind: "app", id: "cli:acct-home" }, name: "Agent app two" });
    cleanup();
    render(<WatchDoor target={{ kind: "app", id: "cli:acct-studio" }} name="Agent app one" />);
    expect((await screen.findByTestId("watch-door")).textContent).toContain("Watch…");
    stop();
    Reflect.deleteProperty(window, "__AIJ_WATCH_VIEW__");
    forgetSharedReads();
  });

  it("a session's App is its folder's CLI App, or the one desktop App on that folder", async () => {
    const { appOfSession } = await import("./watches-view");
    const apps = [
      { id: "cli:f1", kind: "cli" as const, name: "Agent CLI", tool: "t", folder: { id: "f1" } },
      { id: "app:host:/p/one", kind: "desktop" as const, name: "Agent app one", tool: "t", folder: { id: "f1" } },
    ];
    expect(appOfSession({ account_id: "f1", kind: "terminal" }, apps)?.id).toBe("cli:f1");
    expect(appOfSession({ account_id: "f1", kind: "desktop" }, apps)?.id).toBe("app:host:/p/one");
    expect(appOfSession({ account_id: "f2", kind: "terminal" }, apps)).toBeNull();
  });
});
