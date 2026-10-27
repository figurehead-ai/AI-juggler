/**
 * AJ-E2104, AJ-E2107 — the rail's model, rule by rule: one state
 * vocabulary, "probably" for what's worked out, Done clears once seen,
 * places in a stable order, quiet places fold, rows hold still under the
 * pointer, the View is read guarded, and search and ⌘K take its words.
 */
import { describe, expect, it } from "vitest";
import { conversationItem, endedToday, goToWhere, itemsOf, needsYou, openNow, sessionItem, stateWord, OWN_PLACE } from "./rail-items";
import { buildLines, foldSections } from "./rail-lines";
import { foldGroup, frozenOrder, groupItems, isQuiet, quietWords } from "./rail-places";
import { DEFAULT_VIEW, isDefaultView, matches, paletteMatches, parseQuery, parseView } from "./rail-view";
import { parseSeen } from "./seen";
import { contrast, luminance } from "./oklch";
import { swatchOf } from "./palette";
import { PROFILE_HUES, STATUS_HUES, assignProfiles, profileMonogram } from "./profile-marks";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { InstanceInfo } from "./bridge";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
const none = () => null;

function place(app: string, profile: string, pid = 10): PlaceInfo {
  return { app, app_id: app.toLowerCase(), profile, instance: profile, pid, kind: "desktop", app_pid: pid + 1, label: `${app} · ${profile}` };
}

function talk(id: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Work Max", project_id: null, cwd: "/w/app", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3_600_000, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [place("Claude", "Work")],
    pid: 10, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, ...over,
  };
}

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "a1", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/w/app", kind: "terminal", isolation_label: null, project_id: null, conversation_id: null, ...over,
  } as InstanceInfo;
}

describe("one state vocabulary (AJ-E2104 Req.3, AJ-E2107)", () => {
  it("every app's status maps onto Asks you, Working, Idle, Done (review it), Ended", () => {
    expect(conversationItem(talk("a", { state: "asks_you" }), none, null).state).toBe("asks_you");
    expect(conversationItem(talk("w", { state: "working" }), none, null).state).toBe("working");
    expect(conversationItem(talk("i", { state: "idle" }), none, null).state).toBe("idle");
    expect(conversationItem(talk("d", { state: "done_review" }), none, null).state).toBe("done_review");
    expect(conversationItem(talk("e", { live: false, places: [] }), none, null).state).toBe("ended");
    // A tool that doesn't say: Idle, never a guess at Working.
    expect(conversationItem(talk("n", { state: null }), none, null).state).toBe("idle");
    expect(sessionItem(own("s"), none, false).state).toBe("working");
    expect(sessionItem(own("q", { status: "waiting_user" }), none, false)).toMatchObject({ state: "asks_you", need: "asks" });
    expect(sessionItem(own("l", { status: "quota_blocked", needs_attention: "quota_blocked" }), none, false).need).toBe("limit");
    expect(sessionItem(own("f", { status: "failed" }), none, false)).toMatchObject({ state: "ended", need: "failed" });
    expect(sessionItem(own("x", { status: "finished" }), none, false).state).toBe("ended");
    // AJ-E2302/E2306: a runtime that says (its protocol or its hooks) is
    // the state, exactly: idle while running, not "working".
    const exact = (state: string) => own("r", { runtime: { kind: "driven", label: null, beta: false, state, asks: [], can_steer: false, fell_back: null, error: null, limits: [], cut_off: null } });
    expect(sessionItem(exact("idle"), none, false)).toMatchObject({ state: "idle", probably: false });
    expect(sessionItem(exact("working"), none, false).state).toBe("working");
    expect(sessionItem(exact("nonsense"), none, false).state).toBe("working");
  });

  it("anything worked out says probably", () => {
    const tail = conversationItem(talk("t", { state: "asks_you", probably: true }), none, null);
    expect(stateWord(tail.state, tail.probably)).toBe("Probably asks you");
    expect(stateWord("asks_you")).toBe("Asks you");
    expect(sessionItem(own("st", { status: "stuck", needs_attention: "stuck" }), none, false).probably).toBe(true);
  });

  it("Done, review it clears once the user looked: AI Juggler's seen mark, or the app's own view", () => {
    const done = talk("d", { state: "done_review", last_activity_ms: NOW - 60_000 });
    expect(conversationItem(done, none, null)).toMatchObject({ state: "done_review", need: "review" });
    expect(conversationItem(done, none, NOW).state).toBe("idle");
    expect(conversationItem({ ...done, viewed_ms: NOW }, none, null).state).toBe("idle");
    // Something new since: to review again.
    expect(conversationItem({ ...done, last_activity_ms: NOW + 1 }, none, NOW).state).toBe("done_review");
  });

  it("Needs you counts sessions other apps run, asks first, the longest waiting first", () => {
    const items = itemsOf({
      sessions: [own("mine", { status: "waiting_user", last_activity_ms: NOW - 1000 }), own("lim", { status: "quota_blocked" })],
      conversations: [talk("theirs", { state: "asks_you", last_activity_ms: NOW - 5000 }), talk("rev", { state: "done_review" }), talk("calm")],
      projects: [],
      seen: {},
    });
    expect(needsYou(items).map((item) => item.key)).toEqual(["c:a1/theirs", "s:mine", "s:lim", "c:a1/rev"]);
    expect(items.filter(openNow).map((item) => item.key)).toEqual(["c:a1/calm"]);
  });

  it("AI Juggler's own sessions are the place AI Juggler; a snoozed one waits in it", () => {
    const item = sessionItem(own("s", { status: "waiting_user" }), none, true);
    expect(item.place.key).toBe(OWN_PLACE);
    expect(item.need).toBeNull();
  });

  it("ended today, not before, and archived only when asked", () => {
    const today = conversationItem(talk("e", { live: false, places: [], last_activity_ms: NOW - 3_600_000 }), none, null);
    const old = conversationItem(talk("o", { live: false, places: [], last_activity_ms: NOW - 30 * 3_600_000 }), none, null);
    expect(endedToday(today, NOW)).toBe(true);
    expect(endedToday(old, NOW)).toBe(false);
    const archived = { ...sessionItem(own("x", { status: "finished", last_activity_ms: NOW - 1000 }), none, false), archived: true };
    expect(endedToday(archived, NOW)).toBe(false);
    expect(endedToday(archived, NOW, true)).toBe(true);
  });
});

describe("places (AJ-E2104 Req.1, Req.4, Req.6)", () => {
  const items = () =>
    itemsOf({
      sessions: [own("aj")],
      conversations: [
        talk("t1", { places: [place("Terminal", "Work")], state: "working" }),
        talk("c1", { places: [place("Claude", "Work")] }),
        talk("c2", { places: [place("Claude", "Personal")], state: "working" }),
        talk("both", { places: [place("Claude", "Work"), place("Terminal", "Work", 30)] }),
      ],
      projects: [],
      seen: {},
    });

  it("groups by app and profile in a stable order: AI Juggler first, then by app and profile", () => {
    const groups = groupItems(items(), "place");
    expect(groups.map((group) => `${group.label}|${group.place?.profile ?? ""}`)).toEqual(["AI Juggler|", "Claude|Personal", "Claude|Work", "Terminal|Work"]);
    // A row's state changing never moves its place.
    const again = groupItems(items().map((item) => ({ ...item, state: "idle" as const })), "place");
    expect(again.map((group) => group.key)).toEqual(groups.map((group) => group.key));
  });

  it("by account: profiles on one subscription share a group (AJ-E2109)", () => {
    const work = talk("w1", { account_id: "p-work", account_label: "Work" });
    const side = talk("w2", { account_id: "p-side", account_label: "Side" });
    const other = talk("w3", { account_id: "p-own", account_label: "Own" });
    const all = itemsOf({ sessions: [], conversations: [work, side, other], projects: [], seen: {} });
    const sub = { key: "acct-1", label: "Claude Max" };
    const groups = groupItems(all, "account", (id) => (id === "p-work" || id === "p-side" ? sub : null));
    expect(groups.map((group) => [group.label, group.items.length])).toEqual([["Claude Max", 2], ["Own", 1]]);
  });

  it("by pool (AJ-E1702): your pools by name, a session another app runs in no pool, last", () => {
    const all = itemsOf({ sessions: [own("s1"), own("s2")], conversations: [talk("c1")], projects: [], seen: {} });
    const poolOf = (item: { session: InstanceInfo | null }) =>
      item.session === null ? null : item.session.id === "s1" ? { id: "p-team", label: "Team" } : { id: "p-default", label: "app · all sessions" };
    const groups = groupItems(all, "pool", none, poolOf);
    expect(groups.map((group) => [group.label, group.poolId, group.items.length])).toEqual([
      ["app · all sessions", "p-default", 1],
      ["Team", "p-team", 1],
      ["Not in a pool", null, 1],
    ]);
    // A pool's header is the pool, never a place to bring forward.
    expect(groups.every((group) => group.place === null)).toBe(true);
  });

  it("open in two places is one row that says so", () => {
    const both = items().find((item) => item.key === "c:a1/both")!;
    expect(both.alsoIn).toEqual(["Terminal · Work"]);
  });

  it("a quiet place folds to one header; a place shows working rows and the last 3, then +n quiet", () => {
    const quiet = groupItems(items(), "place").find((group) => group.label === "Claude" && group.place?.profile === "Work")!;
    expect(isQuiet(quiet)).toBe(true);
    expect(quietWords(quiet)).toBe("Work · 2 idle");
    const many = { ...quiet, items: [...Array(6)].map((_, n) => ({ ...quiet.items[0]!, key: `k${n}`, lastMs: NOW - n, state: n === 5 ? ("working" as const) : ("idle" as const) })) };
    const fold = foldGroup(many, false);
    expect(fold.shown.map((item) => item.key)).toEqual(["k5", "k0", "k1", "k2"]);
    expect(fold.quiet).toBe(2);
    expect(foldGroup(many, true).quiet).toBe(0);
  });

  it("a pinned session keeps its place: under its place once ended, leading it, never folded away (AJ-E1502)", () => {
    const pin = sessionItem(own("pin", { status: "finished", pinned: true, last_activity_ms: NOW - 9_000_000 }), none, false);
    expect(openNow(pin)).toBe(true);
    expect(endedToday(pin, NOW)).toBe(false);
    // Needs you still comes first: a pinned one that asks is there.
    expect(openNow(sessionItem(own("ask", { status: "waiting_user", pinned: true }), none, false))).toBe(false);
    const quiet = [...Array(5)].map((_, n) => sessionItem(own(`q${n}`, { status: "stopped", last_activity_ms: NOW - n }), none, false));
    const group = groupItems([...quiet, pin], "place")[0]!;
    expect(isQuiet(group)).toBe(false);
    const fold = foldGroup(group, false);
    expect(fold.shown[0]!.key).toBe("s:pin");
    expect(fold.shown).toHaveLength(4);
    expect(fold.quiet).toBe(2);
  });

  it("rows hold still under the pointer: known ones keep their places, new ones join at the end", () => {
    const next = ["c", "a", "new", "b"].map((key) => ({ key }));
    expect(frozenOrder(next, ["a", "b", "c"], (item) => item.key).map((item) => item.key)).toEqual(["a", "b", "c", "new"]);
  });
});

describe("the View and its words (AJ-E2104 Req.5)", () => {
  it("is read guarded: junk or a wrong shape is the defaults; Pool is kept as picked", () => {
    expect(parseView(null)).toEqual(DEFAULT_VIEW);
    expect(parseView("{not json")).toEqual(DEFAULT_VIEW);
    expect(parseView(JSON.stringify({ group: "lanes" })).group).toBe("place");
    // AJ-E1702: the rail shows places until a pool of yours exists.
    expect(parseView(JSON.stringify({ group: "pool" })).group).toBe("pool");
    expect(parseView(JSON.stringify({ group: "project", show: { ended: true, quiet: "yes" }, only: { app: "Claude", mac: 3 } }))).toEqual({
      group: "project",
      show: { ...DEFAULT_VIEW.show, ended: true },
      only: { app: "Claude" },
    });
    expect(isDefaultView(DEFAULT_VIEW)).toBe(true);
    expect(isDefaultView({ ...DEFAULT_VIEW, only: { app: "Claude" } })).toBe(false);
  });

  it("search takes is: and the Only words, then free text", () => {
    const query = parseQuery("is:ended app:cla Profile:work fix login");
    expect([...query.is]).toEqual(["ended"]);
    expect(query.only).toEqual({ app: "cla", profile: "work" });
    expect(query.text).toBe("fix login");
    const item = conversationItem(talk("x", { title: "Fix the login test", places: [place("Claude", "Work Max")] }), none, null);
    expect(matches(item, {}, query)).toBe(true);
    expect(matches(item, {}, parseQuery("app:terminal"))).toBe(false);
    expect(matches(item, { app: "Terminal" }, parseQuery(""))).toBe(false);
    expect(matches(item, {}, parseQuery("is:asks"))).toBe(false);
  });

  it("⌘K takes the same words: is:ended brings the ended ones in", () => {
    const all = [talk("open"), talk("gone", { live: false, places: [] })];
    expect(paletteMatches([], all, [], parseQuery("")).conversations.map((c) => c.id)).toEqual(["open"]);
    expect(paletteMatches([], all, [], parseQuery("is:ended")).conversations.map((c) => c.id)).toEqual(["open", "gone"]);
  });
});

describe("the seen mark is read guarded (AJ-E2107)", () => {
  it("keeps only key → time", () => {
    expect(parseSeen('{"c:a/1": 5, "c:a/2": "x"}')).toEqual({ "c:a/1": 5 });
    expect(parseSeen("[1,2]")).toEqual({});
    expect(parseSeen("nope")).toEqual({});
  });
});

describe("profile badges (AJ-E2104 Req.2)", () => {
  const apart = (a: number, b: number) => Math.min(Math.abs(a - b), 360 - Math.abs(a - b));
  it("white letters 4.5:1 or better, on hues 20° or more from every status colour", () => {
    for (const hue of PROFILE_HUES) {
      expect(contrast(luminance(swatchOf(hue)), 1), `hue ${hue}`).toBeGreaterThanOrEqual(4.5);
      for (const status of STATUS_HUES) expect(apart(hue, status), `${hue} vs ${status}`).toBeGreaterThanOrEqual(20);
    }
  });

  it("profiles sharing a first word get letters that tell them apart", () => {
    assignProfiles(["b-work", "b-personal"]);
    expect(profileMonogram("b-work")).not.toBe(profileMonogram("b-personal"));
  });
});

describe("the maintainer's review of 0.1.4: the way out, and folding in place", () => {
  it("Go to it names the app and profile; AI Juggler's own terminal session has none (its page is where it runs)", () => {
    expect(goToWhere(conversationItem(talk("c"), none, null))).toBe("Claude · Work");
    expect(goToWhere(conversationItem(talk("bare", { places: [] }), none, null))).toBe("An app · Work Max");
    const row = { id: "s", status: "running", needs_attention: null, account_id: "a1", account_label: "Work Max", project_id: null, kind: "terminal", started_at_ms: NOW, last_activity_ms: NOW } as unknown as InstanceInfo;
    expect(goToWhere(sessionItem(row, none, false))).toBeNull();
    expect(goToWhere(sessionItem({ ...row, kind: "desktop" }, none, false))).toBe("its app · Work Max");
  });

  it("a folded section keeps its head and drops its lines, up to the Also line", () => {
    const items = [conversationItem(talk("asks", { state: "asks_you" }), none, null), conversationItem(talk("busy", { state: "working" }), none, null)];
    const { lines } = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null });
    const kinds = (folded: ("needs" | "open")[]) => foldSections(lines, new Set(folded)).map((line) => line.kind);
    expect(kinds([])).toEqual(["section", "need", "section", "header", "row", "also"]);
    expect(kinds(["needs"])).toEqual(["section", "section", "header", "row", "also"]);
    expect(kinds(["open"])).toEqual(["section", "need", "section", "also"]);
  });
});

describe("a row's sub-agents come from one source (AJ-E1520, 0.2)", () => {
  it("is the subagent count its tab reads, never its child processes", () => {
    expect(sessionItem(own("s1", { subtask_count: 3, child_count: 5 }), none, false).subagents).toBe(3);
    expect(sessionItem(own("s2", { subtask_count: null, child_count: 2 }), none, false).subagents).toBe(0);
  });
});
