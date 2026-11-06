/**
 * AJ-E2501…E2507 — the notification views' requirements the first tests
 * left unasserted (the evidence manifests of 2026-11-06): the centre as a
 * dialog (focus in, Tab kept inside, focus back to the bell), a row's
 * parts, Earlier a page at a time, the digest and the way to its
 * settings, Settings › Notifications › Muted with Unmute, the bell's
 * count and name, Home's account row and the band on Home, the window's
 * refresh when the service stops answering, and a toast's focus pause.
 * Your menu's facts and hints are in user-menu-facts.test.tsx.
 */
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NotificationBell } from "./NotificationBell";
import { NotificationCentre } from "./NotificationCentre";
import { NotificationRegion } from "./NotificationRegion";
import { HomeNeeds } from "./HomeNeeds";
import { ActionHostProvider } from "./ItemMenu";
import { AppHeader } from "./AppHeader";
import { resetToasts, showToast, ToastHost } from "./Toast";
import { SettingsScreen } from "../screens/Settings";
import { notificationsSummary, resetNoticeStandIn, standInNeeds, type NoticeInfo } from "../lib/ipc/notices";
import { applySummary, resetNotices, setCentreOpen } from "../lib/notices-store";
import { bellLabel } from "../lib/notice-words";
import { resetBackendHealth } from "../lib/backend-health";
import * as health from "../lib/backend-health";
import { useAppData } from "../shell/use-app-data";
import type { HomeNeed } from "../lib/home-needs";
import { account, asButton, asElement, clearSeams, detailOf, setSeam } from "../test/fixtures";

// The window's session list, failing on demand (the service stopped answering).
const listFails = vi.hoisted(() => ({ on: false }));
vi.mock("../lib/ipc/instances", async (original) => {
  const real = await original<typeof import("../lib/ipc/instances")>();
  return {
    ...real,
    instancesList: async () => {
      if (listFails.on) throw new Error("the service is gone");
      return real.instancesList();
    },
  };
});
const SEAMS = ["__AIJ_NOTICES__", "__AIJ_OFFLINE__", "__AIJ_FIGUREHEAD__", "__AIJ_USER__", "__AIJ_CLOUD_BACKUP__", "__AIJ_FLEET_PEERS__", "__AIJ_APP_INFO__", "__AIJ_UPDATE_STATUS__"];

const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };
const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
let nextId = 1;
function notice(over: Partial<NoticeInfo>): NoticeInfo {
  const id = over.id ?? nextId++;
  return { id, kind: "finished", needs: false, title: `Notice ${id}`, detail: "", project_id: null, project: null, target, facts, action: null, meter: null, created_ms: Date.now(), unread: false, subject: `n:${id}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false, ...over };
}
const quota = (over: Partial<NoticeInfo> = {}) =>
  notice({ kind: "quota", needs: true, unread: true, title: "Work Max is almost out of room", detail: "7% of its 5-hour window left", meter: 7, action: "hand-over", target: { ...target, account_id: "acct-work" }, facts: { ...facts, window: "5-hour", resets_at_ms: Date.now() + 3_600_000, sessions: 2, account: "Work Max" }, ...over });

beforeEach(() => {
  clearSeams(...SEAMS);
  resetNoticeStandIn();
  resetNotices();
  resetBackendHealth();
});
afterEach(() => {
  cleanup();
  setCentreOpen(false);
  resetToasts();
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearSeams(...SEAMS);
});

describe("AJ-E2503 R1 — the centre is a dialog: focus in, Tab inside, focus back", () => {
  it("opened from the bell, takes focus, keeps Tab and ⇧Tab inside, and gives focus back to the bell on Esc", async () => {
    setSeam("__AIJ_NOTICES__", { app: [quota()] });
    render(<NotificationBell sessions={[]} />);
    const bell = screen.getByTestId("notification-bell");
    bell.focus();
    fireEvent.click(bell);
    const dialog = await screen.findByRole("dialog", { name: "Notifications" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    await waitFor(() => expect(document.activeElement).toBe(dialog));
    await screen.findByTestId("notice-primary");
    const controls = Array.from(dialog.querySelectorAll<HTMLElement>("button:not(:disabled)"));
    const first = controls[0]!;
    const last = controls.at(-1)!;
    expect(last.textContent).toBe("Notification settings");
    // Tab from the last control wraps to the first; ⇧Tab from the first to the last.
    last.focus();
    fireEvent.keyDown(last, { key: "Tab" });
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(first, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
    // From the dialog itself, ⇧Tab stays inside too.
    dialog.focus();
    fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
    fireEvent.keyDown(dialog, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(bell);
  });
});

describe("AJ-E2503 R2 — a row's parts", () => {
  it("an unread row has its dot and says Unread; a read one doesn't; each has its mark", async () => {
    setSeam("__AIJ_NOTICES__", { app: [quota({ id: 1, unread: true }), notice({ id: 2, kind: "review", needs: true, unread: false, title: "“Fix billing” finished", action: "open", target: { ...target, session_id: "s-2" } })] });
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    const rows = await waitFor(() => {
      const found = within(screen.getByTestId("notices-needs")).getAllByTestId("notice-need");
      expect(found).toHaveLength(2);
      return found;
    });
    const [unread, read] = [rows.find((row) => row.dataset.kind === "quota")!, rows.find((row) => row.dataset.kind === "review")!];
    // The dot: the first mark on the row, coloured only while unread.
    const dot = (row: HTMLElement) => asElement(row.firstElementChild);
    expect(dot(unread).className).toContain("bg-accent");
    expect(dot(read).className).toContain("bg-transparent");
    expect(unread.textContent).toContain("Unread: ");
    expect(read.textContent).not.toContain("Unread: ");
    // Its mark: an icon in its tone (a warning for the account, done for a review).
    const mark = (row: HTMLElement) => asElement(row.children[1]);
    expect(mark(unread).querySelector("svg")).not.toBeNull();
    expect(mark(unread).className).toContain("bg-warning-soft");
    expect(mark(read).className).toContain("bg-success-soft");
  });

  it("an asking row says how long it has waited; Review and Sign in… are its primaries", async () => {
    setSeam("__AIJ_NOTICES__", {
      app: [
        notice({ id: 11, kind: "asks", needs: true, title: "“Docs bot” needs your reply", detail: "docs-site · Claude app", action: "open", created_ms: Date.now() - 24 * 60_000, target: { ...target, session_id: "s-11" } }),
        notice({ id: 12, kind: "review", needs: true, title: "“Fix billing” finished", action: "open", target: { ...target, session_id: "s-12" } }),
        notice({ id: 13, kind: "signin", needs: true, title: "Work needs its sign-in", action: "sign-in", target: { ...target, account_id: "acct-work" } }),
      ],
    });
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    await waitFor(() => expect(within(screen.getByTestId("notices-needs")).getAllByTestId("notice-need")).toHaveLength(3));
    const row = (kind: string) => screen.getAllByTestId("notice-need").find((each) => each.dataset.kind === kind)!;
    expect(row("asks").textContent).toContain("waiting 24 min");
    expect(row("asks").textContent).toContain("docs-site · Claude app");
    expect(within(row("review")).getByTestId("notice-primary").textContent).toBe("Review — “Fix billing” finished");
    expect(within(row("signin")).getByTestId("notice-primary").textContent).toBe("Sign in… — Work needs its sign-in");
  });

  it("snoozed needs fold under “Snoozed: n”", async () => {
    standInNeeds([
      { key: "s:1", kind: "asks", title: "One asks", detail: "", project_id: null, project: null, target: { ...target, session_id: "s1" }, action: "open" },
      { key: "s:2", kind: "asks", title: "Two asks", detail: "", project_id: null, project: null, target: { ...target, session_id: "s2" }, action: "open" },
      { key: "s:3", kind: "asks", title: "Three asks", detail: "", project_id: null, project: null, target: { ...target, session_id: "s3" }, action: "open" },
    ]);
    const { notificationsSnooze } = await import("../lib/ipc/notices");
    await notificationsSnooze("s:1", Date.now() + 3_600_000);
    applySummary(await notificationsSnooze("s:2", null, true));
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    const fold = await screen.findByTestId("notices-snoozed");
    expect(fold.textContent).toBe("Snoozed: 2");
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    expect(within(screen.getByTestId("notices-needs")).getAllByTestId("notice-need")).toHaveLength(1);
    expect(screen.getByTestId("notices-needs-heading").textContent).toBe("Needs action · 1");
    fireEvent.click(fold);
    expect(within(screen.getByTestId("notices-needs")).getAllByTestId("notice-need")).toHaveLength(3);
  });
});

describe("AJ-E2503 R3 — Earlier: time · detail · its link, 20 at a time, Show earlier", () => {
  it("shows the first 20, then the rest on Show earlier; a row has its time, detail and link", async () => {
    const now = Date.now();
    const earlier = [
      notice({ id: 100, kind: "restarted", title: "“Nightly fix” crashed and was restarted", detail: "try 1 of 3", action: "see-why", created_ms: now - 60_000 }),
      notice({ id: 101, kind: "update", title: "AI Juggler 0.4.1 is ready", detail: "Restart when it suits you", action: "whats-new", created_ms: now - 120_000 }),
      ...Array.from({ length: 23 }, (_, index) => notice({ id: 200 + index, kind: "finished", title: `Session ${index} finished`, detail: "shop", created_ms: now - (index + 3) * 60_000 })),
    ];
    setSeam("__AIJ_NOTICES__", { earlier });
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    const list = await screen.findByTestId("notices-earlier");
    await waitFor(() => expect(within(list).getAllByTestId("notice-earlier")).toHaveLength(20));
    const [restarted, update] = within(list).getAllByTestId("notice-earlier");
    const { happenedAt } = await import("../lib/notice-words");
    expect(restarted!.textContent).toContain(`${happenedAt(now - 60_000, now)}· try 1 of 3`);
    expect(within(restarted!).getByRole("button").textContent).toBe("See why — “Nightly fix” crashed and was restarted");
    expect(within(update!).getByRole("button").textContent).toBe("What's new — AI Juggler 0.4.1 is ready");
    const more = screen.getByTestId("notices-more");
    expect(more.textContent).toBe("Show earlier");
    fireEvent.click(more);
    await waitFor(() => expect(within(screen.getByTestId("notices-earlier")).getAllByTestId("notice-earlier")).toHaveLength(25));
    expect(screen.queryByTestId("notices-more")).toBeNull();
  });
});

describe("AJ-E2503 R4 — the digest and the way to Notification settings", () => {
  it("shows the day's digest at its foot, and Notification settings opens Settings › Notifications", async () => {
    setSeam("__AIJ_NOTICES__", { app: [quota()], earlier: [notice({ id: 300, kind: "finished" }), notice({ id: 301, kind: "finished" }), notice({ id: 302, kind: "restarted" })] });
    const onClose = vi.fn();
    const opened: unknown[] = [];
    const listen = (event: Event) => opened.push(detailOf(event));
    window.addEventListener("aij:open-settings", listen);
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={onClose} />);
    expect(screen.getByTestId("notices-digest").textContent).toMatch(/^Today: 2 finished · 1 restarted · Work Max resets .+$/);
    fireEvent.click(screen.getByRole("button", { name: "Notification settings" }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(opened).toEqual([{ section: "notifications" }]);
    window.removeEventListener("aij:open-settings", listen);
  });

  it("with nothing to digest, says how long history is kept", async () => {
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    expect(screen.getByTestId("notices-digest").textContent).toBe("History is kept for 30 days.");
  });
});

describe("AJ-E2503 R6 / AJ-E2501 R5 — Settings › Notifications › Muted", () => {
  it("lists the mutes a page at a time, and Unmute takes one off", async () => {
    const mutes = [
      { scope: "project", value: "proj-docs", label: "docs-site" },
      { scope: "kind", value: "finished", label: "A session finishes" },
      ...Array.from({ length: 52 }, (_, index) => ({ scope: "project", value: `proj-${index}`, label: `project ${index}` })),
    ];
    setSeam("__AIJ_NOTICES__", { mutes });
    render(<SettingsScreen focusSection="notifications" />);
    const card = await screen.findByTestId("notice-mutes");
    await waitFor(() => expect(within(card).getAllByRole("listitem")).toHaveLength(50));
    expect(within(card).getAllByRole("listitem")[0]!.textContent).toContain("docs-siteA project");
    expect(within(card).getAllByRole("listitem")[1]!.textContent).toContain("A session finishesA kind of notification");
    fireEvent.click(within(card).getByRole("button", { name: "Show more" }));
    await waitFor(() => expect(within(screen.getByTestId("notice-mutes")).getAllByRole("listitem")).toHaveLength(54));
    await act(async () => fireEvent.click(within(screen.getByTestId("notice-mutes")).getByRole("button", { name: /^Unmute ?docs-site$/ })));
    await waitFor(() => expect(screen.queryByRole("button", { name: /^Unmute ?docs-site$/ })).toBeNull());
    const { notificationsMutes } = await import("../lib/ipc/notices");
    expect((await notificationsMutes({ cursor: null, limit: 200 })).items.map((mute) => mute.value)).not.toContain("proj-docs");
  });

  it("with nothing muted, says how to mute", async () => {
    render(<SettingsScreen focusSection="notifications" />);
    expect((await screen.findByTestId("notice-mutes-none")).textContent).toContain("⋯ on an item mutes its project or its kind");
  });
});

describe("AJ-E2502 R3 — the bell", () => {
  it("shows no count at zero, and its name says the unread ones", async () => {
    render(<NotificationBell sessions={[]} />);
    await waitFor(() => expect(screen.getByTestId("notification-bell").getAttribute("aria-label")).toBe("Notifications: nothing needs you"));
    expect(screen.queryByTestId("bell-count")).toBeNull();
    cleanup();
    resetNotices();
    setSeam("__AIJ_NOTICES__", { app: [quota({ id: 1 }), quota({ id: 2, title: "Mia is almost out of room", target: { ...target, account_id: "acct-mia" } })], earlier: [notice({ id: 3, unread: true })] });
    resetNoticeStandIn();
    render(<NotificationBell sessions={[]} />);
    expect((await screen.findByTestId("bell-count")).textContent).toBe("2");
    expect(screen.getByTestId("notification-bell").getAttribute("aria-label")).toBe("Notifications: 2 things need you, 3 unread");
    expect(bellLabel(3, 2)).toBe("Notifications: 3 things need you, 2 unread");
    expect(bellLabel(1, 0)).toBe("Notifications: 1 thing needs you");
  });
});

describe("AJ-E2502 R2 — Home's account almost out of room", () => {
  const WORK = account({ id: "acct-work", label: "Work Max" });
  const base: HomeNeed = { key: "quota:acct-work", kind: "quota", title: "Work Max is almost out of room", detail: "7% of its 5-hour window left", severe: false, account: WORK };

  it("with one of its sessions open, “Continue in another app…” opens that session", () => {
    const onOpenSession = vi.fn();
    render(<HomeNeeds needs={[{ ...base, sessionId: "s-open" }]} handlers={{ onOpenSession }} />);
    const button = screen.getByRole("button", { name: /^Continue in another app…/ });
    fireEvent.click(button);
    expect(onOpenSession).toHaveBeenCalledWith("s-open");
  });

  it("with none open, “Open its app” opens its account", () => {
    const openAccount = vi.fn();
    const onOpenSession = vi.fn();
    render(
      <ActionHostProvider host={{ openAccount }}>
        <HomeNeeds needs={[base]} handlers={{ onOpenSession }} />
      </ActionHostProvider>,
    );
    expect(screen.queryByRole("button", { name: /Continue in another app…/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /^Open its app/ }));
    expect(openAccount).toHaveBeenCalledWith(WORK);
    expect(onOpenSession).not.toHaveBeenCalled();
  });
});

describe("AJ-E2504 R2 — on Home the account banner is in Needs you instead", () => {
  it("Home's band leaves the account out; another screen's shows it", async () => {
    setSeam("__AIJ_NOTICES__", { app: [quota()] });
    const view = render(<NotificationRegion hideMachineAlerts />);
    // The banners' page is read (the band is there), but none is shown.
    await waitFor(async () => expect((await notificationsSummary()).banners).toBe(1));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.queryByTestId("quota-banner")).toBeNull();
    view.rerender(<NotificationRegion />);
    expect((await screen.findByTestId("quota-banner")).textContent).toContain("Work Max is almost out of room");
  });

  it("the shell hides it exactly on Home (no selection)", async () => {
    const source = (await import("../shell/pages.tsx?raw")).default;
    expect(source).toContain('hideMachineAlerts={selection.kind === "none"}');
  });
});

describe("AJ-E2504 R3 — the window's refresh says when the service fails or answers", () => {
  it("a failed refresh is reported and the data stays; the next answer ends it", async () => {
    const failed = vi.spyOn(health, "backendFailed");
    const answered = vi.spyOn(health, "backendAnswered");
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { result } = renderHook(() => useAppData({ kind: "none" }, () => {}));
    await waitFor(() => expect(result.current.loaded).toBe(true));
    await waitFor(() => expect(answered).toHaveBeenCalled());
    const shown = result.current.sessions;
    expect(shown.length).toBeGreaterThan(0);
    listFails.on = true;
    answered.mockClear();
    await act(async () => {
      await result.current.refresh();
    });
    expect(failed).toHaveBeenCalledTimes(1);
    expect(answered).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalledWith("AI Juggler: the refresh failed", expect.any(Error));
    // The last state stays on screen.
    expect(result.current.sessions).toBe(shown);
    listFails.on = false;
    await act(async () => {
      await result.current.refresh();
    });
    expect(answered).toHaveBeenCalledTimes(1);
  });

  it("while it doesn't answer, New session waits and says why", () => {
    const header = () => <AppHeader title="Home" global={null} page="home" pairedCount={0} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} />;
    const view = render(header());
    const button = () => asButton(screen.getByRole("button", { name: /New session/ }));
    expect(button().disabled).toBe(false);
    expect(button().getAttribute("title")).toBeNull();
    setSeam("__AIJ_OFFLINE__", { since_ms: Date.now() - 20_000 });
    view.unmount();
    render(header());
    expect(button().disabled).toBe(true);
    expect(button().getAttribute("title")).toBe("Waits until AI Juggler's service answers again");
  });
});

describe("AJ-E2505 R2 — focus the user moves pauses a toast; focus placed for them doesn't", () => {
  it("the user's focus on its Undo holds the time; leaving it lets it run", () => {
    vi.useFakeTimers();
    render(
      <>
        <button type="button">Elsewhere</button>
        <ToastHost />
      </>,
    );
    screen.getByRole("button", { name: "Elsewhere" }).focus();
    act(() => {
      showToast({ message: "Archived.", undo: { onUndo: () => {} } });
    });
    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    act(() => {
      screen.getByRole("button", { name: /Undo/ }).focus();
    });
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.queryByTestId("toast")).not.toBeNull();
    act(() => {
      screen.getByRole("button", { name: "Elsewhere" }).focus();
    });
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.queryByTestId("toast")).toBeNull();
  });

  it("focus put on Undo for them (nothing else had it) lets the time run", () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    act(() => {
      showToast({ message: "Switched.", focusWhenFree: true, undo: { onUndo: () => {} } });
    });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: /Undo/ }));
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.queryByTestId("toast")).toBeNull();
  });
});
