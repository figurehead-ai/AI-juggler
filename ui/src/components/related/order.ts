/**
 * E31 (AJ-E3103, D-62 decision 21) — the order every related list opens
 * on, and what its filters narrow by, as the UI draws them:
 *
 * 1. Needs you, 2. Working, 3. Open, 4. Active in the last 7 days,
 * 5. Earlier — each by last activity, newest first.
 *
 * The backend orders a live list the same way (`sessions::rows::Rank`);
 * this file is the design preview's (no backend) and the bands' words.
 * Pure, so the order and the filters are unit-tested against the same
 * expectations the backend's tests use.
 */
import type { RailItem } from "../../lib/rail-items";

export type Band = "needs" | "working" | "open" | "recent" | "earlier";

/** The order's sections, in order. */
export const BANDS: readonly Band[] = ["needs", "working", "open", "recent", "earlier"];

export const BAND_WORDS: Record<Band, string> = {
  needs: "Needs you",
  working: "Working",
  open: "Open",
  recent: "Last 7 days",
  earlier: "Earlier",
};

/** "Active in the last 7 days". */
export const RECENT_MS = 7 * 24 * 3_600_000;

/** Where a row stands: what needs you (it asks, is stuck or hit a limit),
 * what's working, what's open (idle, or done to review), then the ended
 * ones by when they were last active. As the backend ranks it. */
export function bandOf(item: Pick<RailItem, "state" | "live" | "lastMs">, now: number): Band {
  if (item.state === "asks_you") return "needs";
  if (item.state === "working") return "working";
  if (item.state === "idle" || item.state === "done_review") return "open";
  return now - item.lastMs <= RECENT_MS ? "recent" : "earlier";
}

/** The list's order: by band, then newest first, then by key. */
export function ordered<T>(rows: readonly T[], itemOf: (row: T) => RailItem, keyOf: (row: T) => string, now: number): T[] {
  const rank = (row: T) => BANDS.indexOf(bandOf(itemOf(row), now));
  return [...rows].sort((a, b) => {
    const ra = rank(a);
    const rb = rank(b);
    // The last two bands are one in the backend's order (ended, newest
    // first), so they never cross.
    const ba = Math.min(ra, 3);
    const bb = Math.min(rb, 3);
    return ba - bb || itemOf(b).lastMs - itemOf(a).lastMs || keyOf(a).localeCompare(keyOf(b));
  });
}

/** A list's state filter: everything, what's open now, what was active in
 * the last 7 days but has ended, or everything that has ended. */
export type StateFilter = "all" | "open" | "recent" | "ended";

export const STATE_WORDS: Record<StateFilter, string> = {
  all: "All",
  open: "Open now",
  recent: "Recent",
  ended: "Ended",
};

/** What a state filter asks the backend (`which`, `since_ms`). */
export function stateQuery(state: StateFilter, now: number): { which: string | null; since_ms: number | null } {
  switch (state) {
    case "open":
      return { which: "open", since_ms: null };
    case "recent":
      return { which: "ended", since_ms: now - RECENT_MS };
    case "ended":
      return { which: "ended", since_ms: null };
    default:
      return { which: null, since_ms: null };
  }
}

/** The filters a list can offer, by what its rows belong to. */
export type FacetKind = "project" | "app" | "account" | "machine";

export interface Filters {
  state: StateFilter;
  /** One value per kind; `""` a project filter's "No project". */
  project: string | null;
  app: string | null;
  account: string | null;
  machine: string | null;
}

export const NO_FILTERS: Filters = { state: "all", project: null, app: null, account: null, machine: null };

/** How many filters are set (the state counts unless it's All). */
export function filtersOn(filters: Filters): number {
  return (filters.state === "all" ? 0 : 1) + [filters.project, filters.app, filters.account, filters.machine].filter((value) => value !== null).length;
}

/** What a row belongs to, for the design preview's filters. */
export interface Belongs {
  project: string;
  app: string;
  account: string | null;
  machine: string;
}

/** Does a row pass the filters (the preview's; the backend narrows a live
 * list)? */
export function passes(item: Pick<RailItem, "state" | "live" | "lastMs">, of: Belongs, filters: Filters, now: number): boolean {
  const ended = item.state === "ended";
  const state =
    filters.state === "all" ||
    (filters.state === "open" && !ended) ||
    (filters.state === "ended" && ended) ||
    (filters.state === "recent" && ended && now - item.lastMs <= RECENT_MS);
  return (
    state &&
    (filters.project === null || filters.project === of.project) &&
    (filters.app === null || filters.app === of.app) &&
    (filters.account === null || filters.account === of.account) &&
    (filters.machine === null || filters.machine === of.machine)
  );
}

/** One value a filter offers. */
export interface Facet {
  id: string;
  count: number;
}

/** Each kind's values over `rows`, the most rows first, then by id. */
export function facetsOf(rows: readonly Belongs[]): Record<FacetKind, Facet[]> {
  const tally = (pick: (of: Belongs) => string | null) => {
    const counts = new Map<string, number>();
    for (const of of rows) {
      const id = pick(of);
      if (id !== null) counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    return [...counts].map(([id, count]) => ({ id, count })).sort((a, b) => b.count - a.count || a.id.localeCompare(b.id));
  };
  return { project: tally((of) => of.project), app: tally((of) => of.app), account: tally((of) => of.account), machine: tally((of) => of.machine) };
}
