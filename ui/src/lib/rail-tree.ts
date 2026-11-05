/**
 * 0.3.9 (the maintainer's walk of 2026-11-04): a session started from
 * another (a Claude app side session: "AWS WS-A" from "AWS migration
 * planning") sits under the one that started it, as the app's own sidebar
 * draws it; Open now showed them side by side.
 *
 * Within a group, rows started from another follow it depth-first, each
 * level in the order it was given (Your order, Name or latest activity
 * among siblings). A row whose parent isn't among the rows (not open, in
 * another group, folded away) keeps its own place, at the top level.
 */
import type { RailItem } from "./rail-items";

/** The deepest a row is drawn in (deeper ones line up with it). */
export const MAX_DEPTH = 3;

export interface NestedRow {
  item: RailItem;
  depth: number;
  /** The row it was started from, when that one is drawn here. */
  parent: RailItem | null;
}

export function nestRows(rows: readonly RailItem[]): NestedRow[] {
  const keys = new Set(rows.map((row) => row.key));
  const nested = (row: RailItem) => row.parentKey !== null && row.parentKey !== row.key && keys.has(row.parentKey);
  const children = new Map<string, RailItem[]>();
  for (const row of rows) {
    if (!nested(row)) continue;
    const list = children.get(row.parentKey!) ?? [];
    list.push(row);
    children.set(row.parentKey!, list);
  }
  const out: NestedRow[] = [];
  const seen = new Set<string>();
  const visit = (item: RailItem, depth: number, parent: RailItem | null) => {
    if (seen.has(item.key)) return;
    seen.add(item.key);
    out.push({ item, depth: Math.min(depth, MAX_DEPTH), parent });
    for (const child of children.get(item.key) ?? []) visit(child, depth + 1, item);
  };
  for (const row of rows) if (!nested(row)) visit(row, 0, null);
  // Parents that name each other in a ring: each keeps its place.
  for (const row of rows) visit(row, 0, null);
  return out;
}
