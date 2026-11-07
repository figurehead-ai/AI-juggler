/**
 * AJ-E3204 R3 — "Show in the rail", per App: an App turned off leaves
 * Open now and Inactive apps (by App), the Also line names it, and its
 * sessions that need you still show; a search still finds it.
 */
import { describe, expect, it } from "vitest";
import { conversationItem } from "./rail-items";
import { buildLines, type Line } from "./rail-lines";
import { appIdOfKey, parseHidden } from "./rail-shown";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { KnownPlaceInfo } from "./ipc/places";

const NOW = new Date(2026, 10, 6, 15, 0, 0).getTime();
const H = 3_600_000;

function place(profile: string): PlaceInfo {
  return { app: "Claude", shown: "Claude app", app_id: "claude-app", profile, instance: profile, pid: 10, kind: "desktop", app_pid: 11, label: `Claude · ${profile}` };
}

function talk(id: string, profile: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Work Max", project_id: null, cwd: "/w/app", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3 * H, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [place(profile)],
    pid: 10, state: "working", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}

function known(instance: string): KnownPlaceInfo {
  return {
    id: `claude-app|/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null, lost: false,
    keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false,
  };
}

const PLACES = [known("Work"), known("Home"), known("Spare")];
const WORK = "app|claude-app|Work";
const SPARE = "app|claude-app|Spare";

function lines(rows: ConversationInfo[], off: string[], query = "") {
  const hidden = new Set(off);
  return buildLines({
    items: rows.map((row) => conversationItem(row, () => null, null)),
    view: DEFAULT_VIEW,
    query: parseQuery(query),
    now: NOW,
    toggled: new Set(),
    frozen: null,
    known: PLACES,
    offRail: (key) => hidden.has(key),
  });
}
const headers = (out: Line[]) => out.flatMap((line) => (line.kind === "header" ? [line.group.key] : line.kind === "idle" ? [`idle:${line.app.key}`] : []));

describe("AJ-E3204 an App kept off the rail", () => {
  const rows = [talk("w1", "Work"), talk("w2", "Work"), talk("h1", "Home")];

  it("shows every App when none is kept off", () => {
    const out = lines(rows, []);
    expect(headers(out.lines)).toEqual(expect.arrayContaining([WORK, "app|claude-app|Home", `idle:${SPARE}`]));
    expect(out.offRail).toEqual([]);
  });

  it("leaves Open now and Inactive apps, and the Also line names it with what's open", () => {
    const out = lines(rows, [WORK, SPARE]);
    expect(headers(out.lines)).toEqual(["app|claude-app|Home"]);
    expect(out.lines.find((line) => line.kind === "section" && line.id === "open")).toMatchObject({ count: 1 });
    expect(out.offRail).toEqual([
      { key: SPARE, short: "Spare", name: "Claude app · Spare", open: 0 },
      { key: WORK, short: "Work", name: "Claude app · Work", open: 2 },
    ]);
    expect(out.lines.some((line) => line.kind === "also")).toBe(true);
  });

  it("still shows its session that needs you", () => {
    const out = lines([talk("ask", "Work", { state: "asks_you" }), talk("h1", "Home")], [WORK]);
    expect(out.lines.filter((line) => line.kind === "need").map((line) => (line as { item: { key: string } }).item.key)).toEqual(["c:a1/ask"]);
  });

  it("a search still finds it", () => {
    const out = lines(rows, [WORK], "Talk w1");
    expect(headers(out.lines)).toContain(WORK);
    expect(out.offRail).toEqual([]);
  });
});

describe("the stored list", () => {
  it("reads a JSON list of App ids, anything else as nothing kept", () => {
    expect([...parseHidden('["cli:a1","claude-app|/p/Work"]')]).toEqual(["cli:a1", "claude-app|/p/Work"]);
    expect(parseHidden(null).size).toBe(0);
    expect(parseHidden("").size).toBe(0);
    expect(parseHidden("not json").size).toBe(0);
    expect([...parseHidden('["x", 3, null]')]).toEqual(["x"]);
  });

  it("a rail key's App: a CLI App by its folder, a desktop App by the places remembered", () => {
    expect(appIdOfKey("cli|claude-code|a1", null)).toBe("cli:a1");
    expect(appIdOfKey(WORK, PLACES)).toBe("claude-app|/p/Work");
    expect(appIdOfKey("app|claude-app|Gone", PLACES)).toBeNull();
  });
});
