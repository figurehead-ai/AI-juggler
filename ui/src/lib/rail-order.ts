/**
 * D-47 — Open now's order, as pure rules (the store that keeps it is
 * `rail-order-store.ts`). Three orders, picked on Open now's header and in
 * Settings › General:
 *
 * - **Your order** (the default): a row keeps its place. A new one joins
 *   the end of its group, in the order it was first seen; a state change
 *   only changes its dot and word; an ended one frees its slot and the
 *   rows under it close up. The user drags rows and places, or moves them
 *   with ⌥⌘↑ and ⌥⌘↓. A group shows every open row (the first 8 then
 *   "+n more" when it's long, never re-picked by activity) and folds only
 *   when the user folds it.
 * - **Latest activity**: the newest event first, held still under the
 *   pointer (AJ-E2104's rules, `foldGroup` and `frozenOrder`).
 * - **Name**: A to Z.
 *
 * Pinned rows lead their group in every order (AJ-E1502).
 *
 * The record is one small JSON value in AI Juggler's settings (so a
 * backup keeps it), read guarded. Rows are kept by the rail's own keys
 * (`s:<session>`, `c:<account>/<conversation>`); a key not seen for
 * `FORGET_DAYS` days is a session gone for good, and is pruned.
 */
import type { RailItem } from "./rail-items";
import { foldGroup, pinned, type GroupBy, type RailGroup } from "./rail-places";

export type OrderMode = "yours" | "latest" | "name";
export const ORDER_MODES: readonly OrderMode[] = ["yours", "latest", "name"];

/** The lenses whose groups the user can arrange (Project is the board's). */
export type OrderLens = Exclude<GroupBy, "project">;
const LENSES: readonly OrderLens[] = ["place", "account", "pool"];

export interface RailOrder {
  mode: OrderMode;
  /** Every row's key in the user's order: first seen, then as moved. */
  rows: string[];
  /** Each lens's groups in the user's order. */
  groups: Record<OrderLens, string[]>;
  /** The groups the user folded (`<lens>:<key>`), Your order and Name. */
  folded: string[];
  /** Pins of sessions other apps run (an AI Juggler session's pin is its own). */
  pins: string[];
  /** The day (since 1970) each row and pin was last seen. */
  seen: Record<string, number>;
}

export const EMPTY_ORDER: RailOrder = { mode: "yours", rows: [], groups: { place: [], account: [], pool: [] }, folded: [], pins: [], seen: {} };

/** A group shows this many rows in Your order and Name, then "+n more". */
export const LONG_GROUP = 8;
/** A row or pin unseen this long is gone for good. */
export const FORGET_DAYS = 30;
const MAX_ROWS = 600;
const MAX_GROUPS = 80;
const MAX_FOLDED = 120;
const MAX_KEY = 400;

/** The words the order menu and Settings use. */
export const ORDER_WORDS: Record<OrderMode, { label: string; what: string }> = {
  yours: { label: "Your order", what: "Stays put. New ones join at the end. Drag or pin to arrange." },
  latest: { label: "Latest activity", what: "Newest event first; holds still under the pointer." },
  name: { label: "Name", what: "A to Z." },
};

const isKey = (value: unknown): value is string => typeof value === "string" && value.length > 0 && value.length <= MAX_KEY;
const keys = (value: unknown, max: number): string[] => (Array.isArray(value) ? [...new Set(value.filter(isKey))].slice(-max) : []);

/** A stored record, checked field by field: junk is the defaults. */
export function parseOrder(raw: string | null): RailOrder {
  if (!raw) return EMPTY_ORDER;
  try {
    const value = JSON.parse(raw) as Partial<Record<keyof RailOrder, unknown>> | null;
    if (typeof value !== "object" || value === null || Array.isArray(value)) return EMPTY_ORDER;
    const mode = ORDER_MODES.includes(value.mode as OrderMode) ? (value.mode as OrderMode) : "yours";
    const stored = (typeof value.groups === "object" && value.groups !== null ? value.groups : {}) as Record<string, unknown>;
    const groups = { place: keys(stored.place, MAX_GROUPS), account: keys(stored.account, MAX_GROUPS), pool: keys(stored.pool, MAX_GROUPS) };
    const seen: Record<string, number> = {};
    if (typeof value.seen === "object" && value.seen !== null && !Array.isArray(value.seen)) {
      for (const [key, day] of Object.entries(value.seen as Record<string, unknown>)) {
        if (isKey(key) && typeof day === "number" && Number.isInteger(day) && day >= 0) seen[key] = day;
      }
    }
    return { mode, rows: keys(value.rows, MAX_ROWS), groups, folded: keys(value.folded, MAX_FOLDED), pins: keys(value.pins, MAX_ROWS), seen };
  } catch {
    return EMPTY_ORDER;
  }
}

/** Days since 1970, local time: the unit `seen` counts in. */
export function dayOf(ms: number): number {
  const at = new Date(ms);
  return Math.floor((ms - at.getTimezoneOffset() * 60_000) / 86_400_000);
}

export interface Present {
  /** Every item the rail knows now (open, ended, inactive): their keys. */
  known: readonly string[];
  /** Open now's items: a new one joins the end, by when it started. */
  open: readonly Pick<RailItem, "key" | "startMs">[];
  /** The lens shown, and its groups in the default order. */
  lens: OrderLens;
  groups: readonly string[];
  today: number;
  /** Groups known by another key now (an old place key and the App it
   * became, D-62): the order and the folds follow them. */
  renamed?: ReadonlyMap<string, string>;
}

/**
 * The record brought up to date: new open rows and new groups join the
 * end; rows and pins seen today say so (one write a day at most); one
 * unseen for `FORGET_DAYS` is pruned. `changed` says whether to keep it.
 */
export function reconcile(order: RailOrder, present: Present): { order: RailOrder; changed: boolean } {
  let changed = false;
  const rows = [...order.rows];
  const inRows = new Set(rows);
  const fresh = present.open.filter((item) => !inRows.has(item.key));
  fresh.sort((a, b) => a.startMs - b.startMs || a.key.localeCompare(b.key));
  for (const item of fresh) {
    if (inRows.has(item.key)) continue;
    rows.push(item.key);
    inRows.add(item.key);
    changed = true;
  }
  const seen = { ...order.seen };
  const known = new Set(present.known);
  for (const key of [...rows, ...order.pins]) {
    if ((known.has(key) || seen[key] === undefined) && seen[key] !== present.today) {
      seen[key] = present.today;
      changed = true;
    }
  }
  const gone = (key: string) => present.today - (seen[key] ?? present.today) > FORGET_DAYS;
  let kept = rows.filter((key) => !gone(key));
  const pins = order.pins.filter((key) => !gone(key));
  // Over the cap: the oldest that aren't here now go first.
  while (kept.length > MAX_ROWS) {
    const at = kept.findIndex((key) => !known.has(key));
    kept = at < 0 ? kept.slice(1) : [...kept.slice(0, at), ...kept.slice(at + 1)];
  }
  const held = new Set([...kept, ...pins]);
  for (const key of Object.keys(seen)) {
    if (!held.has(key)) delete seen[key];
  }
  if (kept.length !== rows.length || pins.length !== order.pins.length) changed = true;
  let lensGroups = [...order.groups[present.lens]];
  let folded = order.folded;
  const renamed = present.renamed;
  if (renamed && renamed.size > 0 && lensGroups.some((key) => renamed.has(key))) {
    lensGroups = [...new Set(lensGroups.map((key) => renamed.get(key) ?? key))];
    changed = true;
  }
  if (renamed && renamed.size > 0) {
    const prefix = `${present.lens}:`;
    const next = [...new Set(folded.map((key) => (key.startsWith(prefix) ? foldKey(present.lens, renamed.get(key.slice(prefix.length)) ?? key.slice(prefix.length)) : key)))];
    if (next.length !== folded.length || next.some((key, at) => key !== folded[at])) {
      folded = next;
      changed = true;
    }
  }
  for (const key of present.groups) {
    if (!lensGroups.includes(key)) {
      lensGroups.push(key);
      changed = true;
    }
  }
  while (lensGroups.length > MAX_GROUPS) {
    const at = lensGroups.findIndex((key) => !present.groups.includes(key));
    lensGroups = at < 0 ? lensGroups.slice(1) : [...lensGroups.slice(0, at), ...lensGroups.slice(at + 1)];
  }
  return { order: { ...order, rows: kept, pins, seen, folded, groups: { ...order.groups, [present.lens]: lensGroups } }, changed };
}

/** Each key's place in a list (missing ones are not in it). */
export function rankOf(list: readonly string[]): Map<string, number> {
  return new Map(list.map((key, index) => [key, index]));
}

const byStart = (a: RailItem, b: RailItem) => a.startMs - b.startMs || a.key.localeCompare(b.key);
const byName = (a: RailItem, b: RailItem) => a.title.localeCompare(b.title, undefined, { numeric: true, sensitivity: "base" }) || a.key.localeCompare(b.key);

/** A group's rows in an order, pinned ones first; in Your order a row
 * the record doesn't hold yet comes after the ones it does, by when it
 * started (the store adds it at the end next). */
export function sortRows(items: readonly RailItem[], mode: Exclude<OrderMode, "latest">, rank: ReadonlyMap<string, number>): RailItem[] {
  const by =
    mode === "name"
      ? byName
      : (a: RailItem, b: RailItem) => (rank.get(a.key) ?? Infinity) - (rank.get(b.key) ?? Infinity) || byStart(a, b);
  return [...items].sort((a, b) => Number(pinned(b)) - Number(pinned(a)) || by(a, b));
}

/** What a group shows: its rows in the order, and how many it leaves out
 * (Your order and Name: "+n more" past `LONG_GROUP`; Latest activity:
 * "+n quiet", as before). `all` is every row in the order. */
export function arrangeGroup(
  group: RailGroup,
  mode: OrderMode,
  rank: ReadonlyMap<string, number>,
  showAll: boolean,
): { all: RailItem[]; shown: RailItem[]; hidden: number; words: "more" | "quiet" } {
  if (mode === "latest") {
    const fold = foldGroup(group, showAll);
    const all = fold.quiet > 0 ? foldGroup(group, true).shown : fold.shown;
    return { all, shown: fold.shown, hidden: fold.quiet, words: "quiet" };
  }
  const all = sortRows(group.items, mode, rank);
  if (showAll || all.length <= LONG_GROUP) return { all, shown: all, hidden: 0, words: "more" };
  return { all, shown: all.slice(0, LONG_GROUP), hidden: all.length - LONG_GROUP, words: "more" };
}

/** The groups in the order: Your order by the user's arrangement (new
 * ones after, in the default order); the others in the default order
 * (AI Juggler's own place first, then by name). */
export function arrangeGroups(groups: readonly RailGroup[], mode: OrderMode, mine: readonly string[]): RailGroup[] {
  if (mode !== "yours") return [...groups];
  const rank = rankOf(mine);
  return groups
    .map((group, index) => ({ group, index }))
    .sort((a, b) => (rank.get(a.group.key) ?? Infinity) - (rank.get(b.group.key) ?? Infinity) || a.index - b.index)
    .map((entry) => entry.group);
}

/**
 * `list` with one of `shown` moved from `from` to `to`, everything else
 * where it was: the slots `shown`'s keys held in `list` are filled again
 * in the new order. Keys `list` lacks join its end first.
 */
export function moveIn(list: readonly string[], shown: readonly string[], from: number, to: number): string[] {
  const out = [...list];
  for (const key of shown) if (!out.includes(key)) out.push(key);
  if (from === to || from < 0 || from >= shown.length) return out;
  const next = [...shown];
  const [moved] = next.splice(from, 1);
  next.splice(Math.max(0, Math.min(to, next.length)), 0, moved!);
  const slots = shown.map((key) => out.indexOf(key)).sort((a, b) => a - b);
  slots.forEach((slot, index) => {
    out[slot] = next[index]!;
  });
  return out;
}

/** Where a row may go: among the pinned ones when it's pinned, else among
 * the others (pinned rows always lead). `[first, last]` positions. */
export function rowRange(all: readonly Pick<RailItem, "pinned">[], index: number): [number, number] {
  const pins = all.filter((item) => item.pinned).length;
  return all[index]?.pinned ? [0, Math.max(0, pins - 1)] : [pins, all.length - 1];
}

/** "Moved to position 2 of 5". */
export function movedWords(to: number, of: number): string {
  return `Moved to position ${to + 1} of ${of}`;
}

/** The record with its arrangement forgotten (Reset my order): rows and
 * places go back to when each was first seen and the default order. Pins,
 * folds and the chosen order stay. */
export function resetOrder(order: RailOrder): RailOrder {
  return { ...order, rows: [], groups: { place: [], account: [], pool: [] } };
}

/** A fold's key in the record. */
export function foldKey(lens: OrderLens, group: string): string {
  return `${lens}:${group}`;
}

/** The lens a View's Group by arranges (Project has its own board). */
export function lensOf(group: GroupBy): OrderLens {
  return LENSES.includes(group as OrderLens) ? (group as OrderLens) : "place";
}
