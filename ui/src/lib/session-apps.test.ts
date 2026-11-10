/**
 * rc22 (A3): a session's App, one order for the backend and the window,
 * so its chip leads to the App that counts it. `fixtures/session-apps.json`
 * holds the cases; the backend's `places::app_of` runs the same file
 * (`crates/juggler-app/src/features/places/tests.rs`).
 */
import { describe, expect, it } from "vitest";

import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import { appIdOfRailKey } from "./apps";
import { appByConversation, appByFolder, appKeyOf } from "./rail-apps";
import { conversationItem, sessionItem } from "./rail-items";
import { setToolsForTests } from "./tools";
import { PREVIEW_TOOLS } from "../preview/tools";
import { appIdOfSession } from "../pages/session/AccountMenu";
import { fromConversation, fromInstance } from "../pages/session/model";
import cases from "./fixtures/session-apps.json";

setToolsForTests(PREVIEW_TOOLS);

const places: KnownPlaceInfo[] = cases.places.map((place) => ({
  id: place.id,
  app_id: "claude-app",
  app: "Claude",
  instance: place.instance,
  profile: place.instance,
  label: `Claude · ${place.instance}`,
  app_pid: null,
  lost: false,
  keep_alive: false,
  paused: null,
  account_id: place.folder,
  seen_ms: place.seen_ms,
  cannot_start: null,
  recent: place.recent,
  found: false,
}));

const byConversation = appByConversation(places);
const byFolder = appByFolder(places);
/** A rail key as the App id the backend names. */
const appOfKey = (key: string, folder: string) => appIdOfRailKey(key, folder, places);

function own(c: (typeof cases.own)[number]): InstanceInfo {
  return {
    id: c.id, launch_recipe_id: "r", provider: "claude-code", account_id: c.folder, account_label: c.folder, model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "finished", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: null, git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: 1, last_activity_ms: 1,
    machine_id: "this-mac", cwd: "/w", kind: c.kind, adopted: false, project_id: null, conversation_id: null, isolation_label: null, app_id: c.launched_in,
  };
}

const PLACE: Record<string, PlaceInfo> = {
  unnamed: { app: "", shown: "", app_id: null, profile: null, instance: null, pid: 7, kind: null, app_pid: null, label: "" },
  terminal: { app: "Terminal", shown: "Terminal", app_id: null, profile: null, instance: null, pid: 7, kind: "terminal", app_pid: 6, label: "Terminal" },
};

function other(c: (typeof cases.other)[number]): ConversationInfo {
  return {
    id: c.id, tool: "claude-code", account_id: c.folder, account_label: c.folder, project_id: null, cwd: "/w", title: "Talk", git_branch: null,
    started_ms: 1, last_activity_ms: 1, tokens: null, live: c.live, session_id: null, places: c.place ? [PLACE[c.place]!] : [], pid: null, state: null,
    probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, started_in: c.started_in,
  };
}

describe("a session's App is the backend's (places::app_of), on the rail and on its chip", () => {
  it.each(cases.own)("AI Juggler's $id ($kind on $folder, launched in $launched_in) → $app", (c) => {
    const row = own(c);
    expect(appOfKey(appKeyOf(sessionItem(row, () => null, false), byConversation, byFolder), c.folder)).toBe(c.app);
    expect(appIdOfSession(fromInstance(row), places)).toBe(c.app);
  });

  it.each(cases.other)("another app's $id (on $folder, live $live, $place, started in $started_in) → $app", (c) => {
    const row = other(c);
    expect(appOfKey(appKeyOf(conversationItem(row, () => null, null), byConversation, byFolder), c.folder)).toBe(c.app);
    expect(appIdOfSession(fromConversation(row), places)).toBe(c.app);
  });
});
