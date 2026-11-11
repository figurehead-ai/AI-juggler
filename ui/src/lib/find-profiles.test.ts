/**
 * D-62 (AJ-E3203): the ask shows only before an answer, only for the app
 * whose profiles sit in the folder it names (what a yes offers: find-profiles.test.tsx).
 */
import { describe, expect, it } from "vitest";
import { askFolder } from "./find-profiles";
const folders = [{ app_id: "claude-app", folder: "~/Library/Application Support/ClaudeProfiles" }];

describe("AJ-E3203 finding a desktop app's other profiles", () => {
  it("asks only before an answer, for the app whose folder it names", () => {
    expect(askFolder({ answer: "ask", folders }, "claude-app")).toBe("~/Library/Application Support/ClaudeProfiles");
    expect(askFolder({ answer: "ask", folders }, "codex-app")).toBeNull();
    expect(askFolder({ answer: "ask", folders: [] }, "claude-app")).toBeNull();
    expect(askFolder({ answer: "on", folders }, "claude-app")).toBeNull();
    expect(askFolder({ answer: "off", folders }, "claude-app")).toBeNull();
    expect(askFolder(null, "claude-app")).toBeNull();
  });

});
