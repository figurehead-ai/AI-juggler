/**
 * AJ-E2404 — a tree table's two mechanics, pure: which rows to put in the
 * DOM (a window over fixed-height rows, so a list of 2,000 keeps at most
 * 60 in the page), and where the keyboard goes (WAI-ARIA treegrid: ↑ ↓
 * move, → opens or goes to the first child, ← closes or goes to the
 * parent, Home and End).
 */

export interface TreeRowLike {
  key: string;
  depth: number;
  parent_key: string | null;
  has_children: boolean;
}

/** The rows `[start, end)` to render for a scroll position. */
export function windowRows(total: number, rowHeight: number, scrollTop: number, viewport: number, overscan = 8, most = 60): [number, number] {
  if (total <= 0) return [0, 0];
  const first = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const visible = Math.ceil(viewport / rowHeight) + overscan * 2;
  const end = Math.min(total, first + Math.min(visible, most));
  return [first, end];
}

/** Where a key press moves the focus, from row `at`: the new index, and
 * whether the row at `at` should open or close instead. */
export function treeKey(
  rows: readonly TreeRowLike[],
  at: number,
  key: string,
  open: (row: TreeRowLike) => boolean,
): { to: number } | { toggle: "open" | "close"; at: number } | null {
  const row = rows[at];
  if (!row) return null;
  switch (key) {
    case "ArrowDown":
      return { to: Math.min(rows.length - 1, at + 1) };
    case "ArrowUp":
      return { to: Math.max(0, at - 1) };
    case "Home":
      return { to: 0 };
    case "End":
      return { to: rows.length - 1 };
    case "ArrowRight":
      if (row.has_children && !open(row)) return { toggle: "open", at };
      if (row.has_children && rows[at + 1]?.parent_key === row.key) return { to: at + 1 };
      return { to: at };
    case "ArrowLeft": {
      if (row.has_children && open(row)) return { toggle: "close", at };
      const parent = rows.findIndex((other) => other.key === row.parent_key);
      if (parent >= 0) return { to: parent };
      // A subprocess hangs under its session; its process parent is the
      // nearest row above at a lesser depth.
      for (let up = at - 1; up >= 0; up--) if (rows[up]!.depth < row.depth) return { to: up };
      return { to: at };
    }
    default:
      return null;
  }
}
