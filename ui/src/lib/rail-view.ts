/**
 * AJ-E2104 — the rail's View (⇧V, and its button): Show and Only, plus
 * Reset; 0.3: Group by is the rail's own switch (App · Project · Account
 * · Pool), kept in the same store. Kept per Mac in this window's storage,
 * read guarded (junk or a storage that throws is the defaults), and one
 * store for every reader (the rail, its button's dot, Home's links).
 *
 * The same words work in search and ⌘K: `is:ended is:inactive
 * is:background is:quiet is:archived is:asks app: profile: account:
 * project: mac:`, then free text.
 */
import { useSyncExternalStore } from "react";
import type { AccountOf, GroupBy } from "./rail-places";
import { appName } from "./app-name";
import { itemsOf, type RailItem } from "./rail-items";
import type { InstanceInfo, ProjectInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";

export const SHOW_KEYS = ["inactive", "quiet", "ended", "background", "subagents", "otherMacs", "archived"] as const;
export type ShowKey = (typeof SHOW_KEYS)[number];
export const ONLY_KEYS = ["app", "profile", "account", "project", "mac"] as const;
export type OnlyKey = (typeof ONLY_KEYS)[number];

export interface RailView {
  group: GroupBy;
  show: Record<ShowKey, boolean>;
  only: Partial<Record<OnlyKey, string>>;
}

/** All off: quiet places fold, ended and inactive wait behind "Also:",
 * other Macs fold after this Mac. */
export const DEFAULT_VIEW: RailView = {
  group: "place",
  show: { inactive: false, quiet: false, ended: false, background: false, subagents: false, otherMacs: false, archived: false },
  only: {},
};

export const SHOW_LABELS: Record<ShowKey, string> = {
  inactive: "Inactive",
  quiet: "Quiet ones",
  ended: "Ended today",
  background: "Background runs",
  subagents: "Subagents",
  otherMacs: "Other Macs",
  archived: "Archived",
};

/** What each one is, beside it, only where the label needs it (the 0.2
 * walk, F18: "Quiet ones" and "Inactive" said nothing of the difference;
 * 0.3 polish: short, on the label's line, so every line is one height). */
export const SHOW_NOTES: Partial<Record<ShowKey, string>> = {
  inactive: "in an app's list, not running",
  quiet: "idle ones, folded away",
  background: "from scripts",
  subagents: "a count on each session",
};

const KEY = "aij.rail.view.v1";
const GROUPS: GroupBy[] = ["place", "project", "pool", "account"];

/** A stored view, checked field by field. */
export function parseView(raw: string | null): RailView {
  if (!raw) return DEFAULT_VIEW;
  try {
    const value = JSON.parse(raw) as Partial<RailView>;
    // Pool is kept as picked; with no pool of yours, the rail shows places.
    const group = GROUPS.includes(value.group as GroupBy) ? (value.group as GroupBy) : "place";
    const show = { ...DEFAULT_VIEW.show };
    for (const key of SHOW_KEYS) if (value.show?.[key] === true) show[key] = true;
    const only: RailView["only"] = {};
    for (const key of ONLY_KEYS) {
      const picked = value.only?.[key];
      if (typeof picked === "string" && picked.length > 0 && picked.length < 200) only[key] = picked;
    }
    return { group, show, only };
  } catch {
    return DEFAULT_VIEW;
  }
}

function read(): RailView {
  try {
    return parseView(localStorage.getItem(KEY));
  } catch {
    return DEFAULT_VIEW;
  }
}

let current: RailView | null = null;
const listeners = new Set<() => void>();

export function railView(): RailView {
  current ??= read();
  return current;
}

export function setRailView(next: RailView | ((view: RailView) => RailView)): void {
  current = typeof next === "function" ? next(railView()) : next;
  try {
    localStorage.setItem(KEY, JSON.stringify(current));
  } catch {
    // Not kept this time; the view still applies.
  }
  for (const listener of listeners) listener();
}

/** Forget the cached view (tests, a storage reset). */
export function resetRailViewCache(): void {
  current = null;
}

export function useRailView(): RailView {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    railView,
    railView,
  );
}

/** Fired with the Show part just turned on: the list brings it into view. */
export const RAIL_REVEAL_EVENT = "aij:rail-reveal";

/** Turns one Show part on (the "Also:" line, Home's links), and the
 * list scrolls to it: a click always shows what it turned on. */
export function showInRail(key: ShowKey, on = true): void {
  setRailView((view) => ({ ...view, show: { ...view.show, [key]: on } }));
  if (on) window.dispatchEvent(new CustomEvent(RAIL_REVEAL_EVENT, { detail: { key } }));
}

export function isDefaultView(view: RailView): boolean {
  return view.group === "place" && !viewChanged(view);
}

/** 0.3: whether the View's own parts (Show, Only) differ from the
 * defaults: its dot and its Reset. The grouping is the rail's switch,
 * always in sight, so it never counts here. */
export function viewChanged(view: RailView): boolean {
  return SHOW_KEYS.some((key) => view.show[key]) || ONLY_KEYS.some((key) => view.only[key] !== undefined);
}

/** The View's Reset: Show and Only back to the defaults, the grouping
 * kept. */
export function resetViewParts(): void {
  setRailView((view) => ({ ...DEFAULT_VIEW, group: view.group }));
}

/** The rail's switch: group sessions by app, project, account or pool. */
export function setGroupBy(group: GroupBy): void {
  setRailView((view) => ({ ...view, group }));
}

// --- the words search and ⌘K take ------------------------------------------------

const IS_WORDS: Record<string, ShowKey | "asks"> = {
  ended: "ended",
  inactive: "inactive",
  background: "background",
  quiet: "quiet",
  archived: "archived",
  asks: "asks",
  subagents: "subagents",
};

export interface RailQuery {
  /** The free text, lower case. */
  text: string;
  is: Set<ShowKey | "asks">;
  only: Partial<Record<OnlyKey, string>>;
}

/** `is:ended app:claude fix login` → the parts. Values are lower case;
 * an unknown `word:` is free text. */
export function parseQuery(input: string): RailQuery {
  const query: RailQuery = { text: "", is: new Set(), only: {} };
  const words: string[] = [];
  for (const word of input.trim().split(/\s+/).filter(Boolean)) {
    const at = word.indexOf(":");
    const name = at > 0 ? word.slice(0, at).toLowerCase() : "";
    const value = at > 0 ? word.slice(at + 1).toLowerCase() : "";
    if (name === "is" && IS_WORDS[value]) query.is.add(IS_WORDS[value]!);
    else if ((ONLY_KEYS as readonly string[]).includes(name) && value) query.only[name as OnlyKey] = value;
    else words.push(word.toLowerCase());
  }
  query.text = words.join(" ");
  return query;
}

/** Whether anything in the query narrows or widens the list. */
export function isSearching(query: RailQuery): boolean {
  return query.text !== "" || query.is.size > 0 || Object.keys(query.only).length > 0;
}

/** An item's value for an Only key (a Mac is "this-mac" here). D-62:
 * `app` is the App ("Claude Code · Work Max"); `account` the Account
 * (subscription) it's signed in to, once `accountOf` knows it; `profile`
 * stays the App's profile, so a View kept with it still works. */
export function onlyValue(item: RailItem, key: OnlyKey, accountOf?: AccountOf): string {
  switch (key) {
    case "app":
      return appName(item.place.app, item.place.profile);
    case "profile":
      return item.place.profile ?? "";
    case "account":
      return accountOf?.(item.accountId)?.label ?? item.accountLabel;
    case "project":
      return item.project ?? "";
    case "mac":
      return item.session?.machine_id ?? "this-mac";
  }
}

/** ⌘K's sessions for a query: the open ones (and the ended or inactive
 * ones when `is:` asks for them) that match its words. */
export function paletteMatches(
  sessions: InstanceInfo[],
  conversations: ConversationInfo[],
  projects: ProjectInfo[],
  query: RailQuery,
): { sessions: InstanceInfo[]; conversations: ConversationInfo[] } {
  // AI Juggler's own: the running ones (as ⌘K always listed them).
  const open = (item: RailItem) => (item.session ? item.session.status === "running" || item.session.status === "starting" : item.live);
  const wanted = (item: RailItem) =>
    (open(item) && (!item.background || query.is.has("background"))) ||
    (query.is.has("ended") && item.state === "ended" && !item.inactive) ||
    (query.is.has("inactive") && item.inactive);
  const items = itemsOf({ sessions, conversations, projects, seen: {} }).filter((item) => wanted(item) && matches(item, {}, query));
  return {
    sessions: items.flatMap((item) => (item.session ? [item.session] : [])),
    conversations: items.flatMap((item) => (item.conversation ? [item.conversation] : [])),
  };
}

/** Whether an item passes the View's Only and the query's words (`app:`
 * matches the start of a word, of the App or of where its window is
 * (`app:terminal`); free text anywhere in what the row says). */
export function matches(item: RailItem, only: RailView["only"], query: RailQuery, accountOf?: AccountOf): boolean {
  for (const key of ONLY_KEYS) {
    const picked = only[key];
    if (picked !== undefined && onlyValue(item, key, accountOf) !== picked) return false;
    const typed = query.only[key];
    if (typed === undefined) continue;
    const value = (key === "app" ? [onlyValue(item, key), item.place.host ?? ""].join(" ") : onlyValue(item, key, accountOf)).toLowerCase();
    if (!value.split(/[\s·/_-]+/).some((word) => word.startsWith(typed)) && !value.startsWith(typed)) return false;
  }
  if (query.is.has("asks") && item.need === null) return false;
  if (query.text === "") return true;
  const haystack = [item.title, item.project ?? "", item.place.app, item.place.profile ?? "", item.place.host ?? "", item.accountLabel, accountOf?.(item.accountId)?.label ?? "", item.tool, item.session?.git_branch ?? item.conversation?.git_branch ?? "", item.session?.model ?? ""]
    .join(" ")
    .toLowerCase();
  return query.text.split(" ").every((word) => haystack.includes(word));
}
