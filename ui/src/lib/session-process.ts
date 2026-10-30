/**
 * Whether a session's process is gone (its terminal reads Ended), from its
 * status and attention rule. Waiting for you or stuck is still alive; one
 * at its limit is gone only when it ended there (0.3: an interactive tool
 * stays open at its limit, rule `limit_hit`).
 */
const PROCESS_GONE = new Set(["finished", "stopped", "failed", "handed_over", "quota_blocked"]);

export function processGone(status: string | undefined, rule: string | null | undefined): boolean {
  if (status === undefined) return false;
  if (status === "quota_blocked") return rule !== "limit_hit";
  return PROCESS_GONE.has(status);
}
