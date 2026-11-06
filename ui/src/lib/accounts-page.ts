/**
 * D-62 (AJ-E3205) — opening an Account's page (a subscription: its
 * limits, its Apps, sessions and projects) from anywhere that names one.
 */
import { rememberTab, type PageTabId } from "../pages/registry";

/** The event App listens for. */
export const OPEN_ACCOUNT = "aij:open-account";

/** Opens Account `id`'s page (`acct:<vendor>:<hash>`); on its tab `tab`
 * when given (an App's "Set on its account" opens its Settings). */
export function openAccount(id: string, tab?: PageTabId): void {
  if (tab) rememberTab("subscription", tab);
  window.dispatchEvent(new CustomEvent<{ id: string }>(OPEN_ACCOUNT, { detail: { id } }));
}

