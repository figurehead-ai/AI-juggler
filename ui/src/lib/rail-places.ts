/**
 * AJ-E2104 — Open now's groups. By place (app + profile, the default), by
 * project, or by account (the subscription a profile is signed in to,
 * AJ-E2109; a profile no account lists stands for itself); pools once
 * they exist (a hook: none on 0.1.x).
 *
 * The order is stable (⌘1–9 go to places, so a place never moves because
 * a row changed): AI Juggler's own place first, then by app and profile.
 * A quiet place (nothing working) folds to one header ("Work · 4 idle");
 * a place shows its working rows and the 3 most recent others, then
 * "+n quiet".
 */
import { OWN_PLACE, type RailItem, type RailPlace } from "./rail-items";

export type GroupBy = "place" | "project" | "account" | "pool";

export interface RailGroup {
  key: string;
  /** The header's words: the app, or the project or the account. */
  label: string;
  /** The place, when grouped by place. */
  place: RailPlace | null;
  /** Its account (grouped by account). */
  accountId: string | null;
  items: RailItem[];
}

/** The last rows a place shows besides the working ones. */
export const RECENT_SHOWN = 3;

/** The account a profile is signed in to: its key and name. */
export type AccountOf = (profileId: string | null) => { key: string; label: string } | null;

function groupOf(item: RailItem, by: GroupBy, accountOf: AccountOf): { key: string; label: string } {
  switch (by) {
    case "project":
      return { key: item.projectId ?? "none", label: item.project ?? "No project" };
    case "account":
      return accountOf(item.accountId) ?? { key: item.accountId ?? item.accountLabel, label: item.accountLabel };
    case "pool": // Pools don't exist on 0.1.x; the lens waits for them.
    case "place":
      return { key: item.place.key, label: item.place.app };
  }
}

/** Groups in a stable order: AI Juggler's place first, then by label and
 * the profile under it. */
export function groupItems(items: RailItem[], by: GroupBy, accountOf: AccountOf = () => null): RailGroup[] {
  const groups = new Map<string, RailGroup>();
  for (const item of items) {
    const { key, label } = groupOf(item, by, accountOf);
    let group = groups.get(key);
    if (!group) {
      group = { key, label, place: by === "place" || by === "pool" ? item.place : null, accountId: by === "account" ? item.accountId : null, items: [] };
      groups.set(key, group);
    }
    group.items.push(item);
  }
  const rank = (group: RailGroup) => (group.key === OWN_PLACE ? 0 : 1);
  return [...groups.values()].sort(
    (a, b) =>
      rank(a) - rank(b) ||
      a.label.localeCompare(b.label) ||
      (a.place?.profile ?? "").localeCompare(b.place?.profile ?? "") ||
      a.key.localeCompare(b.key),
  );
}

/** Nothing in it is working. */
export function isQuiet(group: RailGroup): boolean {
  return group.items.every((item) => item.state !== "working");
}

/** A folded quiet place in words: "Work · 4 idle". */
export function quietWords(group: RailGroup): string {
  const name = group.place?.profile ?? group.label;
  const review = group.items.filter((item) => item.state === "done_review").length;
  const idle = group.items.length - review;
  const parts = [idle > 0 ? `${idle} idle` : null, review > 0 ? `${review} to review` : null].filter(Boolean);
  return `${name} · ${parts.join(" · ")}`;
}

/** What a group shows: its working rows, then the most recent others
 * (all of them when quiet ones are shown), and how many it leaves out. */
export function foldGroup(group: RailGroup, showQuiet: boolean): { shown: RailItem[]; quiet: number } {
  const recent = (a: RailItem, b: RailItem) => b.lastMs - a.lastMs || a.key.localeCompare(b.key);
  const working = group.items.filter((item) => item.state === "working").sort(recent);
  const others = group.items.filter((item) => item.state !== "working").sort(recent);
  if (showQuiet) return { shown: [...working, ...others], quiet: 0 };
  return { shown: [...working, ...others.slice(0, RECENT_SHOWN)], quiet: Math.max(0, others.length - RECENT_SHOWN) };
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
