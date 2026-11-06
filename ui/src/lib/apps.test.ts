/**
 * D-62 (AJ-E3204): the Apps their page and doors see. One per folder in
 * use and one per desktop profile seen; the user's name wins; each knows
 * the rail's key for it, and its page lists just the sessions the rail
 * puts under it (A1: a Claude app profile's page has only its own).
 */
import { describe, expect, it } from "vitest";
import { appIdOfRailKey, appsOf } from "./apps";
import { appItems, appProjects } from "./app-items";
import { appByConversation } from "./rail-apps";
import { conversationItem } from "./rail-items";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { KnownPlaceInfo } from "./ipc/places";
import type { StoredAppInfo } from "./ipc/apps";
import { setToolsForTests } from "../lib/tools";
import { PREVIEW_TOOLS } from "../preview/tools";

// This file's top level reads tool names before the preview's data loads (src/test/setup.ts).
setToolsForTests(PREVIEW_TOOLS);

const NOW = new Date(2026, 10, 2, 15, 0, 0).getTime();
const H = 3_600_000;

const folder = (id: string, label: string): AccountInfo => ({ id, label, provider: "claude-code", login_state: "logged_in" }) as AccountInfo;

function known(instance: string, over: Partial<KnownPlaceInfo> = {}): KnownPlaceInfo {
  return {
    id: `app:claude-app:/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null, lost: false,
    keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false, ...over,
  };
}

const stored = (id: string, over: Partial<StoredAppInfo> = {}): StoredAppInfo => ({
  id, kind: id.startsWith("cli:") ? "cli" : "desktop", tool: "claude-code", host_app_id: null, label: null, folder_id: null, account_id: null, last_seen_ms: 0, ...over,
});

function deskPlace(profile: string): PlaceInfo {
  return { app: "Claude", shown: "Claude app", app_id: "claude-app", profile, instance: profile, pid: 10, kind: "desktop", app_pid: 11, label: `Claude · ${profile}` };
}

function talk(id: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "work", account_label: "Work Max", project_id: "p1", cwd: "/w/app", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3 * H, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [deskPlace("Lab")],
    pid: 10, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}

const past = (id: string, hours: number, over: Partial<ConversationInfo> = {}) => talk(id, { live: false, places: [], pid: null, state: null, last_activity_ms: NOW - hours * H, ...over });

describe("D-62 the Apps", () => {
  it("one per folder and one per desktop profile, named by app and profile unless you named it", () => {
    const apps = appsOf({
      stored: [stored("cli:work", { label: "Day job", account_id: "acct:anthropic:1" }), stored("app:claude-app:/p/Lab", { folder_id: "work", account_id: "acct:anthropic:1" })],
      places: [known("Lab")],
      folders: [folder("work", "Work Max"), folder("side", "Side")],
    });
    expect(apps.map((app) => [app.id, app.name, app.railKey])).toEqual([
      ["cli:work", "Day job", "cli|claude-code|work"],
      ["cli:side", "Claude Code · Side", "cli|claude-code|side"],
      ["app:claude-app:/p/Lab", "Claude app · Lab", "app|claude-app|Lab"],
    ]);
    const lab = apps[2]!;
    expect(lab.folder?.id).toBe("work");
    expect(lab.accountId).toBe("acct:anthropic:1");
    expect(apps[0]!.named && !apps[1]!.named).toBe(true);
  });

  it("without stored rows (the preview), every App still comes from its folders and places", () => {
    const apps = appsOf({ stored: [], places: [known("Lab", { account_id: "work" })], folders: [folder("work", "Work Max")] });
    expect(apps.map((app) => app.name)).toEqual(["Claude Code · Work Max", "Claude app · Lab"]);
    expect(apps[1]!.folder?.id).toBe("work");
  });

  it("AJ-E3203: a profile is named by the config folder its sessions use, else by its own folder; one found beside it is an App too", () => {
    const apps = appsOf({
      stored: [],
      places: [
        known("b-team", { profile: "team-max", account_id: "team" }),
        known("b-side", { found: true }),
        known("", { id: "app:claude-app:", instance: null, profile: "Default" }),
      ],
      folders: [],
    });
    const desktop = apps.filter((app) => app.kind === "desktop");
    expect(desktop.map((app) => [app.name, app.profile])).toEqual([
      ["Claude app · team-max", "team-max"],
      ["Claude app · b-side", "b-side"],
      ["Claude app", null],
    ]);
    // The rail still keys a profile by its own folder.
    expect(desktop[0]!.railKey).toBe("app|claude-app|b-team");
    expect(desktop[1]!.place?.found).toBe(true);
  });

  it("a rail group's App: a CLI group by its folder, a desktop one by its remembered place", () => {
    const places = [known("Lab")];
    expect(appIdOfRailKey("cli|claude-code|work", "work", places)).toBe("cli:work");
    expect(appIdOfRailKey("cli|claude-code|work", null, places)).toBeNull();
    expect(appIdOfRailKey("app|claude-app|Lab", null, places)).toBe("app:claude-app:/p/Lab");
    expect(appIdOfRailKey("app|claude-app|Gone", null, places)).toBeNull();
  });
});

describe("D-62 an App's page lists only its own sessions (A1)", () => {
  const items = [
    talk("open-in-lab"),
    past("ended-in-lab", 2),
    past("ended-in-lab-older", 30, { project_id: "p2" }),
    past("cli-only", 1),
    past("archived-in-lab", 3),
  ].map((row) => conversationItem(row, (id) => (id === "p2" ? "docs" : "storefront"), null));
  items[4]!.archived = true;
  const by = appByConversation([known("Lab", { recent: ["claude-code/ended-in-lab", "claude-code/ended-in-lab-older", "claude-code/archived-in-lab"] })]);

  it("open first, then the most recent; the terminal's and archived ones left out", () => {
    const lab = appItems(items, "app|claude-app|Lab", by);
    expect(lab.map((item) => item.conversation?.id)).toEqual(["open-in-lab", "ended-in-lab", "ended-in-lab-older"]);
    const cli = appItems(items, "cli|claude-code|work", by);
    expect(cli.map((item) => item.conversation?.id)).toEqual(["cli-only"]);
  });

  it("its projects, the most recently used first, with how many sessions each (A4)", () => {
    const lab = appItems(items, "app|claude-app|Lab", by);
    expect(appProjects(lab).map((project) => [project.name, project.sessions])).toEqual([
      ["storefront", 2],
      ["docs", 1],
    ]);
  });
});
