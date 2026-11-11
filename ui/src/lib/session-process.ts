/**
 * Whether a session's process is gone (its terminal reads Ended), from its
 * status and attention rule. Waiting for you or stuck is still alive; one
 * at its limit is gone only when it ended there (0.3: an interactive tool
 * stays open at its limit, rule `limit_hit`; `quota_exit` and the exit
 * classifier's mark are a process that ended at its limit).
 */
const PROCESS_GONE = new Set(["finished", "stopped", "failed", "handed_over", "quota_blocked"]);

export function processGone(status: string | undefined, rule: string | null | undefined): boolean {
  if (status === undefined) return false;
  if (status === "quota_blocked") return rule !== "limit_hit";
  return PROCESS_GONE.has(status);
}

/** The statuses of a session that may still have its process. */
const ACTIVE = new Set(["starting", "running", "waiting_user", "stuck", "quota_blocked"]);

/**
 * Whether a session's process still runs: what Stop, "Go to it", the
 * "scrollback replayed" line and the terminal all read, so they never
 * disagree (0.4.1 baseline pass: a session that ended at its limit
 * offered Stop beside a terminal that said Ended). Its status alone isn't
 * enough: a limit is reached both open (`limit_hit`) and on exit.
 */
export function processRunning(session: { status: string; kind?: string; attention?: { rule: string } | null }): boolean {
  if (!ACTIVE.has(session.status)) return false;
  // A desktop session's process is its app's, which a limit never ends.
  if (session.kind === "desktop") return true;
  return !processGone(session.status, session.attention?.rule ?? null);
}
