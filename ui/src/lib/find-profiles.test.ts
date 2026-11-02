/**
 * D-62 (AJ-E3203): the ask shows only before an answer, only for the app
 * whose profiles sit in the folder it names; a yes says what it found.
 */
import { describe, expect, it } from "vitest";
import { askFolder, foundProfiles, foundWords } from "./find-profiles";
import type { KnownPlaceInfo } from "./ipc/places";

const folders = [{ app_id: "claude-app", folder: "~/Library/Application Support/ClaudeProfiles" }];

const place = (instance: string, over: Partial<KnownPlaceInfo> = {}): KnownPlaceInfo => ({
  id: `app:claude-app:/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null,
  keep_alive: false, paused: null, account_id: null, seen_ms: 0, cannot_start: null, recent: [], found: false, ...over,
});

describe("AJ-E3203 finding a desktop app's other profiles", () => {
  it("asks only before an answer, for the app whose folder it names", () => {
    expect(askFolder({ answer: "ask", folders }, "claude-app")).toBe("~/Library/Application Support/ClaudeProfiles");
    expect(askFolder({ answer: "ask", folders }, "codex-app")).toBeNull();
    expect(askFolder({ answer: "ask", folders: [] }, "claude-app")).toBeNull();
    expect(askFolder({ answer: "on", folders }, "claude-app")).toBeNull();
    expect(askFolder({ answer: "off", folders }, "claude-app")).toBeNull();
    expect(askFolder(null, "claude-app")).toBeNull();
  });

  it("a yes says which profiles it found, by their folder's name", () => {
    const places = [place("b-work"), place("b-play", { found: true }), place("b-side", { found: true }), place("x", { found: true, app_id: "other" })];
    const names = foundProfiles(places, "claude-app");
    expect(names).toEqual(["b-play", "b-side"]);
    expect(foundWords(names)).toBe("Found 2 more profiles: b-play, b-side. Each is an app of its own now, with Start.");
    expect(foundWords(["a"])).toBe("Found 1 more profile: a. Each is an app of its own now, with Start.");
    expect(foundWords(["a", "b", "c", "d", "e", "f"])).toBe("Found 6 more profiles: a, b, c and 3 more. Each is an app of its own now, with Start.");
    expect(foundWords([])).toContain("No other profiles there");
  });
});
