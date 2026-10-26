/**
 * The review of 0.1.5: Home's "Pick up where you left off" rows all read
 * "Untitled session · payments-api". An untitled session takes the first
 * words of the last thing asked, else its branch or project and when it
 * started; no two rows read the same.
 */
import { describe, expect, it } from "vitest";
import { firstWords, startedWords } from "./session-titles";
import { recentRows } from "../components/HomeRecent";
import type { InstanceInfo, ProjectInfo } from "./bridge";
import { handoverTeaser } from "../components/HandoverSection";

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
  it("without words: its project and when it started, never 'Untitled'", () => {
    const [row] = recentRows([session("a")], PROJECTS, new Map(), NOW);
    expect(row!.title).toMatch(/^payments-api · started /);
    expect(row!.title).not.toMatch(/Untitled/);
    expect(row!.where).toBe("main · Work Max");
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
    expect(handoverTeaser([target, { ...target, account_label: "Work", suggested: false }])).toBe("2 accounts can take it over. Suggested: Personal, 80% headroom.");
    expect(handoverTeaser([])).toBe("Add another account to juggle between them.");
    expect(handoverTeaser(null)).toBe("Ranking the other accounts…");
  });
});
