/**
 * The review of 0.1.5: Home's "Pick up where you left off" rows all read
 * "Untitled session · payments-api". An untitled session takes the first
 * words of the last thing asked, else its branch or project and when it
 * started; no two rows read the same.
 */
import { describe, expect, it } from "vitest";
import { firstWords, namedByFacts, shortName, startedWords, tellApart, untitledName } from "./session-titles";
import { conversationTitle } from "./conversations";
import { clock, pastWhen } from "./time-words";
import type { ConversationInfo } from "./ipc/conversations";
import { recentRows } from "../components/HomeRecent";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { handoverTeaser } from "../components/HandoverSection";
import cases from "./fixtures/session-names.json";

const NOW = new Date(2026, 9, 25, 15, 0, 0).getTime();

describe("firstWords", () => {
  it("skips a short opener and stops at the clause", () => {
    expect(firstWords("Good. Keep the old shortcuts working; the test key is •••")).toBe("Keep the old shortcuts working");
  });
  it("cuts a long ask to a few words, with an ellipsis", () => {
    expect(firstWords("Please migrate every billing webhook handler to the new queue and remove the old cron")).toBe("Please migrate every billing webhook handler to the…");
  });
  it("keeps a question's mark, never a redacted key", () => {
    expect(firstWords("Why does checkout fail?")).toBe("Why does checkout fail?");
    expect(firstWords("••• is the key")).toBeNull();
    expect(firstWords("   \n  ")).toBeNull();
  });
});

describe("startedWords", () => {
  it("says today's clock, yesterday's, or the day", () => {
    expect(startedWords(NOW - 3_600_000, NOW)).toMatch(/^started \d/);
    expect(startedWords(NOW - 86_400_000, NOW)).toMatch(/^started yesterday \d/);
  });
  it("D-96: as whenWords says a day, looking back: its weekday in the week, its date past it, its year in another", () => {
    const at = (days: number) => NOW - days * 86_400_000;
    const short = (ms: number, opts: Intl.DateTimeFormatOptions) => new Date(ms).toLocaleDateString([], opts);
    expect(startedWords(at(0) - 3_600_000, NOW)).toBe(`started ${clock(at(0) - 3_600_000)}`);
    expect(startedWords(at(3), NOW)).toBe(`started ${short(at(3), { weekday: "short" })} ${clock(at(3))}`);
    expect(startedWords(at(10), NOW)).toBe(`started ${short(at(10), { weekday: "short", month: "short", day: "numeric" })} ${clock(at(10))}`);
    expect(startedWords(at(400), NOW)).toContain(String(new Date(at(400)).getFullYear()));
    // Never the long day ("Thursday 23 October") the rail has no room for.
    expect(startedWords(at(10), NOW)).not.toMatch(/day /);
  });
});

/** D-96 (persona D2, Devon): two untitled sessions in one folder read as
 * one, "Untitled session · billing". Their facts tell them apart.
 * D-103.8 (persona R2): "client-b · Client A · client-a · started 4:46 PM"
 * named its Account in its name; its row says that, never its name. */
describe("an untitled session's name, by its facts", () => {
  const at = new Date(2026, 9, 25, 1, 14, 0).getTime();
  const untitled = (over: Partial<InstanceInfo> = {}) => session("u", { cwd: "/Users/you/Dev/billing", account_label: "Work", started_at_ms: at, ...over });
  it("is its folder · when it started, never its App or Account (D-103.8)", () => {
    expect(shortName(untitled(), NOW)).toBe(`billing · started ${clock(at)}`);
    expect(shortName(untitled(), NOW)).not.toContain("Untitled");
    expect(shortName(untitled({ cwd: "/Users/you/Dev/client-b", account_label: "Client A · client-a" }), NOW)).toBe(`client-b · started ${clock(at)}`);
  });
  it("an older one carries its day, as pastWhen says it", () => {
    const older = at - 3 * 86_400_000;
    expect(shortName(untitled({ started_at_ms: older }), NOW)).toBe(`billing · started ${pastWhen(older, NOW)}`);
    expect(pastWhen(older, NOW)).not.toBe(clock(older));
  });
  it("tells two in one folder apart by when each started; the same minute on two Accounts reads alike (numbered in a list)", () => {
    expect(shortName(untitled(), NOW)).not.toBe(shortName(untitled({ started_at_ms: at + 60_000 }), NOW));
    expect(shortName(untitled(), NOW)).toBe(shortName(untitled({ account_label: "Personal" }), NOW));
  });
  it("keeps a label, a title's first words and a telling branch first, as before", () => {
    expect(shortName(untitled({ label: "Writer" }), NOW)).toBe("Writer");
    expect(shortName(untitled({ title: "Fix the login test." }), NOW)).toBe("Fix the login test");
    expect(shortName(untitled({ git_branch: "fix/limits" }), NOW)).toBe("fix/limits");
    expect(shortName(untitled({ git_branch: "main" }), NOW)).not.toContain("main");
  });
  it("a desktop one reads the same: its profile isn't in its name either", () => {
    expect(shortName(untitled({ kind: "desktop", account_label: "b-work" }), NOW)).toBe(`billing · started ${clock(at)}`);
    expect(shortName(untitled({ account_label: "you@example.com" }), NOW)).toBe(`billing · started ${clock(at)}`);
  });
  it("says what it knows when it knows less", () => {
    expect(untitledName({ folder: null, startedMs: at }, NOW)).toBe(`Session started ${clock(at)}`);
    expect(untitledName({ folder: "billing", startedMs: null }, NOW)).toBe("billing");
    expect(untitledName({ folder: null, startedMs: null }, NOW)).toBe("Untitled session");
  });
  it("another app's session follows the same rule", () => {
    const c: ConversationInfo = { id: "c-1", tool: "claude-code", account_id: "w", account_label: "Work", project_id: null, cwd: "/Users/you/Dev/billing", title: null, git_branch: "main", started_ms: at, last_activity_ms: at, tokens: null, live: false, session_id: null, places: [], pid: null, state: null, probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null };
    expect(conversationTitle(c, NOW)).toBe(shortName(untitled(), NOW));
    expect(conversationTitle({ ...c, title: "Ship it" }, NOW)).toBe("Ship it");
  });
  it("the same folder and minute: numbered by when each started", () => {
    const rows = [untitled({ id: "b", started_at_ms: at + 5_000 }), untitled({ id: "a", account_label: "Personal" }), untitled({ id: "c", title: "Ship it" })];
    const apart = tellApart(rows, (row) => shortName(row, NOW), (row) => [row.started_at_ms, row.id]);
    expect(rows.map((row) => apart.get(row))).toEqual([`billing · started ${clock(at)} (2)`, `billing · started ${clock(at)} (1)`, "Ship it"]);
    expect(namedByFacts(rows[0]!)).toBe(true);
    expect(namedByFacts(rows[2]!)).toBe(false);
  });
});

/** D-103.8: the backend names a session by the same rule, from the same
 * cases (`crates/juggler-core/src/naming.rs`'s tests). */
describe("the backend and the window name a session the same way", () => {
  const at = new Date(2026, 9, 25, 1, 14, 0).getTime();
  it.each(cases.cases)("$name", (c) => {
    const row = { label: c.label ?? undefined, title: c.title ?? undefined, git_branch: c.branch, cwd: c.folder ? `/Users/you/Dev/${c.folder}` : "", started_at_ms: c.started ? at : 0 };
    expect(shortName(row, NOW)).toBe(c.name.replace("{started}", startedWords(at, NOW)));
    expect(namedByFacts(row)).toBe(c.byFacts);
  });
});

function session(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "unknown", status: "finished", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 7_200_000,
    last_activity_ms: NOW - 600_000, machine_id: "this-mac", cwd: "/Users/you/Dev/payments-api", kind: "terminal", isolation_label: null,
    project_id: "proj-payments", conversation_id: null, ...over,
  } as InstanceInfo;
}
const PROJECTS = [{ id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api" }] as ProjectInfo[];

describe("recentRows", () => {
  it("titles an untitled row by what was asked, the where line keeps the project", () => {
    const asked = new Map([["claude-work\u0000c-1", "Keep the old shortcuts working"]]);
    const [row] = recentRows([session("a", { conversation_id: "c-1" })], PROJECTS, asked, NOW);
    expect(row!.title).toBe("Keep the old shortcuts working");
    expect(row!.where).toBe("payments-api · main · Work Max");
  });
  it("without words: its facts — project · when it started, never 'Untitled' (D-96), never its Account (D-103.8)", () => {
    const [row] = recentRows([session("a")], PROJECTS, new Map(), NOW);
    expect(row!.title).toBe(`payments-api · started ${clock(NOW - 7_200_000)}`);
    expect(row!.title).not.toMatch(/Untitled/);
    // The where line doesn't say again what the title says; it says the Account.
    expect(row!.where).toBe("main · Work Max");
  });
  it("two untitled in one project and minute are numbered", () => {
    const rows = recentRows([session("a"), session("b")], PROJECTS, new Map(), NOW);
    expect(rows.map((row) => row.title)).toEqual([`payments-api · started ${clock(NOW - 7_200_000)} (1)`, `payments-api · started ${clock(NOW - 7_200_000)} (2)`]);
  });
  it("never shows two rows alike", () => {
    const rows = recentRows(
      [session("a", { title: "Fix it" }), session("b", { title: "Fix it", account_label: "Personal" }), session("c", { title: "Fix it", account_label: "Personal", started_at_ms: NOW - 50_000_000 })],
      PROJECTS,
      new Map(),
      NOW,
    );
    expect(new Set(rows.map((row) => row.title)).size).toBe(3);
    expect(rows[0]!.title).toBe("Fix it · Work Max");
  });
});

describe("handoverTeaser", () => {
  it("says how many can take it, and the suggestion", () => {
    const target = { recipe_id: "r", account_id: "a", account_label: "Personal", badge: "", validated: false, cross_provider: false, drops: [], score: 1, filtered_reason: null, top_factors: ["80% headroom"], factors: [], confidence: "", tte_min: null, suggested: true };
    expect(handoverTeaser([target, { ...target, account_label: "Work", suggested: false }])).toBe("2 apps can take it over. Suggested: Personal, 80% headroom.");
    expect(handoverTeaser([])).toBe("Add another app to juggle between them.");
    expect(handoverTeaser(null)).toBe("Ranking the other apps…");
  });
  it("says room it hasn't measured is unmeasured, not short (walk A7)", () => {
    const target = { recipe_id: "r", account_id: "a", account_label: "Personal", badge: "", validated: false, cross_provider: false, drops: [], score: 1, filtered_reason: null, top_factors: [], factors: [], confidence: "", tte_min: null, suggested: false };
    const unknown = { left: null, kind: "unknown" as const, words: "Not measured yet", resets: null, resetsAtMs: null, credits: null };
    expect(handoverTeaser([target, { ...target, account_label: "Work" }])).toBe("2 apps can take it over; their room isn't measured yet.");
    expect(handoverTeaser([{ ...target, room: unknown }])).toBe("1 app can take it over; their room isn't measured yet.");
    expect(handoverTeaser([{ ...target, room: unknown }, { ...target, room: { ...unknown, kind: "blocked" as const } }])).toBe("2 apps can take it over; their room isn't measured yet.");
    expect(handoverTeaser([{ ...target, room: { ...unknown, left: 3, kind: "low" as const, words: "3% left" } }])).toBe("1 app can take it over, but none of their accounts has much room left now.");
  });
});
