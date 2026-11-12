/**
 * 0.4.3 (the maintainer's 0.4.2 test): three running Claude app profiles
 * read "nothing open" with a chevron that unfolded nothing, their sessions
 * only behind a menu, and none wore its Account's Out mark. A running App
 * unfolds to its sessions (`rail-unfold.ts`); a header has a fold only
 * when unfolding shows a row; a folded one shows nothing under it; each
 * App group carries its App's own folder for its mark.
 */
import { describe, expect, it } from "vitest";
import { conversationItem } from "./rail-items";
import { appFolderOf } from "./rail-apps";
import { buildLines, type Line } from "./rail-lines";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { KnownPlaceInfo } from "./ipc/places";

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();
const H = 3_600_000;
const none = () => null;

function place(profile: string): PlaceInfo {
  return { app: "Claude", shown: "Claude app", app_id: "claude-app", profile, instance: profile, pid: 10, kind: "desktop", app_pid: 11, label: `Claude · ${profile}` };
}

function talk(id: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Work Max", project_id: null, cwd: "/w/app", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3 * H, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [place("Work")],
    pid: 10, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}

const past = (id: string, hours: number, over: Partial<ConversationInfo> = {}) => talk(id, { live: false, places: [], pid: null, state: null, last_activity_ms: NOW - hours * H, ...over });

function known(instance: string, over: Partial<KnownPlaceInfo> = {}): KnownPlaceInfo {
  return {
    id: `claude-app|/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null, lost: false,
    keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false, ...over,
  };
}
const running = (instance: string, pid: number, over: Partial<KnownPlaceInfo> = {}) => known(instance, { id: `app:claude-app:/p/${instance}`, app_pid: pid, ...over });

const itemsOf = (rows: ConversationInfo[]) => rows.map((row) => conversationItem(row, none, null));
const lines = (rows: ConversationInfo[], places: KnownPlaceInfo[], toggled: string[] = []) =>
  buildLines({ items: itemsOf(rows), view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(toggled), frozen: null, known: places });
const kinds = (out: Line[]) =>
  out.map((line) => (line.kind === "section" ? `section:${line.id}:${line.count}` : line.kind === "header" ? `header:${line.group.key}` : line.kind === "past" ? `past:${line.item.key}` : line.kind));

describe("0.4.3 a running App unfolds to its sessions", () => {
  // 0.4.3 (the maintainer's 0.4.2 test: three running profiles read
  // "nothing open" with a chevron that unfolded nothing, none marked Out).
  it("a running App with nothing open unfolds to its last 3 at once, pinned first, then Show all; never a dead chevron", () => {
    const places = [running("East", 41, { account_id: "a1" }), running("West", 42, { account_id: "a9" })];
    const ended = [1, 2, 3, 4, 5].map((n) => past(`e${n}`, n, { app: "app:claude-app:/p/East" }));
    const items = itemsOf(ended).map((item) => (item.key === "c:a1/e5" ? { ...item, pinned: true } : item));
    const out = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places }).lines;
    expect(kinds(out)).toEqual([
      "section:open:2",
      "header:app|claude-app|East",
      "past:c:a1/e5",
      "past:c:a1/e1",
      "past:c:a1/e2",
      "past:c:a1/e3",
      "all",
      "header:app|claude-app|West",
      "also",
    ]);
    const heads = out.flatMap((line) => (line.kind === "header" ? [line] : []));
    expect(heads.map((line) => [line.group.key, line.foldable])).toEqual([
      ["app|claude-app|East", true],
      ["app|claude-app|West", false],
    ]);
    const all = out.find((line) => line.kind === "all");
    expect(all && all.kind === "all" ? [all.app, all.count] : null).toEqual(["app|claude-app|East", 5]);
  });

  it("an App lists only what its own records list: one its app no longer lists (deleted there) isn't under it", () => {
    const places = [running("East", 41, { account_id: "a1" })];
    const rows = [past("kept", 1, { app: "app:claude-app:/p/East" }), past("deleted", 2, { app: "app:claude-app:/p/East", app_unlisted: true })];
    const out = lines(rows, places).lines;
    expect(kinds(out)).toEqual(["section:open:1", "header:app|claude-app|East", "past:c:a1/kept", "also"]);
  });

  it("invariant: every header with a fold unfolds to at least one row, in every App shape", () => {
    const places = [running("Empty", 1), running("Past", 2), running("Busy", 3), known("Closed", { id: "app:claude-app:/p/Closed" })];
    const rows = [
      talk("live", { places: [place("Busy")], app: "app:claude-app:/p/Busy" }),
      past("p1", 2, { app: "app:claude-app:/p/Past" }),
      past("b1", 3, { app: "app:claude-app:/p/Busy" }),
      past("c1", 4, { app: "app:claude-app:/p/Closed" }),
    ];
    for (const toggled of [[], ["app|claude-app|Busy#recent"]]) {
      const out = lines(rows, places, toggled).lines;
      out.forEach((line, at) => {
        if (line.kind !== "header" || !line.foldable || line.folded !== null) return;
        const next = out[at + 1];
        expect(next && (next.kind === "row" || next.kind === "past"), `${line.group.key} unfolds to a row`).toBe(true);
      });
      for (const line of out) if (line.kind === "header" && !line.foldable) expect(out[out.indexOf(line) + 1]?.kind).not.toMatch(/^(row|past)$/);
    }
  });

  it("folded by the user, a running App shows nothing under it: its pinned row never sits under a folded header", () => {
    const places = [running("South", 7, { account_id: "a1" }), running("North", 8, { account_id: "a2" })];
    const items = itemsOf([past("jive", 22, { app: "app:claude-app:/p/South" })]).map((item) => ({ ...item, pinned: true }));
    const order = { mode: "yours" as const, rows: new Map(), groups: [], folded: new Set(["place:app|claude-app|South"]) };
    const out = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places, order }).lines;
    expect(kinds(out)).toEqual(["section:open:2", "header:app|claude-app|South", "header:app|claude-app|North", "also"]);
    // Unfolded, it's under its own App.
    const open = lines([past("jive", 22, { app: "app:claude-app:/p/South" })], places).lines;
    expect(kinds(open).slice(0, 3)).toEqual(["section:open:2", "header:app|claude-app|South", "past:c:a1/jive"]);
  });

  it("invariant: by App, each Open now group carries its App's own folder for its Out mark, never its first row's", () => {
    const places = [running("South", 7, { account_id: "out-folder" }), running("Busy", 9, { account_id: "busy-folder" })];
    // A session in Busy runs on another Account's folder: Busy is still Busy's.
    const rows = [talk("x", { places: [place("Busy")], app: "app:claude-app:/p/Busy", account_id: "other-folder" })];
    const out = lines(rows, places).lines;
    const folders = Object.fromEntries(out.flatMap((line) => (line.kind === "header" ? [[line.group.key, line.group.accountId]] : [])));
    expect(folders).toEqual({ "app|claude-app|South": "out-folder", "app|claude-app|Busy": "busy-folder" });
    expect(appFolderOf({ key: "cli|claude-code|f1", items: [] }, null)).toBe("f1");
  });

});
