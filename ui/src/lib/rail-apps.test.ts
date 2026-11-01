/**
 * D-64 (0.3.4): grouped by App, the rail is about apps. An app's sessions
 * that aren't open sit under it ("Last 3 inactive"); the apps with
 * nothing open follow in "Inactive apps", most recently used first.
 */
import { describe, expect, it } from "vitest";
import { conversationItem } from "./rail-items";
import { appByConversation, idleApps, idleSessionsByApp, recentWords } from "./rail-apps";
import { buildLines, foldSections, type Line } from "./rail-lines";
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

/** A session that isn't open any more, `hours` ago. */
const past = (id: string, hours: number, over: Partial<ConversationInfo> = {}) => talk(id, { live: false, places: [], pid: null, state: null, last_activity_ms: NOW - hours * H, ...over });

function known(instance: string, over: Partial<KnownPlaceInfo> = {}): KnownPlaceInfo {
  return {
    id: `claude-app|/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null,
    keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], ...over,
  };
}

const itemsOf = (rows: ConversationInfo[]) => rows.map((row) => conversationItem(row, none, null));
const lines = (rows: ConversationInfo[], places: KnownPlaceInfo[], toggled: string[] = [], query = "") =>
  buildLines({ items: itemsOf(rows), view: DEFAULT_VIEW, query: parseQuery(query), now: NOW, toggled: new Set(toggled), frozen: null, known: places });
const kinds = (out: Line[]) =>
  out.map((line) =>
    line.kind === "section"
      ? `section:${line.id}:${line.count}`
      : line.kind === "header"
        ? `header:${line.group.key}`
        : line.kind === "recent"
          ? `recent:${line.count}`
          : line.kind === "idle"
            ? `idle:${line.app.key}`
            : line.kind === "row" || line.kind === "past" || line.kind === "plain" || line.kind === "need"
              ? `${line.kind}:${line.item.key}`
              : line.kind,
  );

describe("D-64 an app's sessions that aren't open belong to it", () => {
  it("a desktop app's: where they were last seen open, the most recent app winning", () => {
    const by = appByConversation([known("Old", { recent: ["claude-code/x"], seen_ms: NOW - 9 * H }), known("Work", { recent: ["claude-code/x", "claude-code/y"] })]);
    expect(by.get("claude-code/x")).toBe("app|claude-app|Work");
    expect(by.get("claude-code/y")).toBe("app|claude-app|Work");
  });

  it("leaves out open, archived and needing ones; newest first", () => {
    // An archived one (AI Juggler's own sessions archive), and one asking.
    const items = itemsOf([talk("open"), past("old", 9), past("new", 1), past("gone", 2), past("asks", 3)]).map((item) =>
      item.key === "c:a1/gone" ? { ...item, archived: true } : item.key === "c:a1/asks" ? { ...item, need: "asks" as never } : item,
    );
    const idle = idleSessionsByApp(items, appByConversation([known("Work", { recent: ["claude-code/old", "claude-code/new", "claude-code/gone", "claude-code/asks"] })]));
    expect([...idle.keys()]).toEqual(["app|claude-app|Work"]);
    expect(idle.get("app|claude-app|Work")!.map((item) => item.key)).toEqual(["c:a1/new", "c:a1/old"]);
  });

  it("the apps with nothing open: remembered desktop apps, and CLI apps with sessions of their own, most recent first", () => {
    const items = itemsOf([talk("w1"), past("w2", 1), past("c1", 2, { account_id: "a2", account_label: "Personal" })]);
    const places = [known("Work", { recent: ["claude-code/w2"] }), known("Lab", { seen_ms: NOW - 30 * 60_000, app_pid: 42 }), known("Old", { seen_ms: NOW - 9 * 24 * H })];
    const idle = idleSessionsByApp(items, appByConversation(places));
    const apps = idleApps({ openKeys: new Set(["app|claude-app|Work"]), known: places, idle });
    expect(apps.map((app) => [app.key, app.kind])).toEqual([
      ["app|claude-app|Lab", "desktop"],
      ["cli|claude-code|a2", "cli"],
      ["app|claude-app|Old", "desktop"],
    ]);
    expect(apps[0]!.appPid).toBe(42);
    expect(apps[1]).toMatchObject({ accountId: "a2", tool: "claude-code", profile: "Personal" });
    expect(apps[1]!.sessions.map((item) => item.key)).toEqual(["c:a2/c1"]);
  });

  it("says how many it shows", () => {
    expect(recentWords(9)).toBe("Last 3 inactive");
    expect(recentWords(2)).toBe("Last 2 inactive");
  });
});

describe("D-64 the rail grouped by App", () => {
  const rows = [talk("w1"), past("w2", 1), past("w3", 2), past("w4", 3), past("w5", 4), past("c1", 2, { account_id: "a2", account_label: "Personal" })];
  const places = [known("Work", { recent: ["claude-code/w2", "claude-code/w3", "claude-code/w4", "claude-code/w5"], app_pid: 11 }), known("Lab")];

  it("an open app's inactive sessions wait behind its Last 3; the apps with nothing open follow Open now", () => {
    const built = lines(rows, places);
    expect(kinds(built.lines)).toEqual([
      "section:open:1",
      "header:app|claude-app|Work",
      "row:c:a1/w1",
      "recent:4",
      "section:idle:2",
      "idle:cli|claude-code|a2",
      "idle:app|claude-app|Lab",
      "also",
    ]);
    // Nothing on Also for them, and no list apart.
    expect(built.also.inactive).toBe(0);
  });

  it("a click shows the last 3 under the app, and an inactive app's under it", () => {
    const built = lines(rows, places, ["app|claude-app|Work#recent", "cli|claude-code|a2#recent"]);
    expect(kinds(built.lines)).toEqual([
      "section:open:1",
      "header:app|claude-app|Work",
      "row:c:a1/w1",
      "recent:4",
      "past:c:a1/w2",
      "past:c:a1/w3",
      "past:c:a1/w4",
      "section:idle:2",
      "idle:cli|claude-code|a2",
      "past:c:a2/c1",
      "idle:app|claude-app|Lab",
      "also",
    ]);
  });

  it("shows 6 inactive apps, then Show n more", () => {
    const many = Array.from({ length: 8 }, (_, n) => known(`P${n}`, { seen_ms: NOW - (n + 1) * H }));
    const out = kinds(lines([talk("w1")], many).lines);
    expect(out.filter((line) => line.startsWith("idle:"))).toHaveLength(6);
    expect(out).toContain("idle-more");
    expect(kinds(lines([talk("w1")], many, ["idle#all"]).lines).filter((line) => line.startsWith("idle:"))).toHaveLength(8);
  });

  it("while searching: only the apps with a match, their matches shown", () => {
    const out = kinds(lines(rows, places, [], "Talk c1").lines);
    expect(out).toEqual(["section:idle:1", "idle:cli|claude-code|a2", "past:c:a2/c1", "also"]);
  });

  it("while searching, an app with something open is never an inactive app", () => {
    // w2 matches, w1 (open in the same app) doesn't.
    const out = kinds(lines(rows, places, [], "Talk w2").lines);
    expect(out.some((line) => line === "idle:app|claude-app|Work")).toBe(false);
  });

  it("Inactive apps folds from its head, as Open now does", () => {
    const out = kinds(foldSections(lines(rows, places).lines, new Set(["idle"])));
    expect(out.slice(-2)).toEqual(["section:idle:2", "also"]);
  });

  it("by account, inactive sessions stay a view of their own", () => {
    const built = buildLines({ items: itemsOf([talk("w1"), past("w2", 1, { inactive: true })]), view: { ...DEFAULT_VIEW, group: "account", show: { ...DEFAULT_VIEW.show, inactive: true } }, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places });
    const out = kinds(built.lines);
    expect(out).not.toContain("section:idle:1");
    expect(out).toContain("plain:c:a1/w2");
    expect(built.also.inactive).toBe(1);
  });
});
