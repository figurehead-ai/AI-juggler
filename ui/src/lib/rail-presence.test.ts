/**
 * ADR-0032 (D-100): the rail's Apps are the Apps list. Open now and
 * Inactive apps together are every App Accounts lists, each once, and
 * Inactive's count is all of them (a CLI App whose sessions the window
 * hasn't read included).
 */
import { describe, expect, it } from "vitest";
import { conversationItem } from "./rail-items";
import { knownKey, type CliAppFacts } from "./rail-apps";
import { buildLines } from "./rail-lines";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { KnownPlaceInfo } from "./ipc/places";
import { cliAppKey } from "./app-name";

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
    id: `claude-app|/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null, lost: false,
    keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false, ...over,
  };
}

const itemsOf = (rows: ConversationInfo[]) => rows.map((row) => conversationItem(row, none, null));

const cli = (folder: string, label: string): CliAppFacts => ({ railKey: cliAppKey("claude-code", folder, label), name: `Claude Code CLI · ${label}`, app: "Claude Code CLI", profile: label, appId: "claude-code", tool: "claude-code", folder: { id: folder } });

describe("ADR-0032 the rail's Apps are the Apps list", () => {
  it("Open now ∪ Inactive apps is every App of Accounts' list, each once, a CLI App with no session read included", () => {
    const places = [known("Work", { id: "app:claude-app:/p/Work", app_pid: 7 }), known("Lab", { id: "app:claude-app:/p/Lab" })];
    const cliApps = [cli("a1", "Work Max"), cli("old", "Default")];
    const rows = [talk("live", { places: [place("Work")], app: "app:claude-app:/p/Work" }), past("x", 3)];
    const out = buildLines({ items: itemsOf(rows), view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(["idle#all"]), frozen: null, known: places, cliApps }).lines;
    const open = out.flatMap((line) => (line.kind === "header" ? [line.group.key] : []));
    const inactive = out.flatMap((line) => (line.kind === "idle" ? [line.app.key] : []));
    const all = [...places.map(knownKey), ...cliApps.map((app) => app.railKey)];
    expect([...open, ...inactive].sort()).toEqual([...all].sort());
    expect(new Set([...open, ...inactive]).size).toBe(open.length + inactive.length);
    const head = out.find((line) => line.kind === "section" && line.id === "idle");
    expect(head?.kind === "section" ? head.count : -1).toBe(inactive.length);
  });
});
