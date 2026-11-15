/**
 * AJ-E3011 round 2 (#3): "Start one now" for an App whose next session
 * leads a pool. It opens the usual New session, prefilled with that App
 * and the pool's project; the user starts it (never without that click),
 * and the session then leads (the pool's `lead_next`, claimed at spawn).
 * Only a CLI App starts here: a desktop App's sessions start in its own
 * window.
 */

/** A CLI App member's config folder (`app:cli:<folder>`), the New
 * session sheet's account; `null` for any other member. */
export function cliFolderOf(member: string): string | null {
  return member.startsWith("app:cli:") ? member.slice("app:cli:".length) || null : null;
}

/** The pool's one project folder, when it has exactly one (else the
 * sheet asks where). */
export function leadFolder(pool: { scope_all: boolean; scope: readonly (string | null)[] }, projects: readonly { id: string; root?: string }[]): string | null {
  if (pool.scope_all || pool.scope.length !== 1 || pool.scope[0] === null) return null;
  return projects.find((one) => one.id === pool.scope[0])?.root ?? null;
}

/** Opens New session on `member`'s App in `dir`. */
export function startNextLead(member: string, dir: string | null): void {
  const accountId = cliFolderOf(member);
  if (!accountId) return;
  window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId, dir } }));
}
