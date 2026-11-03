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
 * (`ACCOUNT_HUE_OFFSET`). Both draw from `MARK_COLOURS` (lib/mark-hues:
 * clear of the status colours, letters AA in `markInk`), so neither is
 * read as a state.
 *
 * Every call site goes through `appColor` and `accountColor`. The rows
 * that only know an id (a session's folder, a rail place's key) are
 * resolved through what the loaded lists said (`learnFolders`,
 * `learnPlaces`), as project marks are (`withMarks`); the mark components
 * re-render when they learn something (`useMarks`).
 */
import { useSyncExternalStore } from "react";
import { withPalette } from "./palette";
import type { AccountInfo } from "./bridge";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import { cliAppId, cliAppKey, desktopAppKey, ownDesktopAppKey } from "./app-name";
import { MARK_COLOURS } from "./mark-hues";
import { contrast, deltaE, luminance, type Oklch } from "./oklch";

export { MARK_COLOURS, STATUS_HUE_GAPS, STATUS_MIN_DELTA_E } from "./mark-hues";

/** A slot's colour (the same in both themes). */
export function slotOklch(slot: number): Oklch {
  const n = MARK_COLOURS.length;
  return MARK_COLOURS[((Math.trunc(slot) % n) + n) % n]!;
}

function slotColor(slot: number): string {
  const { l, c, h } = slotOklch(slot);
  return `oklch(${l.toFixed(3)} ${c.toFixed(3)} ${h})`;
}

/** The letters on a mark: white on the darker colours, dark on the lighter
 * ones, whichever is AA (the same ink in both themes, as the colour is). */
export const MARK_INK_DARK = "#0e1219";
export function markInk(slot: number): string {
  return contrast(luminance(slotOklch(slot)), 1) >= 4.5 ? "#ffffff" : MARK_INK_DARK;
}

/** The fallback for something not stored yet, made from its id: among the
 * slots `taken` leaves free while any is, the one most unlike every taken
 * colour (so it never wears, or nearly wears, the colour of one that is
 * stored), its id breaking ties; with none free, any slot by its id. */
export function fallbackSlot(id: string, taken: ReadonlySet<number> = new Set()): number {
  let hash = 0;
  for (let i = 0; i < id.length; i += 1) hash = (hash * 31 + id.charCodeAt(i)) % 100_000;
  const free = MARK_COLOURS.map((_, slot) => slot).filter((slot) => !taken.has(slot));
  if (free.length === 0) return hash % MARK_COLOURS.length;
  if (taken.size === 0) return free[hash % free.length]!;
  const apart = (slot: number) => Math.min(...[...taken].map((other) => deltaE(slotOklch(slot), slotOklch(other))));
  const best = Math.max(...free.map(apart));
  const most = free.filter((slot) => apart(slot) >= best - 1e-9);
  return most[hash % most.length]!;
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

/** The slot stored for `key`, else, for a desktop session AI Juggler
 * started on a folder (`app|<host>|folder:<id>`) with no desktop App
 * remembered there, its folder's App's: the App every list names it by
 * (the 0.4 review: one App, one colour on every screen). */
function storedSlot(key: string): number | undefined {
  const own = appSlots.get(key);
  if (own !== undefined) return own;
  const folder = /\|folder:(.+)$/.exec(key)?.[1];
  return folder ? appSlots.get(cliAppId(folder)) : undefined;
}

/** An App's slot: the one given, else the one stored for its id or key,
 * else the fallback from its id. */
export function appSlot(app: AppMark): number {
  if (typeof app === "number") return app;
  return storedSlot(app) ?? fallbackSlot(app, new Set(appSlots.values()));
}

/** Whether an App's colour is its own: given or stored, or only the
 * fallback from its id (a key no loaded list named). */
export function appSlotKnown(app: AppMark): boolean {
  return typeof app === "number" || storedSlot(app) !== undefined;
}

/** An App's one colour, as CSS. */
export function appColor(app: AppMark): string {
  return slotColor(appSlot(app));
}

/** The letters on an App's pill. */
export function appInk(app: AppMark): string {
  return markInk(appSlot(app));
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
export const ACCOUNT_HUE_OFFSET = MARK_COLOURS.length / 2;

/** An Account's one colour, as CSS. */
export function accountColor(account: AccountMark): string {
  return slotColor(accountSlot(account) + ACCOUNT_HUE_OFFSET);
}

/** The letters on an Account's avatar. */
export function accountInk(account: AccountMark): string {
  return markInk(accountSlot(account) + ACCOUNT_HUE_OFFSET);
}
