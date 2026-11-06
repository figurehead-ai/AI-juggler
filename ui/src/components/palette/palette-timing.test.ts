/**
 * AJ-E2901 timing: ⌘K's rows on 500 sessions, 200 projects, 20 accounts
 * and 30 apps — the first build (open) and each keystroke's re-rank. The
 * budget is 50 ms to open and p95 ≤ 16 ms a keystroke; the test fails
 * only past a margin for a shared machine, and prints the numbers as one
 * `AIJ_PALETTE_TIMING {…}` line when `AIJ_PALETTE_TIMING` is set (the
 * evidence's timing.json).
 */
import { describe, expect, it } from "vitest";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { AppModel } from "../../lib/apps";
import { parseQuery } from "../../lib/rail-view";
import { buildGroups, type AccountRow, type PaletteInputs } from "./rows";

const NOW = Date.now();
const WORDS = ["billing", "export", "login", "refactor", "settings", "dashboard", "release", "notes", "search", "palette", "cache", "floor"];
const word = (n: number) => WORDS[n % WORDS.length]!;

function fixture(): Omit<PaletteInputs, "typed" | "query"> {
  const sessions = Array.from({ length: 500 }, (_, n) =>
    ({ id: `s${n}`, provider: n % 3 ? "claude-code" : "codex", account_id: `a${n % 20}`, account_label: `Account ${n % 20}`, status: n % 4 ? "running" : "finished", kind: n % 5 ? "terminal" : "desktop", git_branch: `${word(n)}-${n}`, cwd: `/w/p${n % 200}`, project_id: `p${n % 200}`, conversation_id: null, quota: null, label: `${word(n)} ${word(n + 3)} ${n}`, last_activity_ms: NOW - n * 1000 }) as unknown as InstanceInfo,
  );
  const projects = Array.from({ length: 200 }, (_, n) => ({ id: `p${n}`, name: `${word(n + 5)}-${n}`, root: `/w/${word(n + 5)}-${n}`, missing: false, last_activity_ms: NOW - n }) as unknown as ProjectInfo);
  const apps = Array.from({ length: 30 }, (_, n) => ({ id: `app${n}`, kind: n % 2 ? "cli" : "desktop", tool: "claude-code", appId: null, app: n % 2 ? "Claude Code" : "Claude app", profile: `Profile ${n}`, name: `App ${word(n)} ${n}`, defaultName: `Claude · Profile ${n}`, named: false, railKey: `k${n}`, folder: null, accountId: null, place: null, slot: 0 }) as AppModel);
  const accounts = Array.from({ length: 20 }, (_, n) => ({ key: `k${n}`, accountId: `acc${n}`, name: `Account ${word(n)} ${n}`, tool: "Claude Code", room: `${n * 5}% left in the 5-hour window`, folder: null, subscription: { plan: "Max" } }) as unknown as AccountRow);
  return { actions: [], selectedHeading: null, commands: [], sessions, conversations: [], remoteMachines: [], projects, apps, accounts, recents: [], leftovers: 0, unread: 0, settings: [] };
}

const time = (run: () => void) => {
  const start = performance.now();
  run();
  return performance.now() - start;
};

describe("AJ-E2901 palette timing", () => {
  it("opens and re-ranks within budget on the big fixture", () => {
    const base = fixture();
    const build = (typed: string) => buildGroups({ ...base, typed, query: parseQuery(typed) });
    build(""); // warm up the JIT
    const open = Math.max(...Array.from({ length: 5 }, () => time(() => build(""))));
    const typed = ["b", "bi", "bil", "bill", "billi", "wrk", "wrk b", "wrk bil", "exp", "export", "ref set", "dash", "notes 4", "pal", "cache", "flo", "is:ended login", "zz", "p1", "account 7"];
    const keystrokes = typed.map((text) => time(() => build(text))).sort((a, b) => a - b);
    const p95 = keystrokes[Math.ceil(keystrokes.length * 0.95) - 1]!;
    const env = (globalThis as unknown as { process?: { env: Record<string, string | undefined> } }).process?.env;
    if (env?.AIJ_PALETTE_TIMING) {
      const record = { fixture: { sessions: 500, projects: 200, accounts: 20, apps: 30 }, open_ms: Number(open.toFixed(2)), keystroke_p95_ms: Number(p95.toFixed(2)), keystroke_max_ms: Number(keystrokes.at(-1)!.toFixed(2)), keystrokes: typed.length };
      console.log(`AIJ_PALETTE_TIMING ${JSON.stringify(record)}`);
    }
    expect(open).toBeLessThan(150);
    expect(p95).toBeLessThan(48);
  });
});
