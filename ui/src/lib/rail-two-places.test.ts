/**
 * AJ-E2326 R4 and R5 (the maintainer, 2026-11-22): a conversation AI
 * Juggler continued in its terminal, then resumed by the Claude app
 * profile's own Code child too. Two live writers, and the rail said
 * nothing: the profile's App read "nothing open" while its child held the
 * conversation (the list let AI Juggler's running session stand for it),
 * and the session read as one more CLI run.
 *
 * Now every live place is shown (R5: the conversation's row under the
 * desktop App whose Code child holds it), and both rows say "Open in two
 * places" with the other place, and carry AI Juggler's session to stop (R4).
 */
import { describe, expect, it } from "vitest";
import { buildLines, type Line } from "./rail-lines";
import { itemsOf, needsYou, OWN_APP } from "./rail-items";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import { claudeApp, NOW, open } from "./rail-matrix.test-kit";
import { instance } from "../test/fixtures";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";

const PROFILE = claudeApp("aij-test", 52_844);
/** AI Juggler's own terminal's `claude` (its chain ends at AI Juggler). */
const OURS: PlaceInfo = { app: "AI Juggler", shown: "AI Juggler", app_id: null, profile: null, instance: null, pid: 610, kind: null, app_pid: 42, label: "AI Juggler" };
const CHILD: PlaceInfo = { ...PROFILE.place, pid: 777 };
const RUN = instance({ id: "sess-echo", status: "running", conversation_id: "echo", account_id: PROFILE.folder, account_label: "aij-test", started_at_ms: NOW - 120_000, last_activity_ms: NOW - 60_000 });

function rail(conversations: ConversationInfo[], sessions = [RUN]) {
  const items = itemsOf({ sessions, conversations, projects: [], seen: {} });
  const lines = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: [PROFILE.desktop!] }).lines;
  const header = lines.find((line): line is Extract<Line, { kind: "header" }> => line.kind === "header" && line.group.key === PROFILE.key);
  return { items, header };
}

const held = (places: PlaceInfo[], over: Partial<ConversationInfo> = {}) => open("echo", PROFILE, { session_id: "sess-echo", places, pid: places[0]?.pid ?? null, title: "Echo lab", ...over });

describe("a conversation AI Juggler runs that the Claude app's Code child opens too", () => {
  it("shows under the profile's App too (never 'nothing open'), and both rows say it's open in two places", () => {
    const { items, header } = rail([held([OURS, CHILD])]);
    // R5: the profile's App has the conversation, so it isn't "nothing open".
    expect(header?.group.items.map((item) => item.key)).toEqual([`c:${PROFILE.folder}/echo`]);
    const mirrored = header!.group.items[0]!;
    expect(mirrored.live).toBe(true);
    // R4: both rows: the other place named, and AI Juggler's session to stop.
    expect(mirrored.twoPlaces).toEqual({ ours: RUN, alsoIn: OWN_APP });
    const run = items.find((item) => item.key === "s:sess-echo")!;
    expect(run.twoPlaces).toEqual({ ours: RUN, alsoIn: CHILD.label });
  });

  it("whichever place the list names first", () => {
    const { items, header } = rail([held([CHILD, OURS])]);
    expect(header?.group.items.map((item) => item.twoPlaces?.alsoIn)).toEqual([OWN_APP]);
    expect(items.find((item) => item.key === "s:sess-echo")?.twoPlaces?.alsoIn).toBe(CHILD.label);
  });

  it("held by the Code child alone, it is under the profile, with no two-places note", () => {
    const { items, header } = rail([held([CHILD])]);
    expect(header?.group.items.map((item) => item.key)).toEqual([`c:${PROFILE.folder}/echo`]);
    expect(header!.group.items[0]!.twoPlaces ?? null).toBeNull();
    expect(items.find((item) => item.key === "s:sess-echo")?.twoPlaces ?? null).toBeNull();
  });

  it("never asks twice in Needs you: the session's row carries its need", () => {
    const asking = { ...RUN, status: "waiting_user", needs_attention: "waiting_user" } as typeof RUN;
    const { items } = rail([held([OURS, CHILD], { state: "asks_you" })], [asking]);
    expect(needsYou(items).map((item) => item.key)).toEqual(["s:sess-echo"]);
  });
});

describe("a conversation only AI Juggler's session runs", () => {
  it("stays one row, its session's, with no note", () => {
    const { items, header } = rail([held([OURS])]);
    expect(header?.group.items ?? []).toEqual([]);
    expect(items.map((item) => item.key)).toEqual(["s:sess-echo"]);
    expect(items[0]!.twoPlaces ?? null).toBeNull();
  });

  it("a desktop session AI Juggler started in the app is not shown twice", () => {
    const desktop = { ...RUN, kind: "desktop" as const };
    const { items } = rail([held([CHILD])], [desktop]);
    expect(items.map((item) => item.key)).toEqual(["s:sess-echo"]);
  });
});
