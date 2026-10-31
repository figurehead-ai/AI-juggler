/**
 * AJ-E2503 — how a notice reads: its primary action's name, its second
 * one, its lines in the user's own time ("resets at 9:09 PM", "waiting
 * 24 min"), the digest, and the muted line. Plain words only: the backend
 * sends sentences and facts, never an id to show.
 */
import type { NoticeInfo } from "../types/generated/NoticeInfo";
import type { NoticeDigestInfo } from "../types/generated/NoticeDigestInfo";
import { clock, minutes, resetsWords, whenWords } from "./time-words";

/** When something happened: today a time, else its day. */
export function happenedAt(ms: number, now: number): string {
  const day = new Date(ms).toDateString() === new Date(now).toDateString();
  return day ? clock(ms) : new Date(ms).toLocaleDateString([], { weekday: "short", day: "numeric", month: "short" });
}

/** An account almost out of room: room left, the reset, the sessions on it.
 * One carrying on with its credits (0.3.2): "Its weekly window is used up
 * until Wed 1:35 AM · 14,150 credits left". */
export function quotaDetail(notice: NoticeInfo, now: number): string {
  const { window, resets_at_ms, sessions, credits } = notice.facts;
  if (credits != null) {
    const until = resets_at_ms !== null && resets_at_ms > now ? ` until ${whenWords(resets_at_ms, now)}` : "";
    const parts = [`Its ${window ?? "usage"} window is used up${until}`];
    if (credits) parts.push(credits);
    if (sessions) parts.push(sessions === 1 ? "1 session uses it" : `${sessions} sessions use it`);
    return parts.join(" · ");
  }
  const parts = [notice.meter !== null ? `${notice.meter}% of its ${window ?? "usage"} window left` : notice.detail];
  if (resets_at_ms !== null && resets_at_ms > now) parts.push(resetsWords(resets_at_ms, now));
  if (sessions) parts.push(sessions === 1 ? "1 session uses it" : `${sessions} sessions use it`);
  return parts.join(" · ");
}

/** The line under a need's title. */
export function needLine(notice: NoticeInfo, now: number): string {
  // D-49: snoozed, it says when it comes back into Needs you.
  if (notice.snoozed) return notice.snoozed_until_ms === null ? "Snoozed until you wake it" : `Snoozed · back at ${whenWords(notice.snoozed_until_ms, now)}`;
  if (notice.kind === "quota") return quotaDetail(notice, now);
  if (notice.kind === "asks" || notice.kind === "stuck") {
    const waited = minutes(notice.created_ms, now);
    if (waited === "just now") return "just now";
    return notice.kind === "asks" ? `waiting ${waited}` : `for ${waited}`;
  }
  return notice.detail;
}

/** What happened, under Earlier: its detail; an account carrying on with
 * its credits says until when, in the user's own time. */
export function earlierLine(notice: NoticeInfo, now: number): string {
  return notice.kind === "quota" && notice.facts.credits != null ? quotaDetail(notice, now) : notice.detail;
}

/** The quieter line under that (where; or the sessions on an account). */
export function needMeta(notice: NoticeInfo): string | null {
  if (notice.kind === "quota") return null;
  if (notice.kind === "asks" || notice.kind === "stuck") return notice.detail || null;
  return null;
}

const PRIMARY: Record<string, string> = {
  // D-05's name (the 0.2 walk, F16: it opened the account page).
  "hand-over": "Continue on another account…",
  "see-why": "See why",
  "whats-new": "What's new",
  "sign-in": "Sign in…",
  "this-computer": "See what's using it",
  "clean-up": "Clean up",
  usage: "See the cap",
  limit: "Continue elsewhere",
  // D-05 (0.3, wf27): an account with room again, for the work that left it.
  "switch-back": "Switch back…",
  pool: "Open",
};

/** The one primary action's name. */
export function primaryLabel(notice: NoticeInfo): string | null {
  if (!notice.action) return null;
  if (notice.action === "open") return notice.kind === "review" ? "Review" : "Open";
  return PRIMARY[notice.action] ?? "Open";
}

export type Second = "snooze-hour" | "remind-reset" | "mark-reviewed";

/** Its second action beside the primary, when the design gives it one. */
export function secondOf(notice: NoticeInfo): { id: Second; label: string } | null {
  if (!notice.needs) return null;
  // 0.3.2: "Queue for the reset" said nothing about what it does. A
  // reminder at the reset, offered only when the reset time is known.
  if (notice.kind === "quota") return notice.facts.resets_at_ms != null ? { id: "remind-reset", label: remindLabel(notice.facts.resets_at_ms) } : null;
  if (notice.kind === "review" && notice.target.session_id) return { id: "mark-reviewed", label: "Mark reviewed" };
  if (!notice.snoozed && ["asks", "stuck", "message", "task"].includes(notice.kind)) return { id: "snooze-hour", label: "Snooze 1 h" };
  return null;
}

/** A notice's look: an account carrying on with its credits informs
 * (0.3.2), it doesn't warn. */
export function noticeTone(notice: Pick<NoticeInfo, "kind" | "facts">): ReturnType<typeof toneOf> {
  return notice.kind === "quota" && notice.facts.credits != null ? "info" : toneOf(notice.kind);
}

/** "Remind me at 9:40 PM": what the reminder button says. */
export function remindLabel(resetsAtMs: number, now: number = Date.now()): string {
  return `Remind me at ${whenWords(resetsAtMs, now)}`;
}

/** A row's look: what its mark says. */
export function toneOf(kind: string): "ask" | "warn" | "ok" | "err" | "info" {
  if (kind === "asks" || kind === "message" || kind === "task" || kind === "signin") return "ask";
  if (kind === "quota" || kind === "stuck" || kind === "limit" || kind === "room" || kind === "heavy" || kind === "leftovers" || kind === "cap") return "warn";
  if (kind === "review" || kind === "finished") return "ok";
  if (kind === "failed" || kind === "restarted") return "err";
  return "info";
}

/** One hour from now. */
export const HOUR_MS = 3_600_000;

/** Tomorrow at 09:00, local time. */
export function tomorrowMorning(now: number): number {
  const at = new Date(now);
  at.setDate(at.getDate() + 1);
  at.setHours(9, 0, 0, 0);
  return at.getTime();
}

/** The digest line: "Today: 4 finished · 1 restarted · Work Max resets at 9:09 PM". */
export function digestLine(digest: NoticeDigestInfo, now: number): string | null {
  const parts: string[] = [];
  if (digest.finished) parts.push(`${digest.finished} finished`);
  if (digest.restarted) parts.push(`${digest.restarted} restarted`);
  if (digest.resets) parts.push(digest.resets === 1 ? "1 reset" : `${digest.resets} resets`);
  if (digest.updates) parts.push(digest.updates === 1 ? "1 update" : `${digest.updates} updates`);
  if (digest.next_reset_account && digest.next_reset_ms !== null && digest.next_reset_ms > now) parts.push(`${digest.next_reset_account} ${resetsWords(digest.next_reset_ms, now)}`);
  return parts.length > 0 ? `Today: ${parts.join(" · ")}` : null;
}

/** While muted (the quiet hours): what still holds. */
export function mutedLine(until: string, count: number): string {
  return `Muted until ${until} — Needs you still tracks ${count}.`;
}

/** The bell's name for screen readers. */
export function bellLabel(count: number, unread: number): string {
  const needs = count === 0 ? "nothing needs you" : count === 1 ? "1 thing needs you" : `${count} things need you`;
  return unread > 0 ? `Notifications: ${needs}, ${unread} unread` : `Notifications: ${needs}`;
}
