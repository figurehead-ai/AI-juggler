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
