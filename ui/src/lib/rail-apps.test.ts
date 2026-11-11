/**
 * D-64 (0.3.4): grouped by App, the rail is about apps. An app's sessions
 * that aren't open sit under it ("Last 3 inactive"); the apps with
 * nothing open follow in "Inactive apps", most recently used first.
 */
import { describe, expect, it } from "vitest";
import { conversationItem, sessionItem } from "./rail-items";
import { appByConversation, appByFolder, appKeyOf, idleApps, idleSessionsByApp, knownKey, offeredLines, recentWords } from "./rail-apps";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";
import { appOfConversation } from "./app-of";
import { buildLines, foldSections, type Line, type SectionId } from "./rail-lines";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { KnownPlaceInfo } from "./ipc/places";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { cliAppKey } from "./app-name";

const NOW = new Date(2026, 10, 1, 15, 0, 0).getTime();
const H = 3_600_000;
const none = () => null;

function place(profile: string): PlaceInfo {
  return { app: "Claude", shown: "Claude app", app_id: "claude-app", profile, instance: profile, pid: 10, kind: "desktop", app_pid: 11, label: `Claude · ${profile}` };
}

function talk(id: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Work Max", project_id: null, cwd: "/w/app", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3 * H, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [place("Work")],
    pid: 10, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}

/** A session that isn't open any more, `hours` ago. */
const past = (id: string, hours: number, over: Partial<ConversationInfo> = {}) => talk(id, { live: false, places: [], pid: null, state: null, last_activity_ms: NOW - hours * H, ...over });

function known(instance: string, over: Partial<KnownPlaceInfo> = {}): KnownPlaceInfo {
  return {
    id: `claude-app|/p/${instance}`, app_id: "claude-app", app: "Claude", instance, profile: instance, label: `Claude · ${instance}`, app_pid: null, lost: false,
    keep_alive: false, paused: null, account_id: null, seen_ms: NOW - 5 * H, cannot_start: null, recent: [], found: false, ...over,
  };
}

const itemsOf = (rows: ConversationInfo[]) => rows.map((row) => conversationItem(row, none, null));
const lines = (rows: ConversationInfo[], places: KnownPlaceInfo[], toggled: string[] = [], query = "") =>
  buildLines({ items: itemsOf(rows), view: DEFAULT_VIEW, query: parseQuery(query), now: NOW, toggled: new Set(toggled), frozen: null, known: places });
const kinds = (out: Line[]) =>
  out.map((line) =>
    line.kind === "section"
      ? `section:${line.id}:${line.count}`
      : line.kind === "header"
        ? `header:${line.group.key}`
        : line.kind === "recent"
          ? `recent:${line.count}`
          : line.kind === "idle"
            ? `idle:${line.app.key}`
            : line.kind === "row" || line.kind === "past" || line.kind === "plain" || line.kind === "need"
              ? `${line.kind}:${line.item.key}`
              : line.kind,
  );

describe("D-64 an app's sessions that aren't open belong to it", () => {
  it("a desktop app's: where they were last seen open, the most recent app winning", () => {
    const by = appByConversation([known("Old", { recent: ["claude-code/x"], seen_ms: NOW - 9 * H }), known("Work", { recent: ["claude-code/x", "claude-code/y"] })]);
    expect(by.get("claude-code/x")).toBe("app|claude-app|Work");
    expect(by.get("claude-code/y")).toBe("app|claude-app|Work");
  });

  it("0.3.5: one its app started but AI Juggler never saw open is that app's profile on its folder, never a terminal app", () => {
    const items = itemsOf([past("app-only", 2, { started_in: "claude-app" }), past("terminal", 3)]);
    const places = [known("Work", { account_id: "a1", profile: "Work Max" })];
    const idle = idleSessionsByApp(items, appByConversation(places), appByFolder(places));
    expect(idle.get("app|claude-app|Work")!.map((item) => item.key)).toEqual(["c:a1/app-only"]);
    expect(idle.get("cli|claude-code|a1")!.map((item) => item.key)).toEqual(["c:a1/terminal"]);
    // Named as the user named the profile (its folder's name), like the Accounts page.
    const apps = idleApps({ openKeys: new Set(), known: places, idle });
    expect(apps.find((app) => app.key === "app|claude-app|Work")!.name).toBe("Claude app · Work Max");
    // Its row says where it ran: the Claude app, not Claude Code.
    expect(appOfConversation(items[0]!.conversation!).name).toBe("Claude app");
  });

  it("leaves out open, archived and needing ones; newest first", () => {
    // An archived one (AI Juggler's own sessions archive), and one asking.
    const items = itemsOf([talk("open"), past("old", 9), past("new", 1), past("gone", 2), past("asks", 3)]).map((item) =>
      item.key === "c:a1/gone" ? { ...item, archived: true } : item.key === "c:a1/asks" ? { ...item, need: "asks" as never } : item,
    );
    const idle = idleSessionsByApp(items, appByConversation([known("Work", { recent: ["claude-code/old", "claude-code/new", "claude-code/gone", "claude-code/asks"] })]));
    expect([...idle.keys()]).toEqual(["app|claude-app|Work"]);
    expect(idle.get("app|claude-app|Work")!.map((item) => item.key)).toEqual(["c:a1/new", "c:a1/old"]);
  });

  it("the apps with nothing open: remembered desktop apps, and CLI apps with sessions of their own, most recent first", () => {
    const items = itemsOf([talk("w1"), past("w2", 1), past("c1", 2, { account_id: "a2", account_label: "Personal" })]);
    const places = [known("Work", { recent: ["claude-code/w2"] }), known("Lab", { seen_ms: NOW - 30 * 60_000, app_pid: 42 }), known("Old", { seen_ms: NOW - 9 * 24 * H })];
    const idle = idleSessionsByApp(items, appByConversation(places));
    const apps = idleApps({ openKeys: new Set(["app|claude-app|Work"]), known: places, idle });
    expect(apps.map((app) => [app.key, app.kind])).toEqual([
      ["app|claude-app|Lab", "desktop"],
      ["cli|claude-code|a2", "cli"],
      ["app|claude-app|Old", "desktop"],
    ]);
    expect(apps[0]!.appPid).toBe(42);
    expect(apps[1]).toMatchObject({ accountId: "a2", tool: "claude-code", profile: "Personal" });
    expect(apps[1]!.sessions.map((item) => item.key)).toEqual(["c:a2/c1"]);
  });

  it("AJ-E3203: a profile found by its folder sorts after the ones you've used, with no time, named by its config folder once known", () => {
    const places = [known("b-side", { found: true, seen_ms: NOW }), known("Old", { seen_ms: NOW - 9 * 24 * H }), known("b-team", { profile: "team-max", seen_ms: NOW - H })];
    const apps = idleApps({ openKeys: new Set(), known: places, idle: new Map() });
    expect(apps.map((app) => [app.name, app.lastMs > 0])).toEqual([
      ["Claude app · team-max", true],
      ["Claude app · Old", true],
      ["Claude app · b-side", false],
    ]);
  });

  it("says how many it shows", () => {
    expect(recentWords(9)).toBe("Last 3 inactive");
    expect(recentWords(2)).toBe("Last 2 inactive");
  });
});

describe("D-64 the rail grouped by App", () => {
  const rows = [talk("w1"), past("w2", 1), past("w3", 2), past("w4", 3), past("w5", 4), past("c1", 2, { account_id: "a2", account_label: "Personal" })];
  const places = [known("Work", { recent: ["claude-code/w2", "claude-code/w3", "claude-code/w4", "claude-code/w5"], app_pid: 11 }), known("Lab")];

  it("an open app's inactive sessions wait behind its Last 3; the apps with nothing open follow Open now", () => {
    const built = lines(rows, places);
    expect(kinds(built.lines)).toEqual([
      "section:open:1",
      "header:app|claude-app|Work",
      "row:c:a1/w1",
      "recent:4",
      "section:idle:2",
      "idle:cli|claude-code|a2",
      "idle:app|claude-app|Lab",
      "also",
    ]);
    // Nothing on Also for them, and no list apart.
    expect(built.also.inactive).toBe(0);
  });

  it("a click shows the last 3 under the app, and an inactive app's under it", () => {
    const built = lines(rows, places, ["app|claude-app|Work#recent", "cli|claude-code|a2#recent"]);
    expect(kinds(built.lines)).toEqual([
      "section:open:1",
      "header:app|claude-app|Work",
      "row:c:a1/w1",
      "recent:4",
      "past:c:a1/w2",
      "past:c:a1/w3",
      "past:c:a1/w4",
      "section:idle:2",
      "idle:cli|claude-code|a2",
      "past:c:a2/c1",
      "idle:app|claude-app|Lab",
      "also",
    ]);
  });

  it("shows 6 inactive apps, then Show n more", () => {
    const many = Array.from({ length: 8 }, (_, n) => known(`P${n}`, { seen_ms: NOW - (n + 1) * H }));
    const out = kinds(lines([talk("w1")], many).lines);
    expect(out.filter((line) => line.startsWith("idle:"))).toHaveLength(6);
    expect(out).toContain("idle-more");
    expect(kinds(lines([talk("w1")], many, ["idle#all"]).lines).filter((line) => line.startsWith("idle:"))).toHaveLength(8);
  });

  it("while searching: only the apps with a match, their matches shown", () => {
    const out = kinds(lines(rows, places, [], "Talk c1").lines);
    expect(out).toEqual(["section:idle:1", "idle:cli|claude-code|a2", "past:c:a2/c1", "also"]);
  });

  it("rc4 walk: while searching, an inactive app shows every match, each under the one it was started from", () => {
    const family = [
      past("parent", 30),
      past("child-a", 29, { started_from: "parent" }),
      past("child-b", 28, { started_from: "parent" }),
      past("grandchild", 27, { started_from: "child-a" }),
    ].map((row) => ({ ...row, title: `Move live: ${row.id}` }));
    const out = lines(family, [], [], "Move live").lines.filter((line) => line.kind === "past");
    // All four, not the last three; nested parent → child → grandchild,
    // siblings newest first as everywhere in the rail.
    expect(out.map((line) => (line.kind === "past" ? `${line.item.key}@${line.depth}` : ""))).toEqual([
      "c:a1/parent@0",
      "c:a1/child-b@1",
      "c:a1/child-a@1",
      "c:a1/grandchild@2",
    ]);
  });

  it("while searching, an app with something open is never an inactive app", () => {
    // w2 matches, w1 (open in the same app) doesn't.
    const out = kinds(lines(rows, places, [], "Talk w2").lines);
    expect(out.some((line) => line === "idle:app|claude-app|Work")).toBe(false);
  });

  it("Inactive apps folds from its head, as Open now does", () => {
    const out = kinds(foldSections(lines(rows, places).lines, new Set(["idle"])));
    expect(out.slice(-2)).toEqual(["section:idle:2", "also"]);
  });

  it("by account, inactive sessions stay a view of their own", () => {
    const built = buildLines({ items: itemsOf([talk("w1"), past("w2", 1, { inactive: true })]), view: { ...DEFAULT_VIEW, group: "account", show: { ...DEFAULT_VIEW.show, inactive: true } }, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places });
    const out = kinds(built.lines);
    expect(out).not.toContain("section:idle:1");
    expect(out).toContain("plain:c:a1/w2");
    expect(built.also.inactive).toBe(1);
  });
});

describe("AJ-E3206 a desktop session AI Juggler started", () => {
  it("belongs to the profile it was launched in, else the one profile on its folder, else its folder's CLI App (rc22, A3)", () => {
    const row = { id: "s1", status: "running", needs_attention: null, provider: "claude-code", kind: "desktop", account_id: "a1", account_label: "Work", project_id: null, started_at_ms: NOW, last_activity_ms: NOW } as unknown as InstanceInfo;
    const item = sessionItem(row, none, false);
    const places = [known("old", { account_id: "a1", seen_ms: NOW - 9 * H }), known("work-max", { account_id: "a1", seen_ms: NOW - H })];
    // Launched in the older one: that one's, never the newest seen.
    const launched = sessionItem({ ...row, app_id: places[0]!.id }, none, false);
    expect(appKeyOf(launched, appByConversation(places), appByFolder(places))).toBe(knownKey(places[0]!));
    // Two profiles on its folder and none stored: its folder's CLI App
    // (the backend counts it there), not a guess.
    expect(appKeyOf(item, appByConversation(places), appByFolder(places))).toBe(cliAppKey("claude-code", "a1", "Work"));
    // One profile on its folder: that one's.
    expect(appKeyOf(item, new Map(), appByFolder([places[1]!]))).toBe(knownKey(places[1]!));
    expect(appKeyOf(item, new Map(), new Map())).toBe(cliAppKey("claude-code", "a1", "Work"));
  });

  it("rc23: its open and its ended ones are one App's group, Open now and Inactive apps alike", () => {
    const row = (id: string, status: InstanceInfo["status"]) =>
      sessionItem({
        id, launch_recipe_id: "r", provider: "claude-code", account_id: "a1", account_label: "Work", model: null, launch_model: null, running_model: null,
        model_source: "launch", status, quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, subtask_count: null, git_branch: null,
        git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 2 * H, last_activity_ms: NOW - H, machine_id: "this-mac", cwd: "/w",
        kind: "desktop", adopted: false, project_id: null, conversation_id: null, isolation_label: null, app_id: null,
      }, none, false);
    const kinds2 = (items: ReturnType<typeof row>[], places: KnownPlaceInfo[]) =>
      kinds(buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places }).lines);
    const items = [row("open", "running"), row("ended", "finished")];
    // No profile on its folder: its folder's CLI App, open and ended alike
    // (the ended one sat in Inactive apps, the open one in a group apart).
    const cli = cliAppKey("claude-code", "a1", "Work");
    expect(kinds2(items, [])).toEqual(["section:open:1", `header:${cli}`, "row:s:open", "recent:1", "also"]);
    // One profile on its folder: that profile's, never also an inactive app.
    const work = known("Work", { account_id: "a1" });
    expect(kinds2(items, [work])).toEqual(["section:open:1", `header:${knownKey(work)}`, "row:s:open", "recent:1", "also"]);
  });
});

/**
 * ADR-0032 (D-100): the rail shows what runs. Open now is every App that
 * runs, sessions or not; Inactive apps every known App that doesn't; no
 * App is in both; a session is under the App the backend says owns it.
 */
describe("ADR-0032 presence: the rail matches what runs", () => {
  const sections = (out: Line[]) => {
    const open = new Set<string>();
    const inactive = new Set<string>();
    let at: SectionId | null = null;
    for (const line of out) {
      if (line.kind === "section") at = line.id;
      else if (line.kind === "header" && at === "open") open.add(line.group.key);
      else if (line.kind === "idle") inactive.add(line.app.key);
    }
    return { open, inactive };
  };
  const running = (instance: string, pid: number, over: Partial<KnownPlaceInfo> = {}) => known(instance, { id: `app:claude-app:/p/${instance}`, app_pid: pid, ...over });

  it("an App that runs with nothing open in it is in Open now, saying so, never in Inactive apps", () => {
    const places = [running("Luka", 41), running("Mia", 42), known("Gone")];
    const out = lines([], places).lines;
    const { open, inactive } = sections(out);
    expect([...open].sort()).toEqual(["app|claude-app|Luka", "app|claude-app|Mia"]);
    expect([...inactive]).toEqual(["app|claude-app|Gone"]);
    const header = out.find((line) => line.kind === "header" && line.group.key === "app|claude-app|Luka");
    expect(header && header.kind === "header" ? header.folded : null).toBe("Luka");
    // By App, Open now counts the Apps open.
    expect(kinds(out)[0]).toBe("section:open:2");
  });

  it("each App is in exactly one section, and Open now's Apps are exactly the running ones", () => {
    const places = [running("A", 1), running("B", 2), known("C"), known("D", { id: "app:claude-app:/p/D" })];
    const rows = [talk("live-a", { places: [place("A")], app: "app:claude-app:/p/A" }), past("old", 5, { app: "app:claude-app:/p/D" })];
    const { open, inactive } = sections(lines(rows, places).lines);
    for (const key of open) expect(inactive.has(key)).toBe(false);
    expect(open).toEqual(new Set(places.filter((p) => p.app_pid !== null).map(knownKey)));
    expect(inactive).toEqual(new Set(places.filter((p) => p.app_pid === null).map(knownKey)));
  });

  it("a session is under the App the backend names: a terminal one on a profile's folder is its CLI App's, never the profile's", () => {
    const places = [running("Anela", 7, { account_id: "a1", profile: "Anela" })];
    const terminal: PlaceInfo = { app: "", shown: "", app_id: null, profile: null, instance: null, pid: 90, kind: null, app_pid: null, label: "" };
    // "Jive CLI" (2026-11-11): open with no recognised terminal on its chain.
    const jive = talk("jive", { places: [terminal], app: "cli:a1" });
    const out = lines([jive], places).lines;
    const row = out.find((line) => line.kind === "row" && line.item.key === "c:a1/jive");
    expect(row && row.kind === "row" ? row.group.key : null).toBe(cliAppKey("claude-code", "a1", "Work Max"));
    // The profile still shows, open with nothing in it.
    expect(sections(out).open.has("app|claude-app|Anela")).toBe(true);
    // And its CLI App's name says CLI, so the two never read as one.
    const header = out.find((line) => line.kind === "header" && line.group.key.startsWith("cli|"));
    expect(header && header.kind === "header" ? header.group.place?.app : null).toBe("Claude Code CLI");
  });

  it("pins never mean open: an ended pinned session shows pinned under its App, which stays Inactive", () => {
    const places = [known("Work", { id: "app:claude-app:/p/Work" })];
    const items = itemsOf([past("jive", 22, { app: "app:claude-app:/p/Work" })]).map((item) => ({ ...item, pinned: true }));
    const out = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, known: places }).lines;
    const { open, inactive } = sections(out);
    expect(open.size).toBe(0);
    expect([...inactive]).toEqual(["app|claude-app|Work"]);
    // Shown under it without unfolding its "Last inactive".
    expect(out.some((line) => line.kind === "past" && line.item.key === "c:a1/jive")).toBe(true);
  });

  it("an ended session is under the App the backend last saw it in, whatever its folder", () => {
    const places = [known("Work", { id: "app:claude-app:/p/Work", account_id: "a1" })];
    const items = itemsOf([past("in-app", 2, { app: "app:claude-app:/p/Work" }), past("in-term", 3, { app: "cli:a1", started_in: "claude-app" })]);
    const idle = idleSessionsByApp(items, appByConversation(places), appByFolder(places));
    expect(idle.get("app|claude-app|Work")!.map((item) => item.key)).toEqual(["c:a1/in-app"]);
    expect(idle.get("cli|claude-code|a1")!.map((item) => item.key)).toEqual(["c:a1/in-term"]);
  });
});

/**
 * ADR-0032 Amendment 1 (D-84, D-100): three exclusive states. Open now is
 * the user's Apps that run, "Running, not added" the apps found running
 * that the user hasn't added (Add), Inactive apps the user's that don't;
 * every App is in exactly one.
 */
describe("ADR-0032 Running, not added", () => {
  const running = (instance: string, pid: number, over: Partial<KnownPlaceInfo> = {}) => known(instance, { id: `app:claude-app:/p/${instance}`, app_pid: pid, ...over });
  const offer = (id: string, isRunning = true): PlaceOfferInfo => ({ id, label: `Claude app · ${id.split("/").pop()}`, running: isRunning });
  const build = (rows: ConversationInfo[], places: KnownPlaceInfo[], offers: PlaceOfferInfo[], query = "") =>
    buildLines({ items: itemsOf(rows), view: DEFAULT_VIEW, query: parseQuery(query), now: NOW, toggled: new Set(), frozen: null, known: places, offers }).lines;
  /** Each App's sections, by its id (an offer's, a known App's, a CLI App's `cli:<folder>`). */
  const where = (out: Line[], places: KnownPlaceInfo[]) => {
    const idOf = (key: string) => places.find((place) => knownKey(place) === key)?.id ?? (key.startsWith("cli|") ? `cli:${key.split("|")[2]}` : key);
    const out_ = new Map<string, SectionId[]>();
    const add = (id: string, at: SectionId) => out_.set(id, [...(out_.get(id) ?? []), at]);
    let at: SectionId | null = null;
    for (const line of out) {
      if (line.kind === "section") at = line.id;
      else if (line.kind === "header" && at === "open") add(idOf(line.group.key), "open");
      else if (line.kind === "offer") add(line.offer.id, "offered");
      else if (line.kind === "idle") add(idOf(line.app.key), "idle");
    }
    return out_;
  };

  it("lists each running offer once, between Open now and Inactive apps, and every App is in exactly one section", () => {
    const places = [running("Work", 7), known("Gone", { id: "app:claude-app:/p/Gone" })];
    const offers = [
      offer("app:claude-app:/p/aij-test"),
      offer("app:claude-app:/p/aij-test"), // twice from the backend: once here
      offer("app:claude-app:/p/Work"), // the user's own: Open now has it
      offer("app:claude-app:/p/Gone"), // the user's own, not running: Inactive has it
      offer("app:claude-app:/p/asleep", false), // not running: not here
      offer("cli:a1"), // a CLI App the rail shows already (its session is open)
    ];
    const out = build([talk("t1", { places: [], app: "cli:a1" })], places, offers);
    const sections = where(out, places);
    for (const [id, at] of sections) expect({ id, at }).toEqual({ id, at: [at[0]] });
    expect(sections.get("app:claude-app:/p/aij-test")).toEqual(["offered"]);
    expect(sections.get("app:claude-app:/p/Work")).toEqual(["open"]);
    expect(sections.get("app:claude-app:/p/Gone")).toEqual(["idle"]);
    expect(sections.get("cli:a1")).toEqual(["open"]);
    expect(sections.has("app:claude-app:/p/asleep")).toBe(false);
    // In order: Open now, then Running, not added (its count), then Inactive apps.
    const heads = out.flatMap((line) => (line.kind === "section" ? [`${line.id}:${line.count}`] : []));
    expect(heads).toEqual(["open:2", "offered:1", "idle:1"]);
  });

  it("has no section when nothing runs unadded, and none while a search is on", () => {
    expect(build([], [], [offer("app:claude-app:/p/x", false)]).some((line) => line.kind === "section" && line.id === "offered")).toBe(false);
    expect(build([], [], [offer("app:claude-app:/p/x")], "nothing").some((line) => line.kind === "offer")).toBe(false);
  });

  it("offeredLines on its own: by id, the running ones, minus the user's", () => {
    const places = [known("Mine", { id: "app:claude-app:/p/Mine" })];
    const out = offeredLines([offer("app:claude-app:/p/Mine"), offer("cli:f9"), offer("cli:f1")], places, ["cli|claude-code|f1"]);
    expect(out.map((line) => (line.kind === "offer" ? line.offer.id : `section:${line.count}`))).toEqual(["section:1", "cli:f9"]);
  });
});
