/**
 * AJ-E2104 — the place lens as lines: what the rail draws, top to bottom,
 * each with its height, so a long list renders only what is in view
 * (past about 30 rows) and keys move through it.
 *
 * Needs you (never grouped: a shortcut list), then Open now by place (or
 * account), then the "Also:" line, then the views turned on (ended today,
 * inactive). A session that needs you is in both: in its place, where it
 * stays (D-47), and in Needs you; each list counts it once.
 * Nothing hidden is silent: each thing left out leaves its count — a
 * folded place its header, a long place's rest "+n more" (Latest
 * activity: its quiet rows, "+n quiet"), the rest the "Also:" line.
 *
 * Open now's order is the one picked (`rail-order.ts`): Your order by
 * default, where nothing moves by itself.
 */
import { endedToday, needsYou, openNow, type RailItem } from "./rail-items";
import { frozenOrder, groupItems, isQuiet, NO_POOL, quietWords, type AccountOf, type GroupBy, type PoolOf, type RailGroup } from "./rail-places";
import { arrangeGroup, arrangeGroups, foldKey, lensOf, type OrderMode } from "./rail-order";
import { isSearching, matches, type RailQuery, type RailView } from "./rail-view";
import type { NoticeInfo } from "./ipc/notices";

export type Line =
  | { kind: "section"; id: "needs" | "open"; count: number }
  | { kind: "need"; item: RailItem }
  | { kind: "notice"; notice: NoticeInfo }
  | { kind: "header"; group: RailGroup; folded: string | null; needs: number; digit: number | null }
  | { kind: "row"; item: RailItem; group: RailGroup }
  | { kind: "more"; group: RailGroup; quiet: number; words: "quiet" | "more" }
  | { kind: "also" }
  | { kind: "label"; id: "ended" | "inactive"; count: number }
  | { kind: "plain"; item: RailItem };

export const LINE_HEIGHT: Record<Line["kind"], number> = {
  section: 30,
  need: 64,
  notice: 64,
  header: 32,
  row: 46,
  more: 24,
  also: 36,
  label: 26,
  plain: 42,
};

/** Compact (AJ-E403, §2.2: rail rows 32 px): a row is one line, a
 * Needs you row two; heads and lines stay as they are. */
export const LINE_HEIGHT_COMPACT: Record<Line["kind"], number> = { ...LINE_HEIGHT, need: 48, row: 32, plain: 32 };

/** Each line's height at a density. */
export type LineHeights = Record<Line["kind"], number>;
export function lineHeights(compact: boolean): LineHeights {
  return compact ? LINE_HEIGHT_COMPACT : LINE_HEIGHT;
}

/** Rows past this many render only what's in view. */
export const VIRTUALISE_AT = 30;

export interface AlsoCounts {
  ended: number;
  inactive: number;
  background: number;
}

export interface Lines {
  lines: Line[];
  also: AlsoCounts;
  /** Open now's groups, in order (⌘1–9 go to the first nine). */
  groups: RailGroup[];
  /** Each group's rows in the order, the ones left out too (moves count
   * positions in it). */
  arranged: Map<string, RailItem[]>;
  needs: number;
  /** Rows drawn (needs, open, ended, inactive). */
  rows: number;
}

export interface LineInputs {
  items: RailItem[];
  view: RailView;
  query: RailQuery;
  now: number;
  /** Places the user unfolded (or folded) this visit. */
  toggled: ReadonlySet<string>;
  /** What each list showed before, while the pointer or focus is in the
   * list (re-sorting frozen); `null` when free. */
  frozen: Map<string, string[]> | null;
  /** The Account lens's subscriptions, by profile. */
  accountOf?: AccountOf;
  /** The Pool lens's pools (AJ-E1702), once you've made one. */
  poolOf?: PoolOf;
  /** Open now's order (D-47): Your order when not given. */
  order?: OrderInputs;
  /** D-49: the rest of the backend's Needs you (not a session's), so the
   * section holds what the bell counts; left out while searching. */
  others?: NoticeInfo[];
}

export interface OrderInputs {
  mode: OrderMode;
  /** Each row's place in the user's order. */
  rows: ReadonlyMap<string, number>;
  /** The lens's groups in the user's order. */
  groups: readonly string[];
  /** The folds the user made (`<lens>:<group>`). */
  folded: ReadonlySet<string>;
}

const YOURS: OrderInputs = { mode: "yours", rows: new Map(), groups: [], folded: new Set() };

export function buildLines({ items, view, query, now, toggled, frozen, accountOf, poolOf, order = YOURS, others = [] }: LineInputs): Lines {
  const searching = isSearching(query);
  const shown = items.filter((item) => matches(item, view.only, query));
  const want = (key: "ended" | "inactive" | "background" | "quiet" | "archived") => view.show[key] || query.is.has(key);
  const keep = (list: string, rows: RailItem[]) => (frozen?.has(list) ? frozenOrder(rows, frozen.get(list)!, (item) => item.key) : rows);

  const needs = keep("needs", needsYou(shown));
  const open = shown.filter((item) => openNow(item) || (want("background") && item.background && item.need === null));
  const ended = shown.filter((item) => endedToday(item, now, want("archived"))).sort((a, b) => b.lastMs - a.lastMs);
  const inactive = shown.filter((item) => item.inactive && item.need === null && !item.pinned).sort((a, b) => b.lastMs - a.lastMs);
  const background = shown.filter((item) => item.background && item.need === null);
  // The Project lens is the project board's; the Pool lens waits for a pool.
  const by: GroupBy = view.group === "account" ? "account" : view.group === "pool" && poolOf ? "pool" : "place";
  const groups = arrangeGroups(groupItems(open, by, accountOf, poolOf), order.mode, order.groups);
  const stable = order.mode !== "latest";
  const arranged = new Map<string, RailItem[]>();
  const groupKey = (item: RailItem): string => {
    if (by === "place") return item.place.key;
    if (by === "pool") {
      const pool = poolOf?.(item);
      return pool ? `pool:${pool.id}` : NO_POOL;
    }
    return accountOf?.(item.accountId)?.key ?? item.accountId ?? item.accountLabel;
  };
  const lines: Line[] = [];
  let rows = 0;
  const rest = searching ? [] : others;
  if (needs.length + rest.length > 0) {
    lines.push({ kind: "section", id: "needs", count: needs.length + rest.length });
    for (const item of needs) lines.push({ kind: "need", item });
    for (const notice of rest) lines.push({ kind: "notice", notice });
    rows += needs.length + rest.length;
  }
  if (groups.length > 0) lines.push({ kind: "section", id: "open", count: open.length });
  groups.forEach((group, index) => {
    const needsHere = needs.filter((item) => groupKey(item) === group.key).length;
    const digit = index < 9 ? index + 1 : null;
    // Your order and Name: a place folds only when the user folds it
    // (remembered). Latest activity: a quiet place folds to its header,
    // unless the user opened it, quiet ones are shown, or a search is on.
    const folded = stable
      ? order.folded.has(foldKey(lensOf(by), group.key)) && !searching
      : isQuiet(group) && !want("quiet") && !searching ? !toggled.has(group.key) : toggled.has(group.key);
    const fold = arrangeGroup(group, order.mode, order.rows, want("quiet") || searching || toggled.has(`${group.key}#all`));
    arranged.set(group.key, fold.all);
    if (folded) {
      lines.push({ kind: "header", group, folded: quietWords(group), needs: needsHere, digit });
      return;
    }
    lines.push({ kind: "header", group, folded: null, needs: needsHere, digit });
    // Latest activity re-sorts, so it holds still under the pointer; the
    // other orders never move a row by themselves.
    const drawn = stable ? fold.shown : keep(`group:${group.key}`, fold.shown);
    for (const item of drawn) lines.push({ kind: "row", item, group });
    rows += drawn.length;
    if (fold.hidden > 0) lines.push({ kind: "more", group, quiet: fold.hidden, words: fold.words });
  });
  lines.push({ kind: "also" });
  if (want("ended") && ended.length > 0) {
    lines.push({ kind: "label", id: "ended", count: ended.length });
    for (const item of ended) lines.push({ kind: "plain", item });
    rows += ended.length;
  }
  if (want("inactive") && inactive.length > 0) {
    lines.push({ kind: "label", id: "inactive", count: inactive.length });
    for (const item of inactive) lines.push({ kind: "plain", item });
    rows += inactive.length;
  }
  return {
    lines,
    also: { ended: ended.length, inactive: inactive.length, background: background.length },
    groups,
    arranged,
    needs: needs.length,
    rows,
  };
}

/** The lines with Needs you or Open now folded: a folded section keeps
 * its head (with its count) and drops its rows and places. */
export function foldSections(lines: Line[], folded: ReadonlySet<"needs" | "open">): Line[] {
  if (folded.size === 0) return lines;
  let hiding = false;
  return lines.filter((line) => {
    if (line.kind === "section") {
      hiding = folded.has(line.id);
      return true;
    }
    if (line.kind === "also") hiding = false;
    return !hiding;
  });
}

/** Where a line starts, from the top of the list. */
export function offsetOf(lines: Line[], index: number, heights: LineHeights = LINE_HEIGHT): number {
  return lines.slice(0, index).reduce((sum, line) => sum + heights[line.kind], 0);
}

/** What each list shows now, to freeze it. */
export function shownOrder(lines: Line[]): Map<string, string[]> {
  const out = new Map<string, string[]>();
  const add = (list: string, key: string) => out.set(list, [...(out.get(list) ?? []), key]);
  for (const line of lines) {
    if (line.kind === "need") add("needs", line.item.key);
    else if (line.kind === "row") add(`group:${line.group.key}`, line.item.key);
  }
  return out;
}

/** The lines in view: `[start, end)`, and the space above and below. */
export function windowOf(lines: Line[], top: number, height: number, overscan = 240, heights: LineHeights = LINE_HEIGHT): { start: number; end: number; before: number; after: number } {
  let y = 0;
  let start = 0;
  while (start < lines.length && y + heights[lines[start]!.kind] < top - overscan) {
    y += heights[lines[start]!.kind];
    start += 1;
  }
  const before = y;
  let end = start;
  while (end < lines.length && y < top + height + overscan) {
    y += heights[lines[end]!.kind];
    end += 1;
  }
  let after = 0;
  for (let i = end; i < lines.length; i += 1) after += heights[lines[i]!.kind];
  return { start, end, before, after };
}
