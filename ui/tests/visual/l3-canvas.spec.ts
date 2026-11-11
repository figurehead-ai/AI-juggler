/**
 * L3 (the 0.4 validation plan, §4; AJ-E3304): a pool's canvas, one journey
 * per row of the all-pairs array `docs/qa/pairwise/canvas.csv`, so every
 * pair of member kind, member count, a line's rule (its role pair's or its
 * own), the rule, sealing, how many pools a session is in, pointer or
 * keyboard, theme and width is walked at least once.
 *
 * Each row: a pool of `members` sessions (the first its lead; that one
 * also in 0, 1 or 2 other pools), its role pair Member → Member set as
 * the wizard's grid would; opened from its card on Pools (the canvas
 * chosen where a narrow window opens it as rows). The row's member kind
 * joins from the palette (a session is already there; an App, Account,
 * Tool, Mac or everyone joins later; a session in another app as
 * watched). The line between the first two: its rule shown, the pair's
 * ("This line follows it") or set for that line only. Sealing: it says
 * first who leaves which pool, and seals on Seal it; unsealed rows read
 * the same words and leave it. Keyboard-only rows do all of it with focus
 * and keys, no pointer. Feedback timed (≤ 100 ms) to
 * `docs/qa/evidence/l3/canvas.json`. With fewer than two members there's
 * no line: those rows check there's none and nothing to set.
 */
import { type Locator, type Page } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "./axe-gallery";

const T0 = new Date(2026, 10, 7, 15, 0, 0).getTime();
const ROWS = readFileSync(resolve(process.cwd(), "../docs/qa/pairwise/canvas.csv"), "utf8")
  .trim()
  .split("\n")
  .slice(1)
  .map((line) => {
    const [row, member, members, edge, talk, sealed, pools_per_session, input, theme, width] = line.split(",");
    return { row: Number(row), member, members: Number(members), edge, talk, sealed, pools: pools_per_session === "3+" ? 3 : Number(pools_per_session), input, theme: theme as "dark" | "light", width: Number(width) };
  });

const POOL = "pool-row";
const RULE: Record<string, string> = { allowed: "Allowed", "ask-first": "Ask first", blocked: "Blocked" };
const WIRE: Record<string, string> = { allowed: "allowed", "ask-first": "ask", blocked: "blocked" };
const MORE: Record<string, RegExp> = { app: /^Apps/, account: /^Accounts/, tool: /^Tools/, mac: /^Macs/, "everyone-in-scope": /^Everyone/, watched: /^Sessions in other apps/ };

const name = (index: number) => `Agent ${String(index + 1).padStart(2, "0")}`;
const sid = (index: number) => `s-${String(index + 1).padStart(2, "0")}`;
function session(id: string, label: string) {
  return {
    id, adopted: false, launch_recipe_id: "p-1", provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null, running_model: null, model_source: "launch",
    status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, subtask_count: null, git_branch: null, git_dirty_count: null, needs_attention: null, attention: null,
    started_at_ms: T0 - 3_600_000, last_activity_ms: T0 - 60_000, machine_id: "this-mac", cwd: "/Users/you/Dev/web-shop", kind: "terminal", isolation_label: null, project_id: "proj-web", conversation_id: null, label,
  };
}
function seed(r: (typeof ROWS)[number]) {
  const ids = Array.from({ length: r.members }, (_, index) => sid(index));
  // The pair's own rule: the row's, or (to set this line apart) another.
  const pair = r.edge === "overridden" ? (r.talk === "allowed" ? "blocked" : "allowed") : WIRE[r.talk]!;
  const others = Array.from({ length: Math.max(0, r.pools - 1) }, (_, index) => ({ id: `pool-other-${index + 1}`, name: `Other ${"AB"[index]}`, project_id: "proj-web" }));
  return {
    __AIJ_FIXTURE__: {
      accounts: [{ id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false }],
      projects: [{ id: "proj-web", root: "/Users/you/Dev/web-shop", name: "web-shop", last_activity_ms: T0 - 60_000, active_instance: null, accounts_used: ["Work Max"], instance_count: 1, history_sessions: 1, history_last_used_ms: null, repo: null, missing: false, pinned: false }],
      sessions: [...ids.map((id, index) => session(id, name(index))), session("s-free", "Not in the pool")],
      conversations: [
        {
          id: "w-1", tool: "claude-code", account_id: "claude-work", account_label: "Work Max", project_id: "proj-web", cwd: "/Users/you/Dev/web-shop", title: "Watched in the Claude app", git_branch: "main",
          started_ms: T0 - 3_600_000, last_activity_ms: T0 - 60_000, tokens: null, live: true, session_id: null, pid: 77, state: "working", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null,
          places: [{ app: "Claude", shown: "Claude app", app_id: "claude-app", profile: "Work", instance: "Work", pid: 77, kind: "desktop", app_pid: 78, label: "Claude · Work" }],
        },
      ],
    },
    __AIJ_POOLS__: {
      pools: [{ id: POOL, name: "Row pool", project_id: "proj-web" }, ...others],
      members: [
        ...ids.map((id, index) => ({ session_id: id, pool_id: POOL, role: index === 0 ? "lead" : "member" })),
        ...(ids[0] ? others.map((other) => ({ session_id: ids[0]!, pool_id: other.id, role: "member" })) : []),
      ],
    },
    __AIJ_POOL_RULES__: { [POOL]: { talk: [{ from: "role:member", to: "role:member", rule: pair }] } },
    __AIJ_POOL_CANVAS__: { [POOL]: { edges: ids.length >= 2 ? [{ from_session_id: ids[0], to_session_id: ids[1], messages: 1, tasks: 0, last_at_ms: T0 - 120_000, thread_id: null, pending: [] }] : [], stream: [] } },
    __AIJ_MESSAGES__: { threads: [], messages: {}, needs: [] },
    __AIJ_AGENT_BRIDGE__: { asked: true, on: true },
  };
}

async function walkSteps(page: Page): Promise<number[]> {
  const all = await page.evaluate(() => [...((window as unknown as { __AIJ_WALK__?: { feedbackMs: number | null }[] }).__AIJ_WALK__ ?? [])]);
  return all.map((step) => step.feedbackMs).filter((ms): ms is number => ms !== null);
}
const calls = (page: Page) => page.evaluate(() => [...((window as unknown as { __AIJ_POOL_CALLS__?: string[] }).__AIJ_POOL_CALLS__ ?? [])]);

const times: Record<number, number[]> = {};
test.afterAll(() => {
  if (Object.keys(times).length !== ROWS.length) return;
  const out = resolve(process.cwd(), "../docs/qa/evidence/l3");
  mkdirSync(out, { recursive: true });
  const all = Object.values(times).flat().sort((a, b) => a - b);
  const p95 = all.length ? all[Math.ceil(all.length * 0.95) - 1]! : null;
  writeFileSync(resolve(out, "canvas.json"), JSON.stringify({ table: "docs/qa/pairwise/canvas.csv", rows: ROWS.length, clicks: all.length, feedback_p95_ms: p95, feedback_max_ms: all.at(-1) ?? null, budget_ms: 100, per_row_ms: times }, null, 2) + "\n");
});

for (const r of ROWS) {
  test(`L3 canvas row ${r.row}: ${r.member}, ${r.members} members, ${r.edge}, ${r.talk}, sealed ${r.sealed}, in ${r.pools} pool(s), ${r.input}, ${r.theme} ${r.width}`, async ({ page }) => {
    test.setTimeout(90_000);
    // Keyboard-only: focus and a key, never the pointer.
    const keys = r.input === "keyboard-only";
    const act = async (target: Locator, key = "Enter") => {
      if (keys) {
        // Focus held (a list redrawn between focus and key would take it).
        await expect(async () => {
          await target.focus();
          await expect(target).toBeFocused({ timeout: 300 });
        }).toPass();
        await page.keyboard.press(key);
      } else await target.click();
    };
    await page.addInitScript(
      ({ seeded, theme, t0 }) => {
        Object.assign(window, seeded);
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        localStorage.setItem("aij.walk-timing", "1");
        localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { seeded: seed(r), theme: r.theme, t0: T0 },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: r.theme });
    await page.setViewportSize({ width: r.width, height: r.width === 1280 ? 800 : 700 });
    await page.goto("/");
    await expect(page.getByTestId("rail")).toBeVisible();
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "pools" } })));
    const tile = page.getByTestId("pool-card-tile").filter({ hasText: "Row pool" }).first();
    await expect(tile).toBeVisible();
    await page.waitForTimeout(300); // the cards settle (their counts read)
    await act(tile);
    await expect(page.getByTestId("pool-page")).toBeVisible();
    // A narrow window opens it as rows: the canvas, chosen.
    if ((await page.getByTestId("pool-canvas").count()) === 0) await act(page.getByTestId("pool-mode-canvas"));
    await expect(page.getByTestId("pool-canvas")).toBeVisible();
    await expect(page.getByTestId("canvas-node")).toHaveCount(r.members);

    // The row's member kind joins from the palette.
    if (r.member !== "session") {
      const palette = page.getByTestId("canvas-palette");
      const fold = palette.getByRole("button", { name: /Add a session/ });
      if ((await fold.getAttribute("aria-expanded")) === "false") await act(fold);
      const kind = page.getByTestId("canvas-palette-kind");
      await act(kind);
      await act(page.getByRole("menu").last().getByRole("menuitemradio", { name: MORE[r.member]! }).first());
      const add = page.getByTestId("canvas-palette-more").getByRole("button", { name: /^Add/ }).first();
      await act(add);
      await expect(page.getByTestId("canvas-added-toast")).toBeVisible();
      if (r.member === "watched") await expect(page.getByTestId("canvas-watched-node")).toHaveCount(1);
      else await expect(page.getByTestId("canvas-rule-node").first()).toBeVisible();
      // Folded again, as you would to reach a node under it.
      if ((await fold.getAttribute("aria-expanded")) === "true") await act(fold);
    }

    // Sealing says first who leaves which pool.
    // (First, while the inspector shows the pool: its settings.)
    const seal = page.getByTestId("pool-seal");
    await act(page.getByTestId("inspect-open-settings"));
    await expect(seal).toBeVisible();
    await act(seal.getByTestId("pool-seal-open"));
    const confirm = page.getByTestId("pool-seal-confirm");
    await expect(confirm).toBeVisible();
    if (r.members >= 1 && r.pools > 1) await expect(confirm.getByTestId("pool-seal-changes")).toContainText(`${name(0)} leaves Other A${r.pools > 2 ? ", Other B" : ""}`);
    if (r.sealed === "on") {
      await act(confirm.getByTestId("pool-seal-yes"));
      await expect.poll(() => calls(page)).toContainEqual(`sealed:${POOL}:true`);
      await expect(seal).toContainText("Sealed:");
    }

    // The line between the first two: its rule, the pair's or its own.
    if (r.members >= 2) {
      await act(page.locator(`[data-node-name="${sid(0)}"]`).first());
      const lines = page.getByTestId("inspect-node-edges");
      await expect(lines).toBeVisible();
      await act(lines.getByRole("button", { name: new RegExp(name(1)) }).first());
      const talk = page.getByTestId("edge-talk");
      await expect(talk).toBeVisible();
      const rule = talk.getByTestId("edge-talk-rule");
      if (r.edge === "overridden") {
        await expect(talk).toContainText("This line follows it");
        await act(rule.getByRole("radio", { name: RULE[r.talk]!, exact: true }));
        await expect(talk).toContainText("This line has its own rule");
        await expect.poll(() => calls(page)).toContainEqual(`talk:${POOL}:session:${sid(0)}:session:${sid(1)}:${WIRE[r.talk]}`);
      } else {
        await expect(talk).toContainText("This line follows it");
      }
      await expect(rule.getByRole("radio", { name: RULE[r.talk]!, exact: true })).toHaveAttribute("aria-checked", "true");
    } else {
      // No pair, no line.
      await expect(page.getByTestId("edge-talk")).toHaveCount(0);
    }

    const steps = await walkSteps(page);
    times[r.row] = steps.map((ms) => Math.round(ms));
    expect(Math.max(0, ...steps), `row ${r.row}: every click's feedback ≤ 100 ms (${steps.join(", ")})`).toBeLessThanOrEqual(100);
  });
}
