/**
 * AJ-E2104 — Open now's groups. By App (D-62: "Claude app · b-work",
 * "Claude Code · Work Max"; the default), by
 * project, or by account (the subscription a profile is signed in to,
 * AJ-E2109; a profile no account lists stands for itself), or by pool
 * (AJ-E1702: offered once you've made one; a session another app runs is
 * in no pool).
 *
 * The order is stable (⌘1–9 go to places, so a place never moves because
 * a row changed): the Apps with AI Juggler's own sessions first, then by
 * app and profile.
 * That is the default; in Your order the user's arrangement comes first
 * (`rail-order.ts`, D-47). In Latest activity (`foldGroup`), a quiet
 * place (nothing working) folds to one header ("Work · 4 idle"), and a
 * place shows its working rows and the 3 most recent others, then
 * "+n quiet".
 */
import { OWN_PLACE, SOME_APP, type RailItem, type RailPlace } from "./rail-items";

export type GroupBy = "place" | "project" | "account" | "pool";

export interface RailGroup {
  key: string;
  /** The header's words: the app, or the project or the account. */
  label: string;
  /** The place, when grouped by place. */
  place: RailPlace | null;
  /** Its account (grouped by account). */
  accountId: string | null;
  /** Its pool (grouped by pool): the header opens the pool's page. */
  poolId: string | null;
  items: RailItem[];
}

/** The last rows a place shows besides the working ones. */
export const RECENT_SHOWN = 3;

/** The account a profile is signed in to: its key and name. */
export type AccountOf = (profileId: string | null) => { key: string; label: string } | null;

/** The pool an item is in: its id and name (`null`: in none). */
export type PoolOf = (item: RailItem) => { id: string; label: string } | null;

/** The group of the items in no pool (sessions other apps run). */
export const NO_POOL = "pool:none";

function groupOf(item: RailItem, by: GroupBy, accountOf: AccountOf, poolOf: PoolOf): { key: string; label: string } {
  switch (by) {
    case "project":
      return { key: item.projectId ?? "none", label: item.project ?? "No project" };
    case "account":
      return accountOf(item.accountId) ?? { key: item.accountId ?? item.accountLabel, label: item.accountLabel };
    case "pool": {
      const pool = poolOf(item);
      return pool ? { key: `pool:${pool.id}`, label: pool.label } : { key: NO_POOL, label: "Not in a pool" };
    }
    case "place":
      return { key: item.place.key, label: item.place.app };
  }
}

/** Groups in a stable order: AI Juggler's place first, then by label and
 * the profile under it. */
export function groupItems(items: RailItem[], by: GroupBy, accountOf: AccountOf = () => null, poolOf: PoolOf = () => null): RailGroup[] {
  const groups = new Map<string, RailGroup>();
  for (const item of items) {
    const { key, label } = groupOf(item, by, accountOf, poolOf);
    let group = groups.get(key);
    if (!group) {
      const poolId = by === "pool" && key !== NO_POOL ? key.slice("pool:".length) : null;
      group = { key, label, place: by === "place" ? item.place : null, accountId: by === "account" ? item.accountId : null, poolId, items: [] };
      groups.set(key, group);
    }
    group.items.push(item);
  }
  // The Apps AI Juggler runs sessions in lead; the items in no pool come last.
  const rank = (group: RailGroup) => (group.key === NO_POOL ? 2 : group.place !== null && ownIn(group) ? 0 : 1);
  return [...groups.values()].sort(
    (a, b) =>
      rank(a) - rank(b) ||
      a.label.localeCompare(b.label) ||
      (a.place?.profile ?? "").localeCompare(b.place?.profile ?? "") ||
      a.key.localeCompare(b.key),
  );
}

/** 0.3's key for an item's place (the app, then its instance or the
 * profile; AI Juggler's own sessions one place), so an order or a fold
 * kept then finds the App it became (D-62). */
export function legacyPlaceKey(item: RailItem): string {
  if (item.session) return OWN_PLACE;
  const first = item.conversation?.places?.[0];
  if (first) return `${first.app || SOME_APP}|${first.instance ?? first.profile ?? ""}`;
  return `${SOME_APP}|${item.accountLabel}`;
}

/** Each old place key and the App its sessions are in now; where an old
 * place's sessions went to several Apps, the first one drawn. */
export function renamedPlaces(groups: readonly RailGroup[]): Map<string, string> {
  const renamed = new Map<string, string>();
  for (const group of groups) {
    for (const item of group.items) {
      const old = legacyPlaceKey(item);
      if (old !== group.key && !renamed.has(old)) renamed.set(old, group.key);
    }
  }
  return renamed;
}

/** It holds a session AI Juggler runs (its window is AI Juggler's own). */
export function ownIn(group: Pick<RailGroup, "items">): boolean {
  return group.items.some((item) => item.place.kind === "own");
}

/** A session the user pinned (AJ-E1502): it leads its place in every
 * order and never folds away. */
export function pinned(item: RailItem): boolean {
  return item.pinned;
}

/** Nothing in it is working, needs you or is pinned. */
export function isQuiet(group: RailGroup): boolean {
  return group.items.every((item) => item.state !== "working" && item.need === null && !pinned(item));
}

/** A folded place in words: "Work · 4 idle", "Work · 1 working · 2 idle".
 * What needs you is the header's amber chip, so it isn't said twice. */
export function quietWords(group: RailGroup): string {
  const name = group.place?.profile ?? group.label;
  const count = (test: (item: RailItem) => boolean) => group.items.filter(test).length;
  const working = count((item) => item.state === "working" && item.need === null);
  const review = count((item) => item.state === "done_review");
  const asks = count((item) => item.state === "asks_you" || (item.need !== null && item.state !== "done_review" && item.state !== "working"));
  const idle = group.items.length - working - review - asks;
  const parts = [working > 0 ? `${working} working` : null, idle > 0 ? `${idle} idle` : null, review > 0 ? `${review} to review` : null].filter(Boolean);
  return parts.length > 0 ? `${name} · ${parts.join(" · ")}` : name;
}

/** What a group shows: its pinned rows, its working rows, then the most
 * recent others (all of them when quiet ones are shown), and how many it
 * leaves out. */
export function foldGroup(group: RailGroup, showQuiet: boolean): { shown: RailItem[]; quiet: number } {
  const recent = (a: RailItem, b: RailItem) => b.lastMs - a.lastMs || a.key.localeCompare(b.key);
  const lead = group.items.filter(pinned).sort(recent);
  const working = group.items.filter((item) => !pinned(item) && item.state === "working").sort(recent);
  const others = group.items.filter((item) => !pinned(item) && item.state !== "working").sort(recent);
  if (showQuiet) return { shown: [...lead, ...working, ...others], quiet: 0 };
  return { shown: [...lead, ...working, ...others.slice(0, RECENT_SHOWN)], quiet: Math.max(0, others.length - RECENT_SHOWN) };
}

/**
 * Freeze re-sorting while the pointer or focus is in the list: rows the
 * list showed keep their places, and new ones join at the end, so a row
 * never jumps from under the pointer. `previous` is what was shown.
 */
export function frozenOrder<T>(next: T[], previous: string[], keyOf: (item: T) => string): T[] {
  const at = new Map(previous.map((key, index) => [key, index]));
  const kept = next.filter((item) => at.has(keyOf(item))).sort((a, b) => at.get(keyOf(a))! - at.get(keyOf(b))!);
  const added = next.filter((item) => !at.has(keyOf(item)));
  return [...kept, ...added];
}
