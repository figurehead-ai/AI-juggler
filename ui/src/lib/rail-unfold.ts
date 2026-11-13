/**
 * 0.4.3 — what a running App shows under it in Open now, unfolded (the
 * maintainer's 0.4.2 test: three running profiles read "nothing open"
 * with a chevron that unfolded nothing, their sessions only in a menu).
 *
 * Under its open rows: its pinned ended ones, then its last ones (D-64):
 * at once when nothing is open in it (they're all it has), under a muted
 * "Recent" (D-103.1: "nothing open" above three rows read as a
 * contradiction), else only once its header's "+N recent" is clicked (no
 * row of its own: the maintainer, 0.4.1, on a "Last N inactive" row under
 * every App); then "Show all" (its page) when it has more. While
 * searching, every match. Its header has a fold only when this, or an
 * open row, gives it a row to show (`unfoldsToARow`).
 *
 * Pure, so the order and the invariants are unit-tested.
 */
import { nestRows } from "./rail-tree";
import { RECENT_INLINE, type IdleApp } from "./rail-apps";
import type { RailItem } from "./rail-items";
import type { RailGroup } from "./rail-places";
import type { Line } from "./rail-lines";

/** The lines under Open now group `group`: `sessions` are its own that
 * aren't open, newest first (`idleSessionsByApp`). */
export function unfoldLines(group: RailGroup, sessions: readonly RailItem[], searching: boolean, toggled: ReadonlySet<string>): Line[] {
  const key = group.key;
  const out: Line[] = [];
  const pastRows = (rows: readonly RailItem[]) => {
    for (const { item, depth, parent } of nestRows(rows)) out.push({ kind: "past", item, depth, parent: parent?.title ?? null, app: key });
  };
  if (searching) {
    pastRows(sessions);
    return out;
  }
  const rest = sessions.filter((item) => !item.pinned);
  pastRows(sessions.filter((item) => item.pinned));
  if (rest.length === 0) return out;
  if (group.items.length > 0 && !toggled.has(`${key}#recent`)) return out;
  if (group.items.length === 0) out.push({ kind: "recent", app: key });
  pastRows(rest.slice(0, RECENT_INLINE));
  if (rest.length > RECENT_INLINE) out.push({ kind: "all", app: key, count: sessions.length, group, idle: null });
  return out;
}

/** 0.4.4 (ADR-0032 amendment 2): by Account, its ended pinned sessions
 * under it, as by App: after its open rows in Open now. */
export function pinnedLines(key: string, pinned: readonly RailItem[]): Line[] {
  return nestRows(pinned).map(({ item, depth, parent }) => ({ kind: "past", item, depth, parent: parent?.title ?? null, app: key }));
}

/** 0.4.4: by Account, the Accounts with nothing open but an ended pinned
 * session (pins never mean open, so never in Open now): "Pinned", each
 * one's header ("nothing open"), then its pinned rows, as an App that
 * isn't running keeps its pinned ones under it in Inactive apps. `rest`'s
 * groups hold those pinned sessions; folded (`folded`), its header only. */
export function pinnedSection(rest: readonly RailGroup[], folded: (key: string) => boolean): Line[] {
  if (rest.length === 0) return [];
  const out: Line[] = [{ kind: "section", id: "pinned", count: rest.reduce((sum, group) => sum + group.items.length, 0) }];
  for (const group of rest) {
    const shut = folded(group.key);
    out.push({ kind: "header", group: { ...group, items: [] }, folded: shut ? `${group.label} · ${group.items.length} pinned` : null, needs: 0, digit: null, foldable: true, recent: null });
    if (!shut) out.push(...pinnedLines(group.key, group.items));
  }
  return out;
}

/** Unfolding the group shows a row: an open one, or one of `under`. */
export function unfoldsToARow(group: RailGroup, under: readonly Line[]): boolean {
  return group.items.length > 0 || under.some((line) => line.kind === "past");
}

/** An Inactive apps row's lines under it: its last ones, unfolded by a
 * click, under "Recent" as Open now's (D-103.1: nothing is open in it);
 * searching, every one that matches (rc4 walk), each under the one it was
 * started from; its pinned ones always (ADR-0032: never open). 0.4.3:
 * then "Show all", as Open now's. */
export function inactiveLines(app: IdleApp, searching: boolean, toggled: ReadonlySet<string>): { open: boolean; lines: Line[] } {
  const open = searching || toggled.has(`${app.key}#recent`);
  const past = (rows: readonly RailItem[]): Line[] => nestRows(rows).map(({ item, depth, parent }) => ({ kind: "past", item, depth, parent: parent?.title ?? null, app: app.key }));
  if (searching) return { open, lines: past(app.sessions) };
  const pinned = app.sessions.filter((item) => item.pinned);
  const rest = app.sessions.filter((item) => !item.pinned);
  const lines = past(pinned);
  if (!open || rest.length === 0) return { open, lines };
  lines.push({ kind: "recent", app: app.key }, ...past(rest.slice(0, RECENT_INLINE)));
  if (rest.length > RECENT_INLINE) lines.push({ kind: "all", app: app.key, count: app.sessions.length, group: null, idle: app });
  return { open, lines };
}
