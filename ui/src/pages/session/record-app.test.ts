/**
 * The go sitting (2026-11-07): a closed Claude app profile's session wore
 * "Claude Code · aij-test", its folder's CLI App, though its page lists it
 * under "Claude app · aij-test". A conversation in a profile's own records
 * is that profile's, for the copy in the profile's folder only (a switch
 * copies a conversation into another folder under the same id).
 */
import { describe, expect, it } from "vitest";
import type { KnownPlaceInfo } from "../../lib/ipc/places";
import { appIdOfSession, recordPlaceOf } from "./AccountMenu";
import type { SessionModel } from "./model";

const place: KnownPlaceInfo = {
  id: "app:claude-app:/p/aij-test", app_id: "claude-app", app: "Claude", instance: "aij-test", profile: "aij-test", label: "Claude · aij-test",
  app_pid: null, lost: false, keep_alive: false, paused: null, account_id: "folder-aij", seen_ms: 0, cannot_start: null, recent: ["claude-code/s-1"], found: false,
};
type Facts = Pick<SessionModel, "tool" | "profileId" | "conversationId" | "sessionId" | "places" | "desktop" | "profileLabel">;
const model = (over: Partial<Facts>): Facts => ({ tool: "claude-code", profileId: "folder-aij", conversationId: "s-1", sessionId: null, places: [], desktop: false, profileLabel: "aij-test", ...over });

describe("a closed profile's session is its App's", () => {
  it("in the profile's own folder", () => {
    expect(recordPlaceOf(model({}), [place])?.id).toBe(place.id);
    expect(appIdOfSession(model({}), [place])).toBe(place.id);
  });

  it("never a copy in another folder, nor one AI Juggler runs", () => {
    expect(recordPlaceOf(model({ profileId: "folder-other" }), [place])).toBeUndefined();
    expect(appIdOfSession(model({ profileId: "folder-other" }), [place])).toBe("cli:folder-other");
    expect(recordPlaceOf(model({ sessionId: "sess-1" }), [place])).toBeUndefined();
  });
});
