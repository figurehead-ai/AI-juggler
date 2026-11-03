/**
 * AJ-E3206 req. 2 — one mark and one colour per App, and one per Account.
 *
 * Each App (a CLI tool on a folder, a desktop app in one of its profiles)
 * and each Account (a subscription) is given a colour slot once, by the
 * backend, when it's first stored (schema v42: the least-used slot, the
 * lowest on a tie). The slot travels on the rows that name it
 * (`AccountInfo.app_colour_slot` / `account_colour_slot`,
 * `KnownPlaceInfo.colour_slot`, `StoredAppInfo.colour_slot`,
 * `SubscriptionInfo.colour_slot`), so a colour never shifts as others are
 * added, renamed or re-sorted, and the same App is the same colour on
 * every screen. Something not stored yet (a place no App row matches)
 * gets a fallback made from its id, never from a name on screen.
 *
 * The two marks have different shapes so they're never confused: an App
 * is the dot or pill beside its app's icon (`ProfileBadge`), an Account
 * the round avatar (`AccountAvatar`), its hues started half way round
 * (`ACCOUNT_HUE_OFFSET`). Both draw from these hues: one
 * luminance, white text 4.5:1 or better on each, kept 20° or more away
 * from the status colours, so neither is read as a state.
 *
 * Every call site goes through `appColor` and `accountColor`. The rows
 * that only know an id (a session's folder, a rail place's key) are
 * resolved through what the loaded lists said (`learnFolders`,
 * `learnPlaces`), as project marks are (`withMarks`); the mark components
 * re-render when they learn something (`useMarks`).
 */
import { useSyncExternalStore } from "react";
import { swatchOf, withPalette } from "./palette";
import type { AccountInfo } from "./bridge";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import { cliAppId, cliAppKey, desktopAppKey, ownDesktopAppKey } from "./app-name";
import { PROFILE_HUES } from "./mark-hues";

export { PROFILE_HUES, STATUS_MIN_DELTA_E } from "./mark-hues";

/** A slot's colour, as CSS (the same in both themes). */
function slotColor(slot: number): string {
  const n = PROFILE_HUES.length;
  const { l, c, h } = swatchOf(PROFILE_HUES[((Math.trunc(slot) % n) + n) % n]!);
  return `oklch(${l.toFixed(3)} ${c.toFixed(3)} ${h})`;
}

/** The fallback for something not stored yet: a slot made from its id,
 * among the slots `taken` leaves free while any is (so it never wears the
 * colour of one that is stored), else among all of them. */
export function fallbackSlot(id: string, taken: ReadonlySet<number> = new Set()): number {
  let hash = 0;
  for (let i = 0; i < id.length; i += 1) hash = (hash * 31 + id.charCodeAt(i)) % 100_000;
  const free = PROFILE_HUES.map((_, slot) => slot).filter((slot) => !taken.has(slot));
  return free.length > 0 ? free[hash % free.length]! : hash % PROFILE_HUES.length;
}

// --- what the lists said ----------------------------------------------------

/** Each App's slot by its id (`cli:<folder>`, `app:<host>:<instance>`)
 * and by the rail's key for it (`cli|<tool>|<folder>`,
 * `app|<host>|<instance>`). */
const appSlots = new Map<string, number>();
/** Each folder's Account and that Account's slot. */
const folderAccounts = new Map<string, { id: string | null; slot: number | null }>();
let version = 0;
const listeners = new Set<() => void>();

function changed(): void {
  version += 1;
  for (const listener of listeners) listener();
}

function put(key: string, slot: number): boolean {
  if (appSlots.get(key) === slot) return false;
  appSlots.set(key, slot);
  return true;
}

/** The folders as loaded: each one's CLI App and Account. */
export function learnFolders(folders: readonly AccountInfo[]): void {
  let moved = false;
  for (const folder of folders) {
    if (folder.app_colour_slot != null) {
      moved = put(cliAppId(folder.id), folder.app_colour_slot) || moved;
      moved = put(cliAppKey(folder.provider, folder.id, folder.label), folder.app_colour_slot) || moved;
    }
    const was = folderAccounts.get(folder.id);
    const now = { id: folder.subscription_id ?? null, slot: folder.account_colour_slot ?? null };
    if (was?.id !== now.id || was?.slot !== now.slot) {
      folderAccounts.set(folder.id, now);
      moved = true;
    }
  }
  if (moved) changed();
}

/** The folders as loaded, each with its stored colours learnt and its
 * initials given (AJ-E3206, AJ-E1532). */
export function withFolderMarks(folders: AccountInfo[]): AccountInfo[] {
  learnFolders(folders);
  return withPalette(folders);
}

/** The desktop Apps remembered (the places), as loaded. */
export function learnPlaces(places: readonly KnownPlaceInfo[]): void {
  let moved = false;
  const folders = new Set<string>();
  // Newest first: a session AI Juggler started in a desktop app wears the
  // profile last seen on its folder (as lib/rail-apps groups it).
  for (const place of [...places].sort((a, b) => b.seen_ms - a.seen_ms)) {
    if (place.colour_slot == null) continue;
    moved = put(place.id, place.colour_slot) || moved;
    moved = put(desktopAppKey(place.app_id, place.app, place.instance), place.colour_slot) || moved;
    const own = place.account_id ? ownDesktopAppKey(place.app_id, place.app, place.account_id, "") : null;
    if (own && !folders.has(own)) {
      folders.add(own);
      moved = put(own, place.colour_slot) || moved;
    }
  }
  if (moved) changed();
}

/** Test seam: forget what the lists said. */
export function forgetMarksForTests(): void {
  appSlots.clear();
  folderAccounts.clear();
  changed();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Re-renders the caller when a list teaches a mark something. */
export function useMarks(): number {
  return useSyncExternalStore(subscribe, () => version);
}

// --- the two helpers --------------------------------------------------------

/** An App: its stored slot, else its id or the rail's key for it. */
export type AppMark = number | string;

/** An App's slot: the one given, else the one stored for its id or key,
 * else the fallback from its id. */
export function appSlot(app: AppMark): number {
  if (typeof app === "number") return app;
  return appSlots.get(app) ?? fallbackSlot(app, new Set(appSlots.values()));
}

/** Whether an App's colour is its own: given or stored, or only the
 * fallback from its id (a key no loaded list named). */
export function appSlotKnown(app: AppMark): boolean {
  return typeof app === "number" || appSlots.has(app);
}

/** An App's one colour, as CSS. */
export function appColor(app: AppMark): string {
  return slotColor(appSlot(app));
}

/** An Account: its stored slot; a row of the Accounts list (its slot, or
 * its id or key when not stored); or a folder, by the Account it was last
 * read as. */
export type AccountMark = number | { colour_slot?: number | null; account_id?: string | null; key: string } | { folder: string | null };

/** An Account's slot. A folder whose Account isn't known falls back by
 * the folder's id (its Account unknown, its colour still its own). */
export function accountSlot(account: AccountMark): number {
  if (typeof account === "number") return account;
  if ("folder" in account) {
    if (account.folder === null) return fallbackSlot("");
    const known = folderAccounts.get(account.folder);
    if (known?.slot != null) return known.slot;
    return fallbackSlot(known?.id ?? `folder:${account.folder}`);
  }
  return account.colour_slot ?? fallbackSlot(account.account_id ?? account.key);
}

/** Where an Account's hues start: half way round the list. Most people's
 * first folder is both their first CLI App and their first Account, so
 * both would be slot 0; started apart, an Account never wears the colour
 * of any of the first four Apps, and an App's dot under its Account's
 * avatar never reads as that Account's own. */
export const ACCOUNT_HUE_OFFSET = PROFILE_HUES.length / 2;

/** An Account's one colour, as CSS. */
export function accountColor(account: AccountMark): string {
  return slotColor(accountSlot(account) + ACCOUNT_HUE_OFFSET);
}
