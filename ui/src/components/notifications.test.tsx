import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Banner } from "./Banner";
import { resetToasts, showToast, ToastHost, TOAST_SECONDS } from "./Toast";
import { NotificationCentre } from "./NotificationCentre";
import { NotificationBell } from "./NotificationBell";
import { OfflineBanner } from "./OfflineBanner";
import { UserMenu, shortEmail } from "./UserMenu";
import { resetNoticeStandIn, notificationsSummary, standInNeeds, type NoticeInfo, type StandInNeed, type NoticeSeed } from "../lib/ipc/notices";
import { resetNotices, applySummary } from "../lib/notices-store";
import { backendFailed, resetBackendHealth } from "../lib/backend-health";
import { digestLine, mutedLine, quotaDetail, secondOf } from "../lib/notice-words";

/**
 * AJ-E2501..E2507 — the notification system's views (the approved design
 * of 2026-10-27): §2's Banner and Toast, the centre (Needs action,
 * Earlier, the marks, empty and muted), the bell's one count, the user
 * menu without and with a Figurehead account, and the banner shown when
 * AI Juggler's own service stops answering. Every action is a click.
 */

type Seams = { __AIJ_NOTICES__?: NoticeSeed; __AIJ_OFFLINE__?: { since_ms: number }; __AIJ_FIGUREHEAD__?: unknown; __AIJ_USER__?: unknown };
const w = () => window as unknown as Seams;
const T0 = Date.now();

const facts = { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null };
const target = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
function notice(over: Partial<NoticeInfo>): NoticeInfo {
  return { id: 1, kind: "finished", needs: false, title: "", detail: "", project_id: null, project: null, target, facts, action: null, meter: null, created_ms: T0, unread: false, subject: `n:${over.id ?? 1}`, snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false, ...over };
}
const quota = notice({ id: 7, kind: "quota", needs: true, unread: true, title: "Work Max is almost out of room", detail: "7% of its 5-hour window left", meter: 7, action: "hand-over", target: { ...target, account_id: "acct-work" }, facts: { ...facts, window: "5-hour", resets_at_ms: T0 + 3_600_000, sessions: 2, account: "Work Max" } });
const asks: StandInNeed = { key: "s:1", kind: "asks", title: "“Docs bot” needs your reply", detail: "docs-site · Claude app", project_id: "proj-docs", project: "docs-site", target: { ...target, session_id: "sess-1" }, action: "open" };

beforeEach(() => {
  delete w().__AIJ_NOTICES__;
  delete w().__AIJ_OFFLINE__;
  delete w().__AIJ_FIGUREHEAD__;
  resetNoticeStandIn();
  resetNotices();
  resetBackendHealth();
});
afterEach(() => {
  cleanup();
  resetToasts();
  vi.useRealTimers();
});

describe("§2 Banner", () => {
  it("says who and what, shows its meter, and runs only the action clicked", () => {
    const hand = vi.fn();
    const queue = vi.fn();
    const close = vi.fn();
    render(<Banner title="Work Max is almost out of room" detail="7% of its 5-hour window left" meter={{ used: 93, label: "93% used" }} actions={[{ label: "Hand over…", primary: true, onClick: hand }, { label: "Queue for the reset", onClick: queue }]} onDismiss={close} dismissLabel="Close the banner about Work Max" testId="b" />);
    const meter = screen.getByRole("meter", { name: "93% used" });
    expect(meter.getAttribute("aria-valuenow")).toBe("93");
    expect(hand).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Hand over…" }));
    expect(hand).toHaveBeenCalledTimes(1);
    expect(queue).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Close the banner about Work Max" }));
    expect(close).toHaveBeenCalledTimes(1);
  });

  it("never says an internal id, a confidence badge or a raw window name", () => {
    const words = quotaDetail(quota, T0);
    expect(words).toMatch(/^7% of its 5-hour window left · resets at .+ · 2 sessions use it$/);
    for (const internal of ["five_hour", "Measured", "Computed", "acct-work"]) expect(words).not.toContain(internal);
  });

  it("offline: the last state stays, it says since when, and Retry is a click", () => {
    const retry = vi.fn();
    const { rerender } = render(<OfflineBanner onRetry={retry} />);
    expect(screen.queryByTestId("offline-banner")).toBeNull();
    // A preview without its service is not "stopped answering".
    backendFailed();
    rerender(<OfflineBanner onRetry={retry} />);
    expect(screen.queryByTestId("offline-banner")).toBeNull();
    w().__AIJ_OFFLINE__ = { since_ms: Date.now() - 20_000 };
    rerender(<OfflineBanner onRetry={retry} />);
    const banner = screen.getByTestId("offline-banner");
    expect(banner.getAttribute("role")).toBe("alert");
    expect(banner.textContent).toContain("Showing the last state AI Juggler knew");
    expect(banner.textContent).toContain("It couldn't reach its own service 20 s ago. Actions are paused until it's back.");
    expect(retry).not.toHaveBeenCalled();
    fireEvent.click(within(banner).getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledTimes(1);
  });
});

describe("§2 Toast", () => {
  it("stays six seconds, and hovering pauses its time", () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    act(() => void showToast({ id: "saved", message: "Saved", detail: "Quiet hours 22:00–07:00" }));
    expect(TOAST_SECONDS).toBe(6);
    const toast = screen.getByTestId("toast");
    act(() => vi.advanceTimersByTime(3_000));
    fireEvent.mouseEnter(toast);
    act(() => vi.advanceTimersByTime(20_000));
    expect(screen.queryByTestId("toast")).not.toBeNull();
    fireEvent.mouseLeave(toast);
    act(() => vi.advanceTimersByTime(2_900));
    expect(screen.queryByTestId("toast")).not.toBeNull();
    act(() => vi.advanceTimersByTime(200));
    expect(screen.queryByTestId("toast")).toBeNull();
  });

  it("an error stays until it's closed, and its Retry runs once, on the click", () => {
    vi.useFakeTimers();
    const retry = vi.fn();
    render(<ToastHost />);
    act(() => void showToast({ message: "Couldn't start the session", detail: "Its account isn't signed in.", tone: "error", action: { label: "Retry", run: retry } }));
    act(() => vi.advanceTimersByTime(120_000));
    const toast = screen.getByTestId("toast");
    expect(toast.getAttribute("data-tone")).toBe("error");
    expect(screen.queryByTestId("toast-time")).toBeNull();
    expect(retry).not.toHaveBeenCalled();
    fireEvent.click(within(toast).getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("toast")).toBeNull();
  });

  it("sits at the bottom right, three at most, with the time left as a bar", () => {
    render(<ToastHost />);
    act(() => {
      for (const n of [1, 2, 3, 4]) showToast({ id: `t${n}`, message: `Toast ${n}` });
    });
    const stack = screen.getByTestId("toast-stack");
    expect(stack.className).toContain("bottom-4");
    expect(stack.className).toContain("right-4");
    expect(within(stack).getAllByTestId("toast").map((t) => t.getAttribute("aria-label"))).toEqual(["Toast 2", "Toast 3", "Toast 4"]);
    expect(within(stack).getAllByTestId("toast-time")).toHaveLength(3);
  });
});

describe("the notification centre", () => {
  it("says \"You're all caught up.\" when nothing is there", async () => {
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    expect((await screen.findByTestId("notices-empty")).textContent).toContain("You're all caught up.");
    expect(screen.getByRole("dialog", { name: "Notifications" })).toBeTruthy();
  });

  it("in the quiet hours, says what is muted and what still counts", async () => {
    w().__AIJ_NOTICES__ = { app: [quota], quiet_until: "09:00" };
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    expect((await screen.findByTestId("notices-muted")).textContent).toBe("Muted until 09:00 — Needs you still tracks 1.");
    expect(mutedLine("09:00", 2)).toBe("Muted until 09:00 — Needs you still tracks 2.");
  });

  it("lists Needs action and Earlier; opening it changes nothing by itself", async () => {
    w().__AIJ_NOTICES__ = { app: [quota], earlier: [notice({ id: 3, kind: "restarted", title: "“Nightly fix” crashed and was restarted (try 1 of 3)", action: "see-why", unread: true })] };
    standInNeeds([asks]);
    const before = await notificationsSummary();
    expect(before.count).toBe(2);
    expect(before.unread).toBe(3);
    const onClose = vi.fn();
    render(<NotificationCentre summary={before} sessions={[]} onClose={onClose} />);
    const needs = await screen.findByTestId("notices-needs");
    expect(within(needs).getAllByTestId("notice-need").map((row) => row.getAttribute("data-kind"))).toEqual(["asks", "quota"]);
    expect(within(screen.getByTestId("notices-earlier")).getAllByTestId("notice-earlier").map((row) => row.getAttribute("data-kind"))).toEqual(["restarted"]);
    expect(screen.getByText("Needs action · 2")).toBeTruthy();
    // Rendering it read nothing, snoozed nothing, opened nothing.
    expect(await notificationsSummary()).toEqual(before);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("each need's ⋯ offers snooze and mute, icons on every line; a snoozed need leaves the count until it wakes (D-49)", async () => {
    standInNeeds([asks]);
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    fireEvent.click(await screen.findByTestId("notice-more"));
    const menu = screen.getByRole("menu");
    const items = within(menu).getAllByRole("menuitem");
    expect(items.map((item) => item.querySelector("[data-label]")?.textContent)).toEqual(["Snooze 1 h", "Snooze until tomorrow", "Mute this project", "Mute this kind"]);
    expect(menu.querySelectorAll("svg")).toHaveLength(items.length);
    await act(async () => fireEvent.click(screen.getByTestId("notice-snooze-hour")));
    await waitFor(async () => expect((await notificationsSummary()).snoozed).toBe(1));
    expect((await notificationsSummary()).count).toBe(0);
  });

  it("a snoozed need says when it is back, and can be brought back now", async () => {
    render(
      <>
        <NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />
        <ToastHost />
      </>,
    );
    standInNeeds([asks]);
    const { notificationsSnooze } = await import("../lib/ipc/notices");
    const back = Date.now() + 3_600_000;
    applySummary(await notificationsSnooze("s:1", back));
    expect((await notificationsSummary()).count).toBe(0);
    fireEvent.click(await screen.findByTestId("notices-snoozed"));
    const row = await screen.findByTestId("notice-need");
    expect(row.textContent).toContain("Snoozed · back at");
    fireEvent.click(within(row).getByTestId("notice-more"));
    await act(async () => fireEvent.click(screen.getByTestId("notice-wake")));
    await waitFor(async () => expect((await notificationsSummary()).count).toBe(1));
  });

  it("the snooze toast says it is out of Needs you until it is back", async () => {
    render(
      <>
        <NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />
        <ToastHost />
      </>,
    );
    applySummary(standInNeeds([asks]));
    fireEvent.click(await screen.findByTestId("notice-more"));
    await act(async () => fireEvent.click(screen.getByTestId("notice-snooze-tomorrow")));
    const toast = await screen.findByTestId("toast");
    expect(toast.textContent).toContain("Snoozed until tomorrow");
    expect(toast.textContent).toMatch(/Back at .+ tomorrow; out of Needs you until then\./);
  });

  it("Mark all read clears every dot and no count", async () => {
    w().__AIJ_NOTICES__ = { app: [quota] };
    standInNeeds([asks]);
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    await act(async () => fireEvent.click(await screen.findByTestId("notices-read-all")));
    const after = await notificationsSummary();
    expect(after.unread).toBe(0);
    expect(after.count).toBe(2);
  });

  it("Esc closes it", async () => {
    const onClose = vi.fn();
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("words: the digest, and the second actions the design gives", () => {
    expect(digestLine({ finished: 4, restarted: 1, resets: 0, updates: 1, next_reset_account: "Work Max", next_reset_ms: T0 + 60_000 }, T0)).toMatch(/^Today: 4 finished · 1 restarted · 1 update · Work Max resets .+$/);
    expect(digestLine({ finished: 0, restarted: 0, resets: 0, updates: 0, next_reset_account: null, next_reset_ms: null }, T0)).toBeNull();
    expect(secondOf(quota)?.label).toBe("Queue for the reset");
    expect(secondOf(notice({ kind: "asks", needs: true }))?.label).toBe("Snooze 1 h");
    expect(secondOf(notice({ kind: "review", needs: true, target: { ...target, session_id: "s" } }))?.label).toBe("Mark reviewed");
    expect(secondOf(notice({ kind: "update" }))).toBeNull();
  });
});

describe("the bell", () => {
  it("shows the model's one count, and opens the centre", async () => {
    w().__AIJ_NOTICES__ = { app: [quota] };
    applySummary(standInNeeds([asks]));
    render(<NotificationBell sessions={[]} />);
    expect((await screen.findByTestId("bell-count")).textContent).toBe("2");
    const bell = screen.getByRole("button", { name: /^Notifications: 2 things need you/ });
    expect(bell.getAttribute("aria-keyshortcuts")).toBe("Meta+Shift+N");
    fireEvent.click(bell);
    expect(await screen.findByTestId("notification-centre")).toBeTruthy();
  });
});

describe("your menu", () => {
  it("without a Figurehead account: one quiet Connect, icons on every line, three groups", async () => {
    render(<UserMenu />);
    fireEvent.click(await screen.findByTestId("user-menu-button"));
    const menu = await screen.findByRole("menu", { name: "Your menu" });
    expect(within(menu).getByTestId("user-menu-who").textContent).toContain("On this Mac · no Figurehead account");
    const labels = within(menu).getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent);
    // D-49: My Macs shows only with a Figurehead account.
    expect(labels).toEqual(["Connect a Figurehead account (optional)", "Settings", "This Mac", "Processes", "Privacy", "Keyboard shortcuts", "Help and guides", "Report a problem…", "About and updates"]);
    for (const item of within(menu).getAllByRole("menuitem")) expect(item.querySelector("svg")).not.toBeNull();
    expect(within(menu).getAllByRole("separator")).toHaveLength(2);
    expect(within(menu).getAllByRole("menuitemradio").map((radio) => radio.textContent)).toEqual(["Light", "Dark", "Auto"]);
    expect(within(menu).queryByText(/Disconnect/)).toBeNull();
  });

  it("with one: the address shortened, Cloud backup, My Macs, and Disconnect last", async () => {
    w().__AIJ_FIGUREHEAD__ = { state: "linked", email: "sam@example.com", plan: "Free", device_name: "Studio", features: [], link: null, refreshed_at_ms: T0, offline: false, notice: null, consent_version: 1, consent_current: 1 };
    render(<UserMenu />);
    fireEvent.click(await screen.findByTestId("user-menu-button"));
    const menu = await screen.findByRole("menu", { name: "Your menu" });
    await waitFor(() => expect(within(menu).getByTestId("user-menu-who").textContent).toContain("s…@example.com · Figurehead"));
    const labels = within(menu).getAllByRole("menuitem").map((item) => item.querySelector("[data-label]")?.textContent);
    expect(labels.slice(0, 2)).toEqual(["Cloud backup", "My Macs"]);
    expect(labels.at(-1)).toBe("Disconnect Figurehead…");
    expect(labels).not.toContain("Connect a Figurehead account (optional)");
    expect(shortEmail("faruk@example.com")).toBe("f…@example.com");
  });

  it("Appearance changes the theme in place; Settings opens Settings", async () => {
    window.matchMedia = vi.fn(() => ({ matches: false, addEventListener: () => {}, removeEventListener: () => {} })) as unknown as typeof window.matchMedia;
    const opened: unknown[] = [];
    const listen = (event: Event) => opened.push((event as CustomEvent).detail);
    window.addEventListener("aij:open-settings", listen);
    render(<UserMenu />);
    fireEvent.click(await screen.findByTestId("user-menu-button"));
    fireEvent.click(await screen.findByRole("menuitemradio", { name: "Light" }));
    expect(localStorage.getItem("aij.theme")).toBe("light");
    expect(screen.getByRole("menuitemradio", { name: "Light" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByTestId("user-menu-settings"));
    expect(opened).toEqual([null]);
    window.removeEventListener("aij:open-settings", listen);
  });
});
