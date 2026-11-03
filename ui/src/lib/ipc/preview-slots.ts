/**
 * AJ-E3206 — the preview's stand-in for the backend's colour rule (schema
 * v42), so the preview, the galleries and the visual tests draw each App
 * and Account in the colour the app would: a row the fixture gives no
 * slot takes the least-used one (the lowest on a tie), in the rows'
 * order, after the slots already taken. The rows are filled in place:
 * the preview's rows are shared, as the backend's are kept, so a slot
 * given once stays.
 */
import { MARK_COLOURS } from "../mark-hues";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";
import type { SubscriptionInfo } from "../../types/generated/SubscriptionInfo";

/** The least-used slot (the lowest on a tie) given `taken`. */
function leastUsed(taken: readonly number[]): number {
  const used = MARK_COLOURS.map((_, slot) => taken.filter((one) => one === slot).length);
  return used.indexOf(Math.min(...used));
}

/** Gives each row with no slot the least-used one, in order. */
export function fillSlots<T>(rows: readonly T[], get: (row: T) => number | null | undefined, set: (row: T, slot: number) => void, taken: readonly number[] = []): void {
  const used = [...taken, ...rows.map(get).filter((slot): slot is number => slot != null)];
  for (const row of rows) {
    if (get(row) != null) continue;
    const slot = leastUsed(used);
    set(row, slot);
    used.push(slot);
  }
}

/** The preview's id for an Account (the backend's is a hash of its
 * sign-in; the preview has none). */
export function previewAccountId(row: Pick<SubscriptionInfo, "account_id" | "key" | "state">): string | null {
  return row.account_id ?? (row.state === "signed_in" ? `acct:preview:${row.key}` : null);
}

/** The preview's folders with their CLI Apps' slots, and the Account
 * each is signed in to (the fixture's accounts, else one per signed-in
 * folder, as the preview's accounts list makes them) with its slot. */
export function previewFolderSlots(rows: AccountInfo[], subscriptions: readonly SubscriptionInfo[] | undefined, signedIn: (state: string) => boolean): AccountInfo[] {
  fillSlots(rows, (row) => row.app_colour_slot, (row, slot) => (row.app_colour_slot = slot));
  for (const row of rows) {
    if (row.subscription_id !== undefined) continue;
    const sub = subscriptions?.find((one) => one.state === "signed_in" && one.profiles.some((profile) => profile.id === row.id));
    row.subscription_id = sub ? previewAccountId(sub) : !subscriptions && signedIn(row.login_state) ? `acct:preview:k-${row.id}` : null;
  }
  const accounts = [...new Set(rows.map((row) => row.subscription_id).filter((id): id is string => !!id))].map((id) => ({
    id,
    slot: rows.find((row) => row.subscription_id === id && row.account_colour_slot != null)?.account_colour_slot ?? subscriptions?.find((sub) => previewAccountId(sub) === id)?.colour_slot ?? null,
  }));
  fillSlots(accounts, (account) => account.slot, (account, slot) => (account.slot = slot));
  for (const row of rows) {
    if (row.account_colour_slot != null) continue;
    row.account_colour_slot = row.subscription_id ? (accounts.find((account) => account.id === row.subscription_id)?.slot ?? null) : null;
  }
  return rows;
}

/** The preview's Accounts with their slots, as their folders have them. */
export function previewAccountSlots(rows: SubscriptionInfo[], folders: readonly AccountInfo[]): SubscriptionInfo[] {
  return rows.map((row) => {
    if (row.colour_slot != null) return row;
    const id = previewAccountId(row);
    const slot = id ? (folders.find((folder) => folder.subscription_id === id)?.account_colour_slot ?? null) : null;
    return { ...row, colour_slot: slot };
  });
}

/** The preview's desktop Apps with their slots, after the CLI Apps'. */
export function previewPlaceSlots(places: KnownPlaceInfo[], folders: readonly AccountInfo[]): KnownPlaceInfo[] {
  fillSlots(places, (place) => place.colour_slot, (place, slot) => (place.colour_slot = slot), folders.map((folder) => folder.app_colour_slot).filter((slot): slot is number => slot != null));
  return places;
}
