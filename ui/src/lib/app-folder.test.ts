/**
 * AJ-E2327 — an App's tool folder in words: Add's confirmation names the
 * folder brought in (where it is) and what told; no folder says why; the
 * calm grey says what it means.
 */
import { describe, expect, it } from "vitest";
import { addedNote, folderByWords, toneTip, unlinked } from "./app-folder";

const folder = { tool: "Claude Code", name: "client-a", path: "~/Library/Application Support/ClaudeCodeProfiles/client-a", by: "running", why: null };
const none = { tool: "Claude Code", name: null, path: null, by: null, why: "It isn't running and its app lists no sessions yet, so AI Juggler can't tell which Claude Code folder it uses." };

describe("AJ-E2327 an App's folder in words", () => {
  it("Add's confirmation names the folder brought in and what told", () => {
    const note = addedNote("Claude app · team-a", folder);
    expect(note.message).toBe("Added Claude app · team-a");
    expect(note.detail).toBe("Also uses its Claude Code folder client-a, where it is, so its sessions show here (its open sessions write there).");
    expect(note.unlinked).toBe(false);
  });

  it("no folder: says why, and what to do", () => {
    const note = addedNote("Claude app · team-a", none);
    expect(note.unlinked).toBe(true);
    expect(note.detail).toBe(`${none.why} Choose its folder to see its sessions.`);
    expect(unlinked(none)).toBe(true);
    expect(unlinked(folder)).toBe(false);
    expect(unlinked(null)).toBe(false);
  });

  it("a name guess says it's a guess", () => {
    expect(folderByWords("named")).toContain("a guess, checked when it runs");
  });

  it("the grey icon is explained: muted at rest, why when sessions can't show", () => {
    expect(toneTip(false, folder)).toContain("Muted while nothing here needs you");
    expect(toneTip(true, folder)).toBeNull();
    expect(toneTip(true, none)).toBe(`Its sessions can't be shown: ${none.why}`);
  });
});
