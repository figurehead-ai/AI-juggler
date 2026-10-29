/**
 * How long ago, in the rail's short words: "now", "4 min", "2 h", "3 d".
 */
export function ago(ms: number, now: number): string {
  const m = Math.max(0, Math.round((now - ms) / 60_000));
  if (m < 1) return "now";
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return h < 24 ? `${h} h` : `${Math.floor(h / 24)} d`;
}

/** How long, as Home says it: "just now", "4 min", "2 h", "3 d". */
export function minutes(sinceMs: number, now: number): string {
  const m = Math.max(0, Math.round((now - sinceMs) / 60_000));
  if (m < 1) return "just now";
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return h < 24 ? `${h} h` : `${Math.floor(h / 24)} d`;
}

/** How long ago, short: "40s", "12m", "30h", then days past 48 h ("7d"). */
export function since(ms: number, now: number): string {
  const secs = Math.max(1, Math.floor((now - ms) / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 48 * 3600) return `${Math.floor(secs / 3600)}h`;
  return `${Math.floor(secs / 86_400)}d`;
}

/** A session row's "when", the same everywhere (the rail, a project's or
 * an account's list): an ended one since its last activity ("3 h ago"); a
 * live one since what it's doing now began (waiting, stuck), else since
 * it started ("40 min"). */
export function sessionWhen(
  session: { status: string; started_at_ms: number; last_activity_ms: number | null; attention?: { since_ms: number } | null },
  now: number,
): string {
  const ended = ["finished", "stopped", "handed_over"].includes(session.status);
  const since = ended ? (session.last_activity_ms ?? session.started_at_ms) : (session.attention?.since_ms ?? session.started_at_ms);
  return ended ? agoWords(since, now) : ago(since, now);
}

/** How long ago, in words: "just now", else "3 h ago" (never "now ago"). */
export function agoWords(ms: number, now: number): string {
  const short = ago(ms, now);
  return short === "now" ? "just now" : `${short} ago`;
}

/** A row's day in words: Today, Yesterday, else "Thursday 23 October"
 * (with its year when not this one). */
export function dayWords(ms: number, now: number): string {
  const start = (at: number) => {
    const day = new Date(at);
    day.setHours(0, 0, 0, 0);
    return day.getTime();
  };
  const days = Math.round((start(now) - start(ms)) / 86_400_000);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  const at = new Date(ms);
  const sameYear = at.getFullYear() === new Date(now).getFullYear();
  return at.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long", ...(sameYear ? {} : { year: "numeric" }) });
}

/** "3:10 PM" (or "15:10"): a time of day as the system's own clock writes
 * it — never a hard-coded 12 or 24 hours, never a leading zero beside AM. */
export function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

/** Whole calendar days from `now`'s day to `ms`'s (0 today, 1 tomorrow). */
function daysAhead(ms: number, now: number): number {
  const start = (at: number) => {
    const day = new Date(at);
    day.setHours(0, 0, 0, 0);
    return day.getTime();
  };
  return Math.round((start(ms) - start(now)) / 86_400_000);
}

/** A time to come, by its day — the one rule every reset (and every time
 * one is waited for) is written by (the 0.2 walk, F18, and the 0.2.0
 * re-render: "08:13" and "Sun 07:13 AM" in one dialog): "3:10 PM" today,
 * "8:00 AM tomorrow", "Thu 8:00 AM" within the week, and with its date
 * from a week on ("Thu, Nov 5 8:00 AM"), when the weekday alone would
 * read as today's. */
export function whenWords(ms: number, now: number = Date.now()): string {
  const days = daysAhead(ms, now);
  if (days === 0) return clock(ms);
  if (days === 1) return `${clock(ms)} tomorrow`;
  const day = new Date(ms).toLocaleDateString([], days > 1 && days < 7 ? { weekday: "short" } : { weekday: "short", month: "short", day: "numeric" });
  return `${day} ${clock(ms)}`;
}

/** When a limit resets: "resets at 3:10 PM", "resets at 8:00 AM
 * tomorrow", "resets Thu 8:00 AM". */
export function resetsWords(ms: number, now: number = Date.now()): string {
  const days = daysAhead(ms, now);
  return `resets ${days === 0 || days === 1 ? "at " : ""}${whenWords(ms, now)}`;
}

/** The same, heading a line: "Resets at 3:10 PM". */
export function resetsLead(ms: number, now: number = Date.now()): string {
  const words = resetsWords(ms, now);
  return words.charAt(0).toUpperCase() + words.slice(1);
}
