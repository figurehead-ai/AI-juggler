/** 0.4.1 (finding #1): a closed profile's rail header finds its app by the
 * group's key, not only by a running pid, and its Open starts it. */
import { afterEach, describe, expect, it, vi } from "vitest";
import * as places from "../lib/ipc/places";
import { groupApp, groupClosed, openGroupApp } from "./rail-group-app";
import { knownKey } from "../lib/rail-apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { RailGroup } from "../lib/rail-places";
import type { RailPlace } from "../lib/rail-items";
import { resetToasts, toastEntries } from "./Toast";

afterEach(() => {
  vi.restoreAllMocks();
  resetToasts();
});

const south: KnownPlaceInfo = {
  recent: [],
  id: "app:claude-app:/Users/you/ClaudeProfiles/b-south",
  app_id: "claude-app",
  app: "Claude",
  instance: "b-south",
  profile: "b-south",
  label: "Claude · b-south",
  app_pid: null,
  lost: false,
  keep_alive: false,
  paused: null,
  account_id: "claude-code-1",
  seen_ms: 1,
  cannot_start: null,
  found: false,
};

const desktop: RailPlace = { key: knownKey(south), app: "Claude app", appId: "claude-app", kind: "desktop", profile: "b-south", appPid: null, host: null };
const group = (place: RailPlace = desktop): RailGroup => ({ key: place.key, label: "Claude app · b-south", place, accountId: null, poolId: null, items: [] });

describe("a closed desktop app's rail header", () => {
  it("finds its app by the group's key while it's closed", () => {
    expect(groupApp("place", group(), [south])?.id).toBe(south.id);
    expect(groupApp("account", group(), [south])).toBeNull();
    expect(groupApp("place", group({ ...desktop, kind: "terminal" }), [south])).toBeNull();
  });

  it("Open starts it, and says so", async () => {
    const open = vi.spyOn(places, "placesOpen").mockResolvedValue({ outcome: "Starting", reached: "started" });
    openGroupApp(south);
    expect(open).toHaveBeenCalledWith(south.id);
    expect(toastEntries().map((toast) => toast.options.message)).toContain("Starting Claude · b-south…");
  });
});

describe("whether the header starts or brings forward", () => {
  it("starts only when neither the app nor any of its rows runs", () => {
    expect(groupClosed(south, group())).toBe(true);
    expect(groupClosed({ ...south, app_pid: 9 }, group())).toBe(false);
    expect(groupClosed(south, group({ ...desktop, appPid: 9 }))).toBe(false);
    expect(groupClosed(null, group())).toBe(false);
  });
});
