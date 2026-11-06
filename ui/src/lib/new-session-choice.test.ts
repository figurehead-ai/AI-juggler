/**
 * AJ-E3207 — New session's usual choice, pinned case by case (the L3
 * pairs' history × room × folder rows; N1, N4, N5): the folder it opens
 * on, the App and its why, out of room said and offered (never switched,
 * D-31), and the pickers' top groups.
 */
import { describe, expect, it } from "vitest";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { appGroups, appWhyWords, appsByRoom, chooseApp, chooseFolder, folderGroups, type Room } from "./new-session-choice";

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

  it("else none", () => {
    expect(chooseFolder([], null)).toBeNull();
    expect(chooseFolder([project("never")], null)).toBeNull();
  });
});

describe("the App (N1, N4)", () => {
  it("is the one chosen, before every default", () => {
    const here = project("p", { default_account_id: "home", accounts_used: ["Side"] });
    expect(chooseApp({ ...base, project: here, chosen: "work" })).toMatchObject({ recipe: WORK, why: "chosen" });
  });

  it("then this project's default, then yours, then the last used here", () => {
    const here = project("p", { default_account_id: "home", accounts_used: ["Side"] });
    expect(chooseApp({ ...base, project: here })).toMatchObject({ recipe: HOME, why: "project-default" });
    const yours = [folder("work"), folder("home", { is_default: true }), folder("side")];
    expect(chooseApp({ ...base, folders: yours, project: project("p", { accounts_used: ["Side"] }) })).toMatchObject({ recipe: HOME, why: "your-default" });
    expect(chooseApp({ ...base, project: project("p", { accounts_used: ["Work", "Side"] }) })).toMatchObject({ recipe: SIDE, why: "last-here" });
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

describe("out of room (N5, D-31)", () => {
  const here = project("p", { accounts_used: ["Work"] });

  it("keeps the usual App, says it's out, and offers the one with the most room", () => {
    const choice = chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, home: 90, side: 40 }) });
    expect(choice).toMatchObject({ recipe: WORK, why: "last-here", kind: "out", roomier: HOME });
  });

  it("never offers an App on the same Account: they share its room", () => {
    const account = (id: string) => (id === "home" ? "acct-1" : id === "work" ? "acct-1" : "acct-2");
    const choice = chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, home: 0, side: 70 }), accountOf: account });
    expect(choice).toMatchObject({ recipe: WORK, kind: "out", roomier: SIDE });
  });

  it("low room offers too; plenty offers nothing", () => {
    expect(chooseApp({ ...base, project: here, roomOf: roomOf({ work: 10, home: 90 }) })).toMatchObject({ recipe: WORK, kind: "low", roomier: HOME });
    expect(chooseApp({ ...base, project: here })).toMatchObject({ recipe: WORK, kind: "room", roomier: null });
  });

  it("not signed in: kept, said, and another offered", () => {
    const signedOut = [folder("work", { login_state: "logged_out" }), folder("home"), folder("side")];
    expect(chooseApp({ ...base, folders: signedOut, project: here })).toMatchObject({ recipe: WORK, kind: "blocked", roomier: HOME });
  });

  it("with nothing that has room, offers nothing", () => {
    expect(chooseApp({ ...base, project: here, roomOf: roomOf({ work: 0, home: 0, side: 0 }) })).toMatchObject({ kind: "out", roomier: null });
  });

  it("a window used up that its credits carry on is not out of room", () => {
    const credits = (id: string): Room => ({ left: 0, resetsAtMs: null, credits: id === "work" });
    expect(chooseApp({ ...base, project: here, roomOf: credits })).toMatchObject({ recipe: WORK, kind: "credits", roomier: null });
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
