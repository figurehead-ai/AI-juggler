/**
 * The 0.4.6 rc2 walk (main: "Open now must show what is actually open,
 * where it's open"): a Claude app conversation continued in AI Juggler's
 * terminal, stopped there, then opened again in its Claude app profile.
 * The backend's list (`conversations::commands::stands_for`) now gives the
 * open conversation its own row in the profile's App while the app runs
 * it, and leaves the stopped run as it was; the rail and the App page
 * render exactly that: open under the profile, the run ended under its
 * CLI App, never only the ended run.
 */
import { describe, expect, it } from "vitest";
import { buildLines, type Line } from "./rail-lines";
import { itemsOf, openNow } from "./rail-items";
import { appItems, openHere } from "./app-items";
import { ownersOf } from "./rail-apps";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import { claudeApp, cliApp, NOW, open, past } from "./rail-matrix.test-kit";
import { instance } from "../test/fixtures";
import type { ConversationInfo } from "./ipc/conversations";

const PROFILE = claudeApp("aij-test", 52_844);
const CLI = cliApp("claude-code", PROFILE.folder, "aij-test");
const RUN = instance({ id: "sess-echo", status: "stopped", conversation_id: "echo", account_id: PROFILE.folder, account_label: "aij-test", started_at_ms: NOW - 120_000, last_activity_ms: NOW - 60_000 });

function rail(conversations: ConversationInfo[]) {
  const items = itemsOf({ sessions: [RUN], conversations, projects: [], seen: {} });
  const known = [PROFILE.desktop!];
  const lines = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known }).lines;
  const header = lines.find((line): line is Extract<Line, { kind: "header" }> => line.kind === "header" && line.group.key === PROFILE.key);
  const { byConversation, byFolder } = ownersOf(known);
  const page = appItems(items, PROFILE.key, byConversation, byFolder);
  return { items, header, page };
}

describe("a conversation the Claude app runs after AI Juggler's terminal run of it ended", () => {
  it("is open under the profile, in Open now and on the App page; the run stays ended under its CLI App", () => {
    // The same conversation: the list no longer lets the ended run stand for it.
    const { items, header, page } = rail([open("echo", PROFILE, { title: "Echo lab" })]);
    expect(header?.group.items.map((item) => item.key)).toEqual([`c:${PROFILE.folder}/echo`]);
    expect(page.filter(openHere).map((item) => item.title)).toEqual(["Echo lab"]);
    const run = items.find((item) => item.key === "s:sess-echo");
    expect(run && !openNow(run) && run.place.key).toBe(CLI.key);
  });

  it("the app's copy under its own id is open under the profile too", () => {
    const { header, page } = rail([past("echo", PROFILE, 0.1, { session_id: "sess-echo" }), open("copy", PROFILE, { title: "Echo lab" })]);
    expect(header?.group.items.map((item) => item.key)).toEqual([`c:${PROFILE.folder}/copy`]);
    expect(page.filter(openHere).map((item) => item.title)).toEqual(["Echo lab"]);
  });

  it("before the app opens it, only the ended run, and the profile has nothing open", () => {
    const { header, page } = rail([past("echo", PROFILE, 0.1, { session_id: "sess-echo" })]);
    expect(header?.group.items ?? []).toEqual([]);
    expect(page.filter(openHere)).toEqual([]);
  });
});
