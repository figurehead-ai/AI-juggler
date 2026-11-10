/**
 * AJ-E3207 — New session's usual choice, pinned case by case (the L3
 * pairs' history × room × folder rows; N1, N4, N5): the folder it opens
 * on, the App and its why, out of room said and offered (a guess gives way,
 * the user's own never switched: D-31 as amended by D-97), and the pickers' top groups.
 */
import { describe, expect, it } from "vitest";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { LOW_LEFT } from "./room";
import { appGroups, appWhyWords, appsByRoom, chooseApp, chooseFolder, folderGroups, folderWhyWords, nextFolder, passedOverWords, poolProject, skippedWords, type Room, type UsedSession } from "./new-session-choice";

const T0 = 1_800_000_000_000;

const recipe = (account: string, label: string, name = "Main", extra: Partial<LaunchRecipeInfo> = {}): LaunchRecipeInfo => ({
  id: `r-${account}-${name}`,
  name,
  account_id: account,
  account_label: label,
  provider: "tool-a",
  model: null,
  default_cwd: null,
  restart_on_crash: null,
  ...extra,
});

const folder = (id: string, extra: Partial<AccountInfo> = {}): AccountInfo =>
  ({ id, provider: "tool-a", label: id, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false, ...extra }) as AccountInfo;

const project = (id: string, extra: Partial<ProjectInfo> = {}): ProjectInfo => ({
  id,
  root: `/Users/you/Dev/${id}`,
  name: id,
  last_activity_ms: null,
  active_instance: null,
  accounts_used: [],
  instance_count: 0,
  history_sessions: 0,
  history_last_used_ms: null,
  repo: null,
  missing: false,
  ...extra,
});

const WORK = recipe("work", "Work");
const HOME = recipe("home", "Home");
const SIDE = recipe("side", "Side");
const RECIPES = [WORK, HOME, SIDE];
const FOLDERS = [folder("work"), folder("home"), folder("side")];

/** Room by folder: `left` % (null: not measured). */
const roomOf =
  (left: Record<string, number | null>) =>
  (id: string): Room => ({ left: left[id] ?? null, resetsAtMs: T0 + 3_600_000, credits: false });

const base = { recipes: RECIPES, folders: FOLDERS, roomOf: roomOf({ work: 60, home: 90, side: 40 }) };

describe("the folder", () => {
  const web = project("web-shop", { last_activity_ms: T0 - 60_000 });
  const api = project("api", { history_last_used_ms: T0 - 1_000 });
  const gone = project("gone", { last_activity_ms: T0, missing: true });
  const hidden = project("hidden", { last_activity_ms: T0, forgotten: true });

  it("is the one it was opened for, project or not", () => {
    expect(chooseFolder([web], "/Users/you/Dev/web-shop")).toEqual({ root: web.root, project: web, why: "asked" });
    expect(chooseFolder([web], "/tmp/dropped")).toEqual({ root: "/tmp/dropped", project: null, why: "asked" });
  });

  it("else the project worked in last, here or in the tool's own history; never a moved or forgotten one", () => {
    expect(chooseFolder([web, api, gone, hidden], null)).toMatchObject({ root: api.root, why: "latest" });
  });

  // Walk N4 (rc26): a newcomer's project has no time (adding a folder
  // isn't work in it, 1e51970b0), and New session opened on a temporary
  // folder. A project of theirs is the better first folder; no time claimed.
  it("with no time on any, the project the tool's history used, said as used here before", () => {
    const quiet = project("quiet");
    const used = project("web", { history_sessions: 2, accounts_used: ["Work"] });
    expect(chooseFolder([quiet, used, { ...used, id: "gone", missing: true, history_sessions: 9 }], null)).toMatchObject({ root: used.root, why: "used-before" });
    expect(folderWhyWords("used-before")).toBe("used here before");
  });

  it("else the only project the user added, said as theirs; of several, the pinned one, then by name", () => {
    expect(chooseFolder([project("web")], null)).toMatchObject({ root: "/Users/you/Dev/web", why: "yours" });
    expect(folderWhyWords("yours")).toBe("your project");
    expect(chooseFolder([project("web"), project("api")], null)).toMatchObject({ root: "/Users/you/Dev/api", why: "one-of-yours" });
    expect(chooseFolder([project("web", { pinned: true }), project("api")], null)).toMatchObject({ root: "/Users/you/Dev/web", why: "one-of-yours" });
    expect(folderWhyWords("one-of-yours")).toBe("one of your projects");
  });

  it("else none: no project, or only moved or forgotten ones", () => {
    expect(chooseFolder([], null)).toBeNull();
    expect(chooseFolder([project("gone", { missing: true }), project("hidden", { forgotten: true })], null)).toBeNull();
  });
});

describe("the App (N1, N4)", () => {
  it("is the one chosen, before every default", () => {
    const here = project("p", { default_account_id: "home", accounts_used: ["Side"] });
    expect(chooseApp({ ...base, project: here, chosen: "work" })).toMatchObject({ recipe: WORK, why: "chosen" });
  });

  it("then this project's default, then yours, then one used here before", () => {
    const here = project("p", { default_account_id: "home", accounts_used: ["Side"] });
    expect(chooseApp({ ...base, project: here })).toMatchObject({ recipe: HOME, why: "project-default" });
    const yours = [folder("work"), folder("home", { is_default: true }), folder("side")];
    expect(chooseApp({ ...base, folders: yours, project: project("p", { accounts_used: ["Side"] }) })).toMatchObject({ recipe: HOME, why: "your-default" });
    // The history names who worked here, not who was last: said as such.
    expect(chooseApp({ ...base, project: project("p", { accounts_used: ["Work", "Side"] }) })).toMatchObject({ recipe: SIDE, why: "used-here" });
  });

  it("a project default whose App is gone gives way to the next reason, never to nothing", () => {
    const stale = project("p", { default_account_id: "removed", accounts_used: ["Side"] });
    expect(chooseApp({ ...base, project: stale })).toMatchObject({ recipe: SIDE, why: "used-here" });
  });

  it("the rc12 walk: the last used here is AI Juggler's newest session there, not the history's order", () => {
    const here = project("p", { accounts_used: ["Side", "Work"] });
    const sessions: UsedSession[] = [
      { account_id: "side", project_id: "p", started_at_ms: 2 },
      { account_id: "work", project_id: "other", started_at_ms: 3 },
    ];
    // The history's order says Work; the newest session here was Side.
    expect(chooseApp({ ...base, project: here, sessions })).toMatchObject({ recipe: SIDE, why: "last-here" });
    // None here: the history says who worked here, never who was last.
    expect(chooseApp({ ...base, project: here, sessions: [] })).toMatchObject({ recipe: WORK, why: "used-here" });
  });

  it("walk K4: the reason is the folder's own, and a history's names are never said as the last used", () => {
    // The backend sorts a project's `accounts_used` by name: its last name
    // was said as "the last you used here" on every folder that held it.
    const web = project("web", { accounts_used: ["Home", "Side"] });
    const data = project("data", { accounts_used: ["Home", "Side"] });
    const sessions: UsedSession[] = [{ account_id: "home", project_id: "web", started_at_ms: 5 }];
    const onWeb = chooseApp({ ...base, project: web, sessions })!;
    expect([onWeb.recipe, appWhyWords(onWeb.why)]).toEqual([HOME, "the last you used here"]);
    // Changed to data, where AI Juggler never ran: the reason is data's.
    const onData = chooseApp({ ...base, project: data, sessions })!;
    expect([onData.recipe, appWhyWords(onData.why)]).toEqual([SIDE, "used here before"]);
  });

  it("with no history here: the App used last anywhere", () => {
    const sessions = [
      { account_id: "side", started_at_ms: T0 - 5_000 },
      { account_id: "work", started_at_ms: T0 - 1_000 },
    ] as InstanceInfo[];
    expect(chooseApp({ ...base, project: project("new"), sessions })).toMatchObject({ recipe: WORK, why: "last-anywhere" });
  });

  it("with no history at all: the most room, said as the best guess (N4)", () => {
    const choice = chooseApp({ ...base, project: null });
    expect(choice).toMatchObject({ recipe: HOME, why: "most-room", kind: "room", roomier: null });
    expect(appWhyWords(choice!.why)).toBe("the most room left");
  });

  it("with no room measured: the only App, else the first that can start", () => {
    const unknown = roomOf({});
    expect(chooseApp({ recipes: [WORK], folders: FOLDERS, roomOf: unknown, project: null })).toMatchObject({ recipe: WORK, why: "only" });
    const signedOut = [folder("work", { login_state: "logged_out" }), folder("home"), folder("side")];
    expect(chooseApp({ recipes: RECIPES, folders: signedOut, roomOf: unknown, project: null })).toMatchObject({ recipe: HOME, why: "first" });
    // Walk N4: a guess says why it's one.
    expect(appWhyWords("first")).toBe("a first guess: no app has run a session yet");
  });

  it("is one per App: its first launch settings", () => {
    const deep = recipe("work", "Work", "Deep work");
    const quick = recipe("work", "Work", "Quick fixes");
    expect(chooseApp({ ...base, recipes: [deep, quick], project: null })?.recipe).toBe(deep);
  });

  it("is none without any App", () => {
    expect(chooseApp({ ...base, recipes: [], project: null })).toBeNull();
  });
});

describe("out of room or low (N5, D-31 as amended by D-97)", () => {
  const here = project("p", { accounts_used: ["Work"] });

  it("D-97: the usual App out of room gives way to the one with the most room, and names it", () => {
    const choice = chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, home: 90, side: 40 }) });
    expect(choice).toMatchObject({ recipe: HOME, why: "most-room", kind: "room", roomier: null, skipped: WORK });
  });

  it("never gives way to an App on the same Account: they share its room", () => {
    const account = (id: string) => (id === "home" ? "acct-1" : id === "work" ? "acct-1" : "acct-2");
    const choice = chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, home: 90, side: 70 }), accountOf: account });
    expect(choice).toMatchObject({ recipe: SIDE, why: "most-room", skipped: WORK });
  });

  it("with the others unmeasured, it stays, warned, and offers one of them as such (walk N5)", () => {
    const choice = chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0 }) });
    expect(choice).toMatchObject({ recipe: WORK, why: "used-here", kind: "out", roomier: HOME, skipped: null });
    // A measured one with room: it gives way to that one.
    expect(chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, side: 70 }) })).toMatchObject({ recipe: SIDE, skipped: WORK });
  });

  it("D-97: the usual App on a Low Account gives way too, at the Low mark's own line; plenty stays", () => {
    const low = chooseApp({ ...base, project: here, roomOf: roomOf({ work: LOW_LEFT, home: 90 }) })!;
    expect(low).toMatchObject({ recipe: HOME, why: "most-room", kind: "room", skipped: WORK });
    expect(skippedWords("Work", { left: LOW_LEFT, resetsAtMs: null, credits: false })).toBe(`Work is almost out (${LOW_LEFT}% left)`);
    // Just over the line it has room: it stays, said as before.
    expect(chooseApp({ ...base, project: here, roomOf: roomOf({ work: LOW_LEFT + 1, home: 90 }) })).toMatchObject({ recipe: WORK, why: "used-here", kind: "room", skipped: null });
    expect(chooseApp({ ...base, project: here })).toMatchObject({ recipe: WORK, kind: "room", roomier: null });
    // The App used last anywhere is a guess as well.
    const last: UsedSession[] = [{ account_id: "work", project_id: null, started_at_ms: T0 }];
    expect(chooseApp({ ...base, project: null, sessions: last, roomOf: roomOf({ work: 5, home: 90 }) })).toMatchObject({ recipe: HOME, skipped: WORK });
  });

  it("D-97: with no other App that has room, the usual Low one stays, warned, and nothing is skipped", () => {
    expect(chooseApp({ ...base, project: here, roomOf: roomOf({ work: 5, home: 3, side: 0 }) })).toMatchObject({ recipe: WORK, why: "used-here", kind: "low", roomier: null, skipped: null });
  });

  // "Always use for web" (PRINCIPLES' worked example): the default the
  // user set still gives way to the warning, never to a switch.
  it("the project's default out of room: kept, said as your default, the one with room offered", () => {
    const fixed = project("p", { default_account_id: "work", accounts_used: ["Side"] });
    const choice = chooseApp({ ...base, project: fixed, roomOf: roomOf({ work: 0, home: 90, side: 40 }) })!;
    expect(choice).toMatchObject({ recipe: WORK, why: "project-default", kind: "out", roomier: HOME });
    expect(appWhyWords(choice.why)).toBe("your default here");
  });

  // The rc27 walk: the first default was an App that isn't signed in
  // (Claude Code · aij-test, "used here before"), shown with Sign in first.
  // A guess from history that can't start gives way to one that can, and
  // the why names it; the user's own choice still stands, said and offered.
  it("not signed in, only guessed from history: passed over for one that can start, and said", () => {
    const signedOut = [folder("work", { login_state: "logged_out" }), folder("home"), folder("side")];
    const choice = chooseApp({ ...base, folders: signedOut, project: here })!;
    expect(choice).toMatchObject({ recipe: HOME, why: "most-room", kind: "room", passedOver: WORK });
    expect(passedOverWords("Claude Code · Work")).toBe("Claude Code · Work isn't signed in");
    // The App used last anywhere that can't start is never the guess either.
    const last: UsedSession[] = [{ account_id: "work", project_id: null, started_at_ms: T0 }];
    expect(chooseApp({ ...base, folders: signedOut, project: null, sessions: last })).toMatchObject({ recipe: HOME, why: "most-room" });
  });

  it("not signed in, the user's own choice: kept, said, and another offered", () => {
    const signedOut = [folder("work", { login_state: "logged_out" }), folder("home"), folder("side")];
    const fixed = project("p", { default_account_id: "work" });
    expect(chooseApp({ ...base, folders: signedOut, project: fixed })).toMatchObject({ recipe: WORK, why: "project-default", kind: "blocked", roomier: HOME, passedOver: null });
    expect(chooseApp({ ...base, folders: signedOut, project: here, chosen: "work" })).toMatchObject({ recipe: WORK, why: "chosen", kind: "blocked" });
  });

  it("nothing can start: the usual one stays, said", () => {
    const none = [folder("work", { login_state: "logged_out" }), folder("home", { login_state: "logged_out" }), folder("side", { login_state: "logged_out" })];
    expect(chooseApp({ ...base, folders: none, project: here })).toMatchObject({ recipe: WORK, why: "used-here", kind: "blocked", passedOver: null });
  });

  it("with nothing that has room, offers nothing", () => {
    expect(chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, home: 0, side: 0 }) })).toMatchObject({ kind: "out", roomier: null });
  });

  it("a window used up that its credits carry on is not out of room", () => {
    const credits = (id: string): Room => ({ left: 0, resetsAtMs: null, credits: id === "work" });
    expect(chooseApp({ ...base, project: here, roomOf: credits })).toMatchObject({ recipe: WORK, kind: "credits", roomier: null });
  });
});

// D-96 E (persona D5, Devon): the right App was picked, but only the
// percentages said why the usual one wasn't.
describe("a skip says why (D-96 E)", () => {
  it("room picked over the App it would have opened on, almost out: that one is named, in the Low mark's words", () => {
    const choice = chooseApp({ ...base, project: null, roomOf: roomOf({ work: 5, home: 90, side: 40 }) })!;
    expect(choice).toMatchObject({ recipe: HOME, why: "most-room", kind: "room", skipped: WORK });
    expect(skippedWords("Work", { left: 5, resetsAtMs: null, credits: false })).toBe("Work is almost out (5% left)");
  });

  it("out of room: said with its reset, as the Out mark says it", () => {
    const choice = chooseApp({ ...base, project: null, roomOf: roomOf({ work: 0, home: 90 }) })!;
    expect(choice).toMatchObject({ recipe: HOME, skipped: WORK });
    expect(skippedWords("Work", { left: 0, resetsAtMs: null, credits: false })).toBe("Work is out of room for now");
    expect(skippedWords("Work", { left: 0, resetsAtMs: T0 + 3_600_000, credits: false }, T0)).toMatch(/^Work is out of room until \S/);
  });

  it("an estimate says about, as the room mark does: the text claims what it measures", () => {
    expect(skippedWords("Work", { left: 5, resetsAtMs: null, credits: false }, T0, true)).toBe("Work is almost out (about 5% left)");
    expect(skippedWords("Work", { left: 0, resetsAtMs: T0 + 3_600_000, credits: false }, T0, true)).toMatch(/^Work is out of room until about \S/);
    expect(skippedWords("Work", { left: 0, resetsAtMs: null, credits: false }, T0, true)).toBe("Work is out of room for now");
  });

  it("no skip when the App passed over still has room, has credits, or is the one picked", () => {
    expect(chooseApp({ ...base, project: null })).toMatchObject({ recipe: HOME, skipped: null });
    const credits = (id: string): Room => ({ left: id === "work" ? 0 : 90, resetsAtMs: null, credits: id === "work" });
    expect(chooseApp({ ...base, project: null, roomOf: credits })).toMatchObject({ recipe: HOME, skipped: null });
    expect(chooseApp({ ...base, project: null, roomOf: roomOf({ work: 90, home: 5 }) })).toMatchObject({ recipe: WORK, skipped: null });
    expect(skippedWords("Work", { left: 60, resetsAtMs: null, credits: false })).toBeNull();
  });

  it("D-97: the user's own on a Low Account stands, warned, the other offered; never a skip", () => {
    const here = project("p", { accounts_used: ["Side"] });
    const low = roomOf({ work: 5, home: 90, side: 60 });
    // Picked in the sheet, or by the door that opened it.
    expect(chooseApp({ ...base, project: here, chosen: "work", roomOf: low })).toMatchObject({ recipe: WORK, why: "chosen", kind: "low", roomier: HOME, skipped: null });
    // "Always use for p".
    const fixed = project("p", { default_account_id: "work" });
    expect(chooseApp({ ...base, project: fixed, roomOf: low })).toMatchObject({ recipe: WORK, why: "project-default", kind: "low", roomier: HOME, skipped: null });
    // Kept from Start another.
    expect(chooseApp({ ...base, project: here, kept: "work", roomOf: low })).toMatchObject({ recipe: WORK, why: "kept", kind: "low", roomier: HOME, skipped: null });
    // The user's own default.
    const yours = [folder("work", { is_default: true }), folder("home"), folder("side")];
    expect(chooseApp({ ...base, folders: yours, project: null, roomOf: low })).toMatchObject({ recipe: WORK, why: "your-default", kind: "low", roomier: HOME, skipped: null });
    // Out of room, the same.
    expect(chooseApp({ ...base, project: fixed, roomOf: roomOf({ work: 0, home: 90 }) })).toMatchObject({ recipe: WORK, why: "project-default", kind: "out", roomier: HOME, skipped: null });
  });
});

// D-96 F: "Start another: <next project>" after a start.
describe("start another (D-96 F)", () => {
  const billing = project("billing", { last_activity_ms: T0 - 1_000 });
  const shop = project("shop", { last_activity_ms: T0 - 60_000 });
  const docs = project("docs", { history_last_used_ms: T0 - 3_600_000 });
  const quiet = project("quiet");

  it("the next is the next project worked in, by the order New session opens on, never one started in this run", () => {
    const all = [quiet, docs, shop, billing];
    expect(nextFolder(all, [billing.root])).toMatchObject({ root: shop.root, why: "latest" });
    expect(nextFolder(all, [billing.root, shop.root])).toMatchObject({ root: docs.root });
    // The tool's history alone counts as work there.
    const used = project("used", { history_sessions: 3 });
    expect(nextFolder([billing, used, quiet], [billing.root])).toMatchObject({ root: used.root, why: "used-before" });
  });

  it("none when only a guess is left (no work in it), or a moved or forgotten one", () => {
    expect(nextFolder([billing, quiet], [billing.root])).toBeNull();
    expect(nextFolder([billing, { ...shop, missing: true }, { ...docs, forgotten: true }], [billing.root])).toBeNull();
    expect(nextFolder([], [])).toBeNull();
  });

  it("the App the user chose before is kept, said so, after the project's own default and before the rest", () => {
    const choice = chooseApp({ ...base, project: project("shop", { accounts_used: ["Side"] }), kept: "home" })!;
    expect(choice).toMatchObject({ recipe: HOME, why: "kept" });
    expect(appWhyWords(choice.why, "billing")).toBe("as you chose for billing");
    const yours = [folder("work", { is_default: true }), folder("home"), folder("side")];
    expect(chooseApp({ ...base, folders: yours, project: null, kept: "home" })).toMatchObject({ recipe: HOME, why: "kept" });
    // "Always use for shop" still wins.
    expect(chooseApp({ ...base, project: project("shop", { default_account_id: "side" }), kept: "home" })).toMatchObject({ recipe: SIDE, why: "project-default" });
  });

  it("room is checked again: a kept App now out is kept, warned and the one with room offered (D-31)", () => {
    expect(chooseApp({ ...base, project: null, kept: "work", roomOf: roomOf({ work: 0, home: 90 }) })).toMatchObject({ recipe: WORK, why: "kept", kind: "out", roomier: HOME, skipped: null });
  });
});

describe("the pickers' top groups", () => {
  it("folders: Recent (five at most), then Pinned not already there", () => {
    const rows = Array.from({ length: 7 }, (_, at) => project(`p${at}`, { last_activity_ms: T0 - at * 1000 }));
    const pinned = project("pinned", { pinned: true });
    const groups = folderGroups([...rows, pinned, project("gone", { last_activity_ms: T0, missing: true })]);
    expect(groups).toEqual([
      { label: "Recent", values: ["p0", "p1", "p2", "p3", "p4"] },
      { label: "Pinned", values: ["pinned"] },
    ]);
  });

  it("apps: Suggested is the choice and the most room; then the ones used here, newest first", () => {
    const here = project("web-shop", { accounts_used: ["Side", "Work", "Home"] });
    const ranked = appsByRoom(base);
    const choice = chooseApp({ ...base, project: here, chosen: "work" });
    expect(appGroups(choice, ranked, here)).toEqual([
      { label: "Suggested", values: ["work", "home"] },
      { label: "Used in web-shop", values: ["side"] },
    ]);
  });
});

// The rc27 walk: New session from a pool's page opened on another folder.
// The page opens it in its project, the order's "the one it was opened for".
describe("a pool's page opens New session in its project", () => {
  it("a project's own pool, or a pool over one project; none for several or every project", () => {
    expect(poolProject({ project_id: "web", scope: ["web"], scope_all: false })).toBe("web");
    expect(poolProject({ project_id: null, scope: ["web"], scope_all: false })).toBe("web");
    expect(poolProject({ project_id: null, scope: ["web", "api"], scope_all: false })).toBeNull();
    expect(poolProject({ project_id: null, scope: [], scope_all: true })).toBeNull();
    expect(poolProject({ project_id: null, scope: [null], scope_all: false })).toBeNull();
    expect(poolProject(undefined)).toBeNull();
  });
});
