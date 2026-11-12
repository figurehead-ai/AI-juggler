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
 *
 * D-64, ADR-0032 (D-100): by App, Open now holds the running Apps (each
 * one's past sessions behind "Last 3 inactive"), "Running, not added" the
 * apps found running (Add), "Inactive apps" the rest; each App in one.
 */
import { cliPlace, endedToday, needsYou, openNow, OWN_APP, type RailItem } from "./rail-items";
import { frozenOrder, groupItems, isQuiet, NO_POOL, quietWords, type AccountOf, type GroupBy, type PoolOf, type RailGroup } from "./rail-places";
import { arrangeGroup, arrangeGroups, foldKey, lensOf, type OrderMode } from "./rail-order";
import { isSearching, matches, type RailQuery, type RailView } from "./rail-view";
import type { NoticeInfo } from "./ipc/notices";
import type { KnownPlaceInfo } from "./ipc/places";
import { nestRows } from "./rail-tree";
import { appByConversation, appByFolder, appKeyOf, IDLE_APPS_SHOWN, idleApps, idleSessionsByApp, knownAppWord, knownKey, offeredLines, RECENT_INLINE, runningGroups, type IdleApp, type OfferLine } from "./rail-apps";
import { appName, desktopProfile } from "./app-name";

export type SectionId = "needs" | "open" | "offered" | "idle";

export type Line =
  | { kind: "section"; id: SectionId; count: number }
  | { kind: "need"; item: RailItem }
  | { kind: "notice"; notice: NoticeInfo }
  | { kind: "header"; group: RailGroup; folded: string | null; needs: number; digit: number | null }
  | { kind: "row"; item: RailItem; group: RailGroup; depth?: number; parent?: string | null }
  | { kind: "more"; group: RailGroup; quiet: number; words: "quiet" | "more" }
  | { kind: "also" }
  | { kind: "label"; id: "ended" | "inactive"; count: number }
  | { kind: "plain"; item: RailItem }
  /** D-64: an app's "Last 3 inactive" (`app`: its key). */
  | { kind: "recent"; app: string; count: number; open: boolean }
  /** D-64: one of an app's last inactive sessions, under it. */
  | { kind: "past"; item: RailItem; app: string; depth?: number; parent?: string | null }
  /** D-64: an app with nothing open. */
  | { kind: "idle"; app: IdleApp; open: boolean }
  /** D-64: "Show n more" inactive apps. */
  | { kind: "idle-more"; hidden: number }
  /** D-100: an app running that the user hasn't added (Add). */
  | Extract<OfferLine, { kind: "offer" }>;

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
  recent: 26,
  past: 42,
  idle: 46,
  "idle-more": 26,
  offer: 34,
};

/** Compact (AJ-E403, §2.2: rail rows 32 px): a row is one line, a
 * Needs you row two; heads and lines stay as they are. */
export const LINE_HEIGHT_COMPACT: Record<Line["kind"], number> = { ...LINE_HEIGHT, need: 48, row: 32, plain: 32, past: 32, idle: 32 };

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
  /** Rows drawn (needs, open, ended, inactive, an app's last ones, the
   * inactive apps). */
  rows: number;
  /** AJ-E3204: the Apps kept off the rail (by App), with what's open in
   * each, for the Also line. */
  offRail: OffRail[];
}

/** An App kept off the rail: its key, its short name (the profile, else
 * the app) and full name, its sessions open. */
export interface OffRail {
  key: string;
  short: string;
  name: string;
  open: number;
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
  /** D-64: the desktop apps remembered (the App grouping's inactive apps,
   * and where a desktop app's past sessions were last open). */
  known?: readonly KnownPlaceInfo[];
  /** AJ-E3204: an App (by its rail key) kept off the rail by its own
   * Settings. By App only; its sessions that need you still show. */
  offRail?: (key: string) => boolean;
  /** D-100: the apps found running, not added (`places_offers`). */
  offers?: Parameters<typeof offeredLines>[0];
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

export function buildLines({ items, view, query, now, toggled, frozen, accountOf, poolOf, order = YOURS, others = [], known = [], offRail = () => false, offers = [] }: LineInputs): Lines {
  const searching = isSearching(query);
  const filtering = searching || Object.keys(view.only).length > 0;
  const shown = items.filter((item) => matches(item, view.only, query, accountOf, poolOf));
  const want = (key: "ended" | "inactive" | "background" | "quiet" | "archived") => view.show[key] || query.is.has(key);
  const keep = (list: string, rows: RailItem[]) => (frozen?.has(list) ? frozenOrder(rows, frozen.get(list)!, (item) => item.key) : rows);

  const needs = keep("needs", needsYou(shown));
  // 0.4.1: an open session on a folder only one desktop profile runs on
  // is that profile's (its Start, Restart, Keep alive), not its CLI App's.
  const byFolder = appByFolder(known);
  const byConversation = appByConversation(known);
  const ofKey = new Map(known.map((place) => [knownKey(place), place]));
  const theirs = (item: RailItem): RailItem => {
    // A desktop session AI Juggler started: in the App its ended ones are
    // under (`appKeyOf`), so an App's open and past sessions are one group
    // (rc23: a CLI App's past ones and its open one sat apart).
    if (item.session && item.place.kind === "own" && item.place.key.startsWith("app|")) {
      const key = appKeyOf(item, byConversation, byFolder);
      if (key === item.place.key) return item;
      const place = ofKey.get(key);
      if (place) return { ...item, place: { ...item.place, key, app: knownAppWord(place), appId: place.app_id, profile: desktopProfile(place.instance, place.profile) } };
      return { ...item, place: cliPlace(item.session.provider, item.session.account_id, item.session.account_label, { name: OWN_APP, kind: "own", pid: null }) };
    }
    // ADR-0032 (D-100): a conversation is under the App the backend says
    // it belongs to (`appKeyOf` reads it); a terminal, an editor or tmux
    // is a detail of a CLI App's row, never a desktop App's.
    const conversation = item.conversation;
    if (!conversation) return item;
    const key = appKeyOf(item, byConversation, byFolder);
    if (key === item.place.key) return item;
    const place = ofKey.get(key);
    if (place) return { ...item, place: { key, app: knownAppWord(place), appId: place.app_id, kind: "desktop", profile: desktopProfile(place.instance, place.profile), appPid: place.app_pid, host: null } };
    if (key.startsWith("cli|")) {
      const first = conversation.places?.[0];
      return { ...item, place: cliPlace(conversation.tool, conversation.account_id, conversation.account_label, { name: first?.app || null, kind: first?.kind ?? null, pid: first?.app_pid ?? null }) };
    }
    return item;
  };
  const openAll = shown.filter((item) => openNow(item) || (want("background") && item.background && item.need === null)).map(theirs);
  const ended = shown.filter((item) => endedToday(item, now, want("archived"))).sort((a, b) => b.lastMs - a.lastMs);
  const background = shown.filter((item) => item.background && item.need === null);
  // The Project lens is the project board's; the Pool lens waits for a pool.
  const by: GroupBy = view.group === "account" ? "account" : view.group === "pool" && poolOf ? "pool" : "place";
  // D-64: by App, an inactive session is under its app, not a list apart.
  const apps = by === "place";
  // AJ-E3204: an App kept off the rail leaves Open now and Inactive apps
  // (a search still finds it); Needs you keeps its sessions.
  const off = (key: string) => apps && !searching && offRail(key);
  const open = openAll.filter((item) => !off(item.place.key));
  const offOpen = new Map<string, OffRail>();
  for (const item of openAll) {
    if (!off(item.place.key)) continue;
    const had = offOpen.get(item.place.key);
    if (had) had.open += 1;
    else offOpen.set(item.place.key, { key: item.place.key, short: item.place.profile ?? item.place.app, name: appName(item.place.app, item.place.profile), open: 1 });
  }
  const inactive = apps ? [] : shown.filter((item) => item.inactive && item.need === null && !item.pinned).sort((a, b) => b.lastMs - a.lastMs);
  const idle = apps ? idleSessionsByApp(shown, byConversation, byFolder) : new Map<string, RailItem[]>();
  // Its last ones, under an app: unfolded by a click; searching, every one
  // that matches (rc4 walk). Each sits under the one it was started from.
  const past = (key: string, sessions: readonly RailItem[]) => {
    const unfolded = searching || toggled.has(`${key}#recent`);
    // ADR-0032: this App's own; its pinned ones always show (never open).
    const pinned = sessions.filter((item) => item.pinned);
    const listed = unfolded ? (searching ? sessions : [...pinned, ...sessions.filter((item) => !item.pinned).slice(0, RECENT_INLINE)]) : pinned;
    return { open: unfolded, rows: nestRows(listed).map(({ item, depth, parent }) => ({ item, depth, parent: parent?.title ?? null })) };
  };
  const grouped = groupItems(open, by, accountOf, poolOf); // ADR-0032: + every running App:
  if (apps && !filtering) grouped.push(...runningGroups(known, new Set(grouped.map((group) => group.key)), off));
  const groups = arrangeGroups(grouped, order.mode, order.groups);
  // D-100: Open now shows only what's open; an open App keeps just its
  // pinned ended rows (its others: "Show recent", History, its page).
  const pinnedUnder = (key: string) => {
    for (const { item, depth, parent } of nestRows((idle.get(key) ?? []).filter((item) => item.pinned))) lines.push({ kind: "past", item, depth, parent: parent?.title ?? null, app: key });
  };
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
  // By App, Open now counts the Apps open; otherwise the sessions.
  if (groups.length > 0) lines.push({ kind: "section", id: "open", count: apps ? groups.length : open.length });
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
    // An App open with nothing in it: its header, saying so.
    if (folded || group.items.length === 0) {
      lines.push({ kind: "header", group, folded: quietWords(group), needs: needsHere, digit });
      pinnedUnder(group.key);
      return;
    }
    lines.push({ kind: "header", group, folded: null, needs: needsHere, digit });
    // Latest activity re-sorts, so it holds still under the pointer; the
    // other orders never move a row by themselves.
    const drawn = stable ? fold.shown : keep(`group:${group.key}`, fold.shown);
    // 0.3.9: a session started from another sits under it.
    for (const { item, depth, parent } of nestRows(drawn)) lines.push({ kind: "row", item, group, depth, parent: parent?.title ?? null });
    rows += drawn.length;
    if (fold.hidden > 0) lines.push({ kind: "more", group, quiet: fold.hidden, words: fold.words });
    pinnedUnder(group.key);
  });
  if (apps) {
    // While a search or filter is on, only the apps with something that
    // matches; one with something open isn't inactive, whatever it hides.
    const openKeys = new Set([...items.filter(openNow).map((item) => theirs(item).place.key), ...known.filter((place) => place.app_pid !== null).map(knownKey)]);
    const every = idleApps({ openKeys, known, idle });
    lines.push(...offeredLines(filtering ? [] : offers, known, [...groups.map((group) => group.key), ...every.map((app) => app.key)])); // D-100
    for (const app of every) if (off(app.key) && !offOpen.has(app.key)) offOpen.set(app.key, { key: app.key, short: app.profile ?? app.app, name: app.name, open: 0 });
    const resting = every.filter((app) => !off(app.key) && (!filtering || app.sessions.length > 0));
    if (resting.length > 0) {
      const all = filtering || toggled.has("idle#all");
      const drawn = all ? resting : resting.slice(0, IDLE_APPS_SHOWN);
      lines.push({ kind: "section", id: "idle", count: resting.length });
      for (const app of drawn) {
        const last = past(app.key, app.sessions);
        lines.push({ kind: "idle", app, open: last.open });
        for (const { item, depth, parent } of last.rows) lines.push({ kind: "past", item, depth, parent, app: app.key });
        rows += 1 + last.rows.length;
      }
      if (drawn.length < resting.length) lines.push({ kind: "idle-more", hidden: resting.length - drawn.length });
    }
  }
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
    offRail: [...offOpen.values()].sort((a, b) => a.name.localeCompare(b.name)),
  };
}

/** The lines with Needs you, Open now or Inactive apps folded: a folded
 * section keeps its head (with its count) and drops its rows and places. */
export function foldSections(lines: Line[], folded: ReadonlySet<SectionId>): Line[] {
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

/** A line's React key: what it shows, never its place in the list. */
export function lineKey(line: Line, index: number): string {
  switch (line.kind) {
    case "notice":
      return `notice:${line.notice.subject}`;
    case "need":
    case "plain":
    case "row":
      return `${line.kind}:${line.item.key}`;
    case "header":
    case "more":
      return `${line.kind}:${line.group.key}`;
    case "past":
      return `past:${line.app}:${line.item.key}`;
    case "recent":
      return `recent:${line.app}`;
    case "idle":
      return `idle:${line.app.key}`;
    case "offer":
      return `offer:${line.offer.id}`;
    default:
      return `${line.kind}:${"id" in line ? line.id : index}`;
  }
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

