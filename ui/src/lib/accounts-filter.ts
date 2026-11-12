/**
 * 0.4.2 (D-100 round): the top bar's "4 accounts out" lands on what it
 * says: Accounts, on its Accounts tab, showing only the out ones, with
 * an "Out · 4" chip that clears with ×; "2 accounts low" the same with
 * the low ones.
 *
 * The filter travels by its id (`lib/landing`, actions-target-by-id):
 * the click opens the screen with `{ id: "accounts", filter: "out" }`,
 * the window's navigation keeps it, and the screen draws from that.
 * Opening Accounts any other way opens it unfiltered.
 */
import type { SubscriptionInfo } from "./subscriptions";
import { accountMark } from "./room-mark";
import { openScreenOn } from "./landing";
import type { QuotaDisplayData } from "./ipc/quota";

/** A filter of the Accounts screen, by its id: the room its mark says. */
export type AccountsFilter = "out" | "low";

/** `value` as a filter's id; `null` for none (an event can carry anything). */
export function accountsFilterOf(value: unknown): AccountsFilter | null {
  return value === "out" || value === "low" ? value : null;
}

/** The filter's chip word: "Out", "Low" (the mark's own short words). */
export function filterWord(filter: AccountsFilter): string {
  return filter === "out" ? "Out" : "Low";
}

/** Opens Accounts with `filter` (`null`: unfiltered). */
export function openAccounts(filter: AccountsFilter | null = null): void {
  openScreenOn("accounts", filter);
}

/** An Account's room by the one rule the top bar's count uses
 * (`roomCounts`): signed in, and what its mark says; `null` with room. */
export function accountRoom(account: SubscriptionInfo, displays: readonly QuotaDisplayData[], now: number = Date.now()): AccountsFilter | null {
  return accountMark(account, displays, now)?.kind ?? null;
}

/** The Accounts the screen shows: with a filter, only the ones whose
 * room it names; with none, every one, the out ones first (each side in
 * its own order). */
export function accountsShown<T>(rows: readonly T[], room: (row: T) => AccountsFilter | null, filter: AccountsFilter | null): T[] {
  if (filter) return rows.filter((row) => room(row) === filter);
  return [...rows.filter((row) => room(row) === "out"), ...rows.filter((row) => room(row) !== "out")];
}
