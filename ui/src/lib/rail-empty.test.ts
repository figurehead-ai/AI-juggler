/**
 * AJ-E2327 — the rail's empty state shows only when Open now is really
 * empty, worded for its lens. The fresh 0.5.0 install read "Open now 3"
 * (three running Apps whose sessions it couldn't read) above "Nothing
 * open yet": 0.5.0 showed it whenever no session was known.
 */
import { describe, expect, it } from "vitest";
import { buildLines } from "./rail-lines";
import { railEmptyWords } from "./rail-empty";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { KnownPlaceInfo } from "./ipc/places";

const NOW = new Date(2026, 10, 23, 12, 0, 0).getTime();

function known(instance: string, over: Partial<KnownPlaceInfo> = {}): KnownPlaceInfo {
  return {
    id: `app:claude-app:/Users/you/Library/Application Support/ClaudeProfiles/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`,
    app_pid: null, lost: false, keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 60_000, cannot_start: null, recent: [], found: false, ...over,
  };
}

const lines = (places: KnownPlaceInfo[], group: "place" | "account" = "place", offers = false) =>
  buildLines({
    items: [], view: { ...DEFAULT_VIEW, group }, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places,
    offers: offers ? [{ id: "app:claude-app:/Users/you/Library/Application Support/ClaudeProfiles/lab", label: "Claude app · lab", running: true }] : [],
  }).lines;

describe("AJ-E2327 the rail's empty state", () => {
  it("never shows under Open now: three running Apps with no session known", () => {
    const running = ["work", "team-a", "lab"].map((name, index) => known(name, { app_pid: 100 + index }));
    const out = lines(running);
    expect(out.some((line) => line.kind === "section" && line.id === "open" && line.count === 3)).toBe(true);
    // 0.5.0's rule (no session known: `items.length === 0`) said "Nothing
    // open yet" here.
    expect(railEmptyWords("place", out)).toBeNull();
  });

  it("shows when nothing is open, worded for its lens", () => {
    expect(railEmptyWords("place", lines([known("work")]))?.title).toBe("No apps open yet");
    expect(railEmptyWords("account", lines([], "account"))?.title).toBe("Nothing open on your accounts yet");
    expect(railEmptyWords("pool", lines([]))?.title).toBe("Nothing open in your pools yet");
  });

  it("points at Running, not added when an app there can be added", () => {
    const words = railEmptyWords("place", lines([], "place", true));
    expect(words?.body).toBe("Add a running app below to see its sessions here.");
  });
});
