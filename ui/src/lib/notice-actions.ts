/**
 * AJ-E2503 — what a notice's actions do, on the user's click only: open
 * what it is about (a session's page, another app's session, a pool, an
 * account, a screen), hand over, remind at the reset, mark reviewed,
 * snooze, mute. Opening a notice marks it read. Nothing here runs by
 * itself: every function is called from a click (or ↵ on a focused row).
 */
import { openAccount } from "./accounts-page";
import { accountOfProfile, cachedSubscriptions } from "./subscriptions";
import { useActionHost } from "../components/ItemMenu";
import { showToast } from "../components/Toast";
import { useConversations } from "./conversations";
import { errorText } from "./bridge";
import { instanceMarkReviewed, instanceUnmarkReviewed } from "./ipc/park";
import { resetQueueQueue } from "./ipc/alerts";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { focusTask } from "./ipc/tasks";
import { focusThread } from "./ipc/messages";
import { notificationsMute, notificationsRead, notificationsSnooze, type NoticeInfo } from "./ipc/notices";
import { acted } from "./notices-store";
import { focusHandoverSoon, openOnAccount } from "./handover-focus";
import { switchSourceOf } from "./ipc/conversations";
import { conversationTitle } from "./conversations";
import { shortName } from "./session-titles";
import { HOUR_MS, tomorrowMorning, type Second } from "./notice-words";
import { whenWords } from "./time-words";
import { openOnTab, type PageTabId } from "../pages/registry";

/** Sessions still open (a reminder is for these). */
const OPEN = new Set(["running", "waiting_user", "stuck", "starting", "quota_blocked"]);

/** A reminder's key for a session another app runs (the backend's
 * `REMINDER_CONVERSATION`). */
export const REMINDER_CONVERSATION = "conv:";

/** A session's tab, by the notice's kind (else its body). */
const SESSION_TAB: Partial<Record<string, PageTabId>> = { heavy: "processes", restarted: "history" };

/** The screens a notice can open, by the page its target names. */
const SCREENS: Record<string, string> = { this_mac: "thismac", processes: "cleanup", usage: "usage", accounts: "accounts", settings: "settings" };

function openScreen(id: string): void {
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id } }));
}

export interface NoticeActions {
  /** Its primary (opens what it is about, or hands over…). */
  primary: (notice: NoticeInfo) => void;
  /** A hand-over has a session to continue (else it opens the account). */
  canHandOver: (notice: NoticeInfo) => boolean;
  /** 0.4: the session a hand-over continues, by name ("Fix the checkout
   * flow"), else null: the banner names it, so it never reads as being
   * about the page under it. */
  handOverSubject: (notice: NoticeInfo) => string | null;
  /** Its second action beside the primary. */
  second: (notice: NoticeInfo, which: Second) => void;
  /** True once it took (the banner hides it at once, and puts it back
   * if it didn't). */
  snooze: (notice: NoticeInfo, until: "hour" | "tomorrow") => Promise<boolean>;
  /** Ends its snooze now: back into Needs you. */
  wake: (notice: NoticeInfo) => void;
  mute: (notice: NoticeInfo, scope: "project" | "pool" | "kind") => void;
  /** When its session last did something (asks, stuck): the moment its
   * page's state chip counts from, so the two say the same minutes (walk
   * K3: "for 2 min" in the bell, "Probably stuck · 7 min" on its page). */
  quietSince: (notice: NoticeInfo) => number | null;
}

export function useNoticeActions(sessions: InstanceInfo[]): NoticeActions {
  const host = useActionHost();
  const { all: conversations } = useConversations();
  const open = (notice: NoticeInfo) => {
    const { target } = notice;
    // AJ-E3104: a pool's message or task opens the pool on it (its
    // session is only who it's from).
    if (target.pool_id) {
      if (notice.kind === "message") {
        focusThread(target.thread_id ?? null);
        openOnTab("pool", "messages");
      } else if (notice.kind === "task") {
        if (target.task_id) focusTask(target.task_id);
        openOnTab("pool", "tasks");
      }
      return host.openPool?.(target.pool_id);
    }
    // A session opens on the tab the notice is about: its processes
    // (working hard), its history (why it restarted), else its body.
    if (target.session_id) openOnTab("session", SESSION_TAB[notice.kind] ?? "terminal");
    if (target.session_id && sessions.some((s) => s.id === target.session_id)) return host.openSessionById?.(target.session_id);
    const conversation = target.conversation_id ? conversations.find((c) => c.id === target.conversation_id && (!target.account_id || c.account_id === target.account_id)) : undefined;
    if (conversation || (target.conversation_id && target.account_id && host.openConversationById)) {
      openOnTab("conversation", "overview");
      if (conversation) return host.openConversation?.(conversation);
      // Not loaded (past the list): its page reads it by its ids.
      return host.openConversationById!(target.account_id!, target.conversation_id!, target.project_id ?? null);
    }
    if (target.session_id) return host.openSessionById?.(target.session_id);
    if (target.page === "updates") return void window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "updates" } }));
    // AJ-E2912: debug mode stopped by itself: its log is in Diagnostics.
    if (target.page === "diagnostics") return void window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "diagnostics" } }));
    // 0.3.3: keep alive's notices open its switches.
    if (target.page === "desktop_apps") return void window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "keep-alive" } }));
    if (target.page && SCREENS[target.page]) return openScreen(SCREENS[target.page]!);
    // D-62 (AJ-E3205): a notice about an account's limits or renewal
    // opens the Account (its subscription) when its sign-in is known.
    // AJ-E3104: a renewal on its Settings (renewal and fee), the rest on
    // its limits.
    if (target.account_id) {
      const account = accountOfProfile(cachedSubscriptions() ?? [], target.account_id)?.account_id;
      if (account) return openAccount(account, notice.kind === "renewal" ? "settings" : "overview");
      openOnTab("app", "overview");
      return host.openAccountById?.(target.account_id);
    }
    if (target.project_id) return host.openProjectById?.(target.project_id);
    showToast({ id: `nothing:${notice.id}`, message: "Nothing to open for this one", detail: "What it was about isn't here any more." });
  };
  const primary = (notice: NoticeInfo) => {
    if (notice.unread) void acted(() => notificationsRead(notice.id));
    switch (notice.action) {
      case "hand-over": {
        // F16: its newest open session, AI Juggler's or another app's,
        // with its switch list; the account only when none is open.
        const found = openOnAccount(notice.target.account_id, sessions, conversations);
        if (!found) return open(notice);
        if ("session" in found) {
          host.openSessionById?.(found.session.id);
          focusHandoverSoon(found.session.id);
        } else {
          host.openConversation?.(found.conversation);
          focusHandoverSoon(switchSourceOf(found.conversation));
        }
        return;
      }
      case "limit":
        if (notice.target.session_id) return void window.dispatchEvent(new CustomEvent("aij:limit", { detail: { sessionId: notice.target.session_id } }));
        return open(notice);
      // D-05 (0.3): an account has room again — the switch back for the
      // work that left it (its newest); the plan opens, nothing runs yet.
      case "switch-back":
        if (notice.target.session_id) return void window.dispatchEvent(new CustomEvent("aij:switch-back", { detail: { sessionId: notice.target.session_id } }));
        return open(notice);
      case "sign-in":
        // Its sign-in card is on the App's Overview.
        if (!notice.target.account_id) return open(notice);
        openOnTab("app", "overview");
        return host.openAccountById?.(notice.target.account_id);
      case "this-computer":
        return openScreen("thismac");
      case "clean-up":
        return openScreen("cleanup");
      case "usage":
        return openScreen("usage");
      default:
        return open(notice);
    }
  };
  const snooze = (notice: NoticeInfo, until: "hour" | "tomorrow") => {
    const at = until === "hour" ? Date.now() + HOUR_MS : tomorrowMorning(Date.now());
    // D-49: snoozed, it is out of the Needs you count until it wakes.
    const back = `Back at ${whenWords(at)}; out of Needs you until then.`;
    return acted(() => notificationsSnooze(notice.subject, at), "Couldn't snooze it").then((done) => {
      if (done) showToast({ id: `snoozed:${notice.subject}`, message: until === "hour" ? "Snoozed for an hour" : "Snoozed until tomorrow", detail: back, undo: { onUndo: () => acted(() => notificationsSnooze(notice.subject, null)) } });
      return done;
    });
  };
  const wake = (notice: NoticeInfo) => void acted(() => notificationsSnooze(notice.subject, null), "Couldn't wake it");
  const mute = (notice: NoticeInfo, scope: "project" | "pool" | "kind") => {
    const value = scope === "project" ? notice.project_id : scope === "pool" ? notice.target.pool_id : notice.kind;
    if (!value) return;
    // D-62: a pool's notices name it first in their line ("Reviews · …").
    const pool = scope === "pool" ? (notice.detail.split(" · ")[0] ?? "this pool") : "";
    const label = scope === "project" ? (notice.project ?? value) : pool;
    const message = scope === "project" ? `Muted ${notice.project ?? "this project"}` : scope === "pool" ? `Muted ${pool}` : "Muted this kind";
    void acted(() => notificationsMute(scope, value, label), "Couldn't mute it").then(
      (done) => done && showToast({ id: `muted:${scope}:${value}`, message, detail: "It stops interrupting; Needs you still counts it. Unmute in Notification settings." }),
    );
  };
  const second = (notice: NoticeInfo, which: Second) => {
    if (which === "snooze-hour") return void snooze(notice, "hour");
    if (which === "mark-reviewed" && notice.target.session_id) {
      const id = notice.target.session_id;
      void instanceMarkReviewed(id).then(
        () => showToast({ id: `reviewed:${id}`, message: "Marked reviewed", detail: notice.title, undo: { onUndo: () => instanceUnmarkReviewed(id) } }),
        (cause) => showToast({ tone: "error", message: "Couldn't mark it reviewed", detail: errorText(cause), action: { label: "Retry", run: () => second(notice, which) } }),
      );
      return;
    }
    if (which === "remind-reset") {
      // 0.3.2: a reminder at the reset for each open session on the
      // account, AI Juggler's or another app's. At the reset a banner
      // offers to open it; nothing restarts by itself.
      const account = notice.target.account_id;
      const at = notice.facts.resets_at_ms;
      const own = sessions.filter((s) => s.account_id === account && OPEN.has(s.status));
      const others = conversations.filter((c) => c.account_id === account && c.live && c.session_id === null);
      if (!account || at === null || own.length + others.length === 0) {
        showToast({ id: "remind-reset", message: "Nothing to remind you about", detail: "No session is open on it now." });
        return;
      }
      const keys = [...own.map((s) => s.id), ...others.map((c) => `${REMINDER_CONVERSATION}${c.id}`)];
      const what = keys.length === 1 ? "to open its session" : `to open its ${keys.length} sessions`;
      void Promise.all(keys.map((key) => resetQueueQueue(key, account, at))).then(
        () => showToast({ id: "remind-reset", message: `Reminder set for ${whenWords(at)}`, detail: `When it has room again, a banner offers ${what}. Nothing restarts by itself.` }),
        (cause) => showToast({ tone: "error", message: "Couldn't set the reminder", detail: errorText(cause), action: { label: "Retry", run: () => second(notice, which) } }),
      );
    }
  };
  const canHandOver = (notice: NoticeInfo) => openOnAccount(notice.target.account_id, sessions, conversations) !== null;
  const handOverSubject = (notice: NoticeInfo) => {
    const found = openOnAccount(notice.target.account_id, sessions, conversations);
    if (!found) return null;
    if ("conversation" in found) return conversationTitle(found.conversation);
    const seen = conversations.find((c) => c.session_id === found.session.id);
    return seen ? conversationTitle(seen) : shortName(found.session);
  };
  const quietSince = (notice: NoticeInfo) => {
    const session = notice.target.session_id ? sessions.find((s) => s.id === notice.target.session_id) : undefined;
    if (session) return session.last_activity_ms ?? session.started_at_ms;
    const conversation = notice.target.conversation_id ? conversations.find((c) => c.id === notice.target.conversation_id) : undefined;
    return conversation?.last_activity_ms ?? null;
  };
  return { primary, canHandOver, handOverSubject, second, snooze, wake, mute, quietSince };
}
