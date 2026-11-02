/**
 * D-62 (AJ-E3205) — opening an Account's page (a subscription: its
 * limits, its Apps, sessions and projects) from anywhere that names one.
 */

/** The event App listens for. */
export const OPEN_ACCOUNT = "aij:open-account";

/** Opens Account `id`'s page (`acct:<vendor>:<hash>`). */
export function openAccount(id: string): void {
  window.dispatchEvent(new CustomEvent<{ id: string }>(OPEN_ACCOUNT, { detail: { id } }));
}
