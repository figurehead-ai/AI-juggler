/**
 * AJ-E2503 — what a notice's actions do, on the user's click only: open
 * what it is about (a session's page, another app's session, a pool, an
 * account, a screen), hand over, queue for the reset, mark reviewed,
 * snooze, mute. Opening a notice marks it read. Nothing here runs by
 * itself: every function is called from a click (or ↵ on a focused row).
 */
import { useActionHost } from "../components/ItemMenu";
import { showToast } from "../components/Toast";
import { useConversations } from "./conversations";
import { errorText, instanceMarkReviewed, instanceUnmarkReviewed, resetQueueQueue, type InstanceInfo } from "./bridge";
import { focusTask } from "./ipc/tasks";
import { notificationsMute, notificationsRead, notificationsSnooze, type NoticeInfo } from "./ipc/notices";
import { acted } from "./notices-store";
import { liveOn } from "./needs-you";
import { clock, HOUR_MS, tomorrowMorning, type Second } from "./notice-words";
import { rememberTab } from "../pages/registry";

/** The screens a notice can open, by the page its target names. */
const SCREENS: Record<string, string> = { this_mac: "thismac", processes: "cleanup", usage: "usage", accounts: "accounts", settings: "settings" };

function openScreen(id: string): void {
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id } }));
}

export interface NoticeActions {
  /** Its primary (opens what it is about, or hands over…). */
  primary: (notice: NoticeInfo) => void;
  /** Its second action beside the primary. */
  second: (notice: NoticeInfo, which: Second) => void;
  snooze: (notice: NoticeInfo, until: "hour" | "tomorrow") => void;
  /** Ends its snooze now: back into Needs you. */
  wake: (notice: NoticeInfo) => void;
  mute: (notice: NoticeInfo, scope: "project" | "kind") => void;
}

export function useNoticeActions(sessions: InstanceInfo[]): NoticeActions {
  const host = useActionHost();
  const { all: conversations } = useConversations();
  const open = (notice: NoticeInfo) => {
    const { target } = notice;
    if (target.session_id && sessions.some((s) => s.id === target.session_id)) return host.openSessionById?.(target.session_id);
    const conversation = target.conversation_id ? conversations.find((c) => c.id === target.conversation_id && (!target.account_id || c.account_id === target.account_id)) : undefined;
    if (conversation) return host.openConversation?.(conversation);
    if (target.session_id) return host.openSessionById?.(target.session_id);
    if (target.pool_id) {
      if (target.task_id) {
        focusTask(target.task_id);
        rememberTab("pool", "tasks");
      }
      return host.openPool?.(target.pool_id);
    }
    if (target.page === "updates") return void window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "updates" } }));
    if (target.page && SCREENS[target.page]) return openScreen(SCREENS[target.page]!);
    if (target.account_id) return host.openAccountById?.(target.account_id);
    if (target.project_id) return host.openProjectById?.(target.project_id);
  };
  const primary = (notice: NoticeInfo) => {
    if (notice.unread) void acted(() => notificationsRead(notice.id));
    switch (notice.action) {
      case "hand-over": {
        const session = liveOn(sessions, notice.target.account_id);
        if (!session) return open(notice);
        host.openSessionById?.(session.id);
        window.setTimeout(() => window.dispatchEvent(new Event("aij:focus-handover")), 0);
        return;
      }
      case "limit":
        if (notice.target.session_id) return void window.dispatchEvent(new CustomEvent("aij:limit", { detail: { sessionId: notice.target.session_id } }));
        return open(notice);
      case "sign-in":
        return notice.target.account_id ? host.openAccountById?.(notice.target.account_id) : open(notice);
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
    const back = `Back at ${clock(at)}${until === "tomorrow" ? " tomorrow" : ""}; out of Needs you until then.`;
    void acted(() => notificationsSnooze(notice.subject, at), "Couldn't snooze it").then((done) => done && showToast({ id: `snoozed:${notice.subject}`, message: until === "hour" ? "Snoozed for an hour" : "Snoozed until tomorrow", detail: back, undo: { onUndo: () => acted(() => notificationsSnooze(notice.subject, null)) } }));
  };
  const wake = (notice: NoticeInfo) => void acted(() => notificationsSnooze(notice.subject, null), "Couldn't wake it");
  const mute = (notice: NoticeInfo, scope: "project" | "kind") => {
    const value = scope === "project" ? notice.project_id : notice.kind;
    if (!value) return;
    void acted(() => notificationsMute(scope, value, scope === "project" ? (notice.project ?? value) : ""), "Couldn't mute it").then(
      (done) => done && showToast({ id: `muted:${scope}:${value}`, message: scope === "project" ? `Muted ${notice.project ?? "this project"}` : "Muted this kind", detail: "It stops interrupting; Needs you still counts it. Unmute in Notification settings." }),
    );
  };
  const second = (notice: NoticeInfo, which: Second) => {
    if (which === "snooze-hour") return snooze(notice, "hour");
    if (which === "mark-reviewed" && notice.target.session_id) {
      const id = notice.target.session_id;
      void instanceMarkReviewed(id).then(
        () => showToast({ id: `reviewed:${id}`, message: "Marked reviewed", detail: notice.title, undo: { onUndo: () => instanceUnmarkReviewed(id) } }),
        (cause) => showToast({ tone: "error", message: "Couldn't mark it reviewed", detail: errorText(cause), action: { label: "Retry", run: () => second(notice, which) } }),
      );
      return;
    }
    if (which === "queue-reset") {
      const account = notice.target.account_id;
      const at = notice.facts.resets_at_ms;
      const live = sessions.filter((s) => s.account_id === account && s.status !== "finished" && s.status !== "stopped" && s.status !== "failed");
      if (!account || at === null || live.length === 0) {
        showToast({ id: "queue-reset", message: "Nothing to queue", detail: "No session is using it now." });
        return;
      }
      void Promise.all(live.map((s) => resetQueueQueue(s.id, account, at))).then(
        () => showToast({ id: "queue-reset", message: "Queued for the reset", detail: `At ${clock(at)} you'll be offered to carry on. Nothing restarts by itself.` }),
        (cause) => showToast({ tone: "error", message: "Couldn't queue it", detail: errorText(cause), action: { label: "Retry", run: () => second(notice, which) } }),
      );
    }
  };
  return { primary, second, snooze, wake, mute };
}
