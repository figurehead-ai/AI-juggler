/**
 * AJ-E2104 — the place lens as lines: what the rail draws, top to bottom,
 * each with its height, so a long list renders only what is in view
 * (past about 30 rows) and keys move through it.
 *
 * Needs you (never grouped), then Open now by place (or account), then
 * the "Also:" line, then the views turned on (ended today, inactive).
 * Nothing hidden is silent: each thing left out leaves its count — a
 * folded place its header, a place's quiet rows "+n quiet", the rest the
 * "Also:" line.
 */
import { endedToday, needsYou, openNow, type RailItem } from "./rail-items";
import { foldGroup, frozenOrder, groupItems, isQuiet, quietWords, type AccountOf, type GroupBy, type RailGroup } from "./rail-places";
import { isSearching, matches, type RailQuery, type RailView } from "./rail-view";

export type Line =
  | { kind: "section"; id: "needs" | "open"; count: number }
  | { kind: "need"; item: RailItem }
  | { kind: "header"; group: RailGroup; folded: string | null; needs: number; digit: number | null }
  | { kind: "row"; item: RailItem; group: RailGroup }
  | { kind: "more"; group: RailGroup; quiet: number }
  | { kind: "also" }
  | { kind: "label"; id: "ended" | "inactive"; count: number }
  | { kind: "plain"; item: RailItem };

export const LINE_HEIGHT: Record<Line["kind"], number> = {
  section: 28,
  need: 46,
  header: 32,
  row: 42,
  more: 24,
  also: 30,
  label: 26,
  plain: 42,
};

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
}

export function buildLines({ items, view, query, now, toggled, frozen, accountOf }: LineInputs): Lines {
  const searching = isSearching(query);
  const shown = items.filter((item) => matches(item, view.only, query));
  const want = (key: "ended" | "inactive" | "background" | "quiet" | "archived") => view.show[key] || query.is.has(key);
  const keep = (list: string, rows: RailItem[]) => (frozen?.has(list) ? frozenOrder(rows, frozen.get(list)!, (item) => item.key) : rows);

  const needs = keep("needs", needsYou(shown));
  const open = shown.filter((item) => openNow(item) || (want("background") && item.background && item.need === null));
  const ended = shown.filter((item) => endedToday(item, now, want("archived"))).sort((a, b) => b.lastMs - a.lastMs);
  const inactive = shown.filter((item) => item.inactive && item.need === null).sort((a, b) => b.lastMs - a.lastMs);
  const background = shown.filter((item) => item.background && item.need === null);
  const by: GroupBy = view.group === "account" ? "account" : "place";
  const groups = groupItems(open, by, accountOf);
  const lines: Line[] = [];
  let rows = 0;
  if (needs.length > 0) {
    lines.push({ kind: "section", id: "needs", count: needs.length });
    for (const item of needs) lines.push({ kind: "need", item });
    rows += needs.length;
  }
  if (groups.length > 0) lines.push({ kind: "section", id: "open", count: open.length });
  groups.forEach((group, index) => {
    const needsHere = needs.filter((item) => (by === "place" ? item.place.key : (accountOf?.(item.accountId)?.key ?? item.accountId ?? item.accountLabel)) === group.key).length;
    const digit = index < 9 ? index + 1 : null;
    // A quiet place folds to its header, unless the user opened it, quiet
    // ones are shown, or a search is on.
    const quiet = isQuiet(group) && !want("quiet") && !searching;
    const folded = quiet !== toggled.has(group.key);
    if (folded) {
      lines.push({ kind: "header", group, folded: quietWords(group), needs: needsHere, digit });
      return;
    }
    lines.push({ kind: "header", group, folded: null, needs: needsHere, digit });
    const fold = foldGroup(group, want("quiet") || searching || toggled.has(`${group.key}#all`));
    for (const item of keep(`group:${group.key}`, fold.shown)) lines.push({ kind: "row", item, group });
    rows += fold.shown.length;
    if (fold.quiet > 0) lines.push({ kind: "more", group, quiet: fold.quiet });
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
export function offsetOf(lines: Line[], index: number): number {
  return lines.slice(0, index).reduce((sum, line) => sum + LINE_HEIGHT[line.kind], 0);
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
export function windowOf(lines: Line[], top: number, height: number, overscan = 240): { start: number; end: number; before: number; after: number } {
  let y = 0;
  let start = 0;
  while (start < lines.length && y + LINE_HEIGHT[lines[start]!.kind] < top - overscan) {
    y += LINE_HEIGHT[lines[start]!.kind];
    start += 1;
  }
  const before = y;
  let end = start;
  while (end < lines.length && y < top + height + overscan) {
    y += LINE_HEIGHT[lines[end]!.kind];
    end += 1;
  }
  let after = 0;
  for (let i = end; i < lines.length; i += 1) after += LINE_HEIGHT[lines[i]!.kind];
  return { start, end, before, after };
}
