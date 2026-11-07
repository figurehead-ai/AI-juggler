/**
 * L3 (the 0.4 validation plan, §4; AJ-E3304): New pool, one journey per
 * row of the all-pairs array `docs/qa/pairwise/wizard.csv`, so every pair
 * of template, scope, member kind, capability, role, level, a member's own
 * level, a pair's rule, sealing, theme and width is walked at least once.
 *
 * Each row: Pools › New pool › the template (or Set up by hand, which
 * opens the canvas to set up there); Where (every project, one, several,
 * or sessions with no project); Who: a member of the row's kind (a whole
 * App, Account, Tool, Mac or everyone joins later) and, beside it, a
 * session of the row's capability (full tools, shell only, watched); "not
 * matched": a session the pool can't take as it is shows greyed, with its
 * reason and its fix. Roles and talk: the member's role and its own level
 * (or the pool's), the pool's level, one talk cell changed ("one-pair"),
 * sealed or not. The review says it; Make makes it with the user's own
 * calls and opens its canvas; a "custom" level is set there, in the
 * header (the wizard offers Read, Work together and Full). A pair of two
 * members set apart is the canvas table's (`canvas.csv`, edge overridden).
 * Every click's feedback is timed (≤ 100 ms), written to
 * `docs/qa/evidence/l3/wizard.json`.
 */
import { type Page } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "./axe-gallery";

const T0 = new Date(2026, 10, 7, 15, 0, 0).getTime();
const ROWS = readFileSync(resolve(process.cwd(), "../docs/qa/pairwise/wizard.csv"), "utf8")
  .trim()
  .split("\n")
  .slice(1)
  .map((line) => {
    const [row, template, scope, member, capability, role, level, own_level, pair_override, sealed, theme, width] = line.split(",");
    return { row: Number(row), template, scope, member, capability, role, level, own_level, pair_override, sealed, theme: theme as "dark" | "light", width: Number(width) };
  });

/** The table's template names, as the wizard's cards are keyed. */
const TEMPLATE: Record<string, string> = { "writer-and-reviewer": "writer-reviewer" };
const KIND: Record<string, RegExp> = { session: /^Sessions/, app: /^Apps/, account: /^Accounts/, tool: /^Tools/, mac: /^Macs/, "everyone-in-scope": /^Everyone/ };
const REACH: Record<string, string> = { full: "full tools", shell: "shell only", watched: "watched" };
const ROLE: Record<string, string> = { lead: "Lead", member: "Member", reviewer: "Reviewer", observer: "Observer" };
const LEVEL: Record<string, string> = { read: "Read", "work-together": "Work together", full: "Full", custom: "Work together" };

const folder = (id: string, provider: string, label: string) => ({ id, provider, label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const project = (id: string, name: string) => ({ id, root: `/Users/you/Dev/${name}`, name, last_activity_ms: T0 - 60_000, active_instance: null, accounts_used: [], instance_count: 2, history_sessions: 2, history_last_used_ms: null, repo: null, missing: false, pinned: false });
function session(id: string, provider: string, account: string, branch: string, projectId: string | null, status = "running") {
  return {
    id, adopted: false, launch_recipe_id: null, provider, account_id: account, account_label: account, model: null, launch_model: null, running_model: null, model_source: "launch",
    status, quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, subtask_count: null, git_branch: branch, git_dirty_count: null,
    needs_attention: null, attention: null, started_at_ms: T0 - 1_200_000, last_activity_ms: status === "running" ? T0 - 30_000 : T0 - 3_600_000, machine_id: "this-mac",
    cwd: projectId ? `/Users/you/Dev/${projectId.replace("proj-", "")}` : "/Users/you/scratch", kind: "terminal", isolation_label: null, project_id: projectId, conversation_id: null,
  };
}
function watched(id: string, title: string, projectId: string | null) {
  return {
    id, tool: "claude-code", account_id: "claude-work", account_label: "Work Max", project_id: projectId, cwd: projectId ? `/Users/you/Dev/${projectId.replace("proj-", "")}` : "/Users/you/scratch", title,
    git_branch: "main", started_ms: T0 - 3_600_000, last_activity_ms: T0 - 60_000, tokens: null, live: true, session_id: null, pid: 77, state: "working", probably: false,
    viewed_ms: null, background: false, inactive: false, subtask_count: null,
    places: [{ app: "Claude", shown: "Claude app", app_id: "claude-app", profile: "Work", instance: "Work", pid: 77, kind: "desktop", app_pid: 78, label: "Claude · Work" }],
  };
}
const FIXTURE = {
  accounts: [folder("claude-work", "claude-code", "Work Max"), folder("kimi-home", "kimi-code", "Kimi home")],
  projects: [project("proj-ai-juggler", "ai-juggler"), project("proj-payments", "payments-api"), project("proj-docs", "docs-site")],
  sessions: [
    session("s-full", "claude-code", "claude-work", "feat/full-tools", "proj-ai-juggler"),
    session("s-shell", "kimi-code", "kimi-home", "fix/shell-only", "proj-ai-juggler"),
    session("s-full-np", "claude-code", "claude-work", "spike/no-project", null),
    session("s-shell-np", "kimi-code", "kimi-home", "spike/shell-np", null),
    session("s-pay", "claude-code", "claude-work", "feat/payments", "proj-payments"),
    session("s-docs", "claude-code", "claude-work", "docs/guide", "proj-docs"),
    session("s-ended", "claude-code", "claude-work", "old/ended", "proj-ai-juggler", "stopped"),
  ],
  conversations: [watched("w-1", "Watched in the Claude app", "proj-ai-juggler"), watched("w-np", "Watched with no project", null), watched("w-pay", "Watched on payments", "proj-payments")],
};

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
  writeFileSync(resolve(out, "wizard.json"), JSON.stringify({ table: "docs/qa/pairwise/wizard.csv", rows: ROWS.length, clicks: all.length, feedback_p95_ms: p95, feedback_max_ms: all.at(-1) ?? null, budget_ms: 100, per_row_ms: times }, null, 2) + "\n");
});

/** Picks the first row that can join, of the kind shown, whose reach says `reach` (any when null). */
async function pick(page: Page, reach: string | null): Promise<string> {
  const rows = page.getByTestId("new-pool-candidate").filter({ has: page.locator("button[role=checkbox]:not([disabled])") });
  const row = reach ? rows.filter({ hasText: reach }).first() : rows.first();
  await expect(row).toBeVisible();
  const label = (await row.locator("button span.flex-1 span").first().textContent()) ?? "";
  await row.getByRole("checkbox").click();
  await expect(row.getByRole("checkbox")).toHaveAttribute("aria-checked", "true");
  return label.trim();
}

for (const r of ROWS) {
  test(`L3 New pool row ${r.row}: ${r.template}, ${r.scope}, ${r.member}, ${r.capability}, ${r.role}, ${r.level}, own ${r.own_level}, ${r.pair_override}, sealed ${r.sealed}, ${r.theme} ${r.width}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme, t0 }) => {
        const w = window as unknown as Record<string, unknown>;
        Object.assign(w, { __AIJ_FIXTURE__: fixture, __AIJ_RANKING__: [], __AIJ_ADVICE__: [] });
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        localStorage.setItem("aij.walk-timing", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: FIXTURE, theme: r.theme, t0: T0 },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: r.theme });
    await page.setViewportSize({ width: r.width, height: r.width === 1280 ? 800 : 700 });
    await page.goto("/");
    await expect(page.getByTestId("rail")).toBeVisible();
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "pools" } })));
    await page.getByRole("button", { name: "New pool" }).first().click();
    const wizard = page.getByTestId("new-pool");
    await expect(wizard).toBeVisible();

    // By hand: the canvas, to set it up there (its own table's).
    if (r.template === "by-hand") {
      await page.getByTestId("new-pool-by-hand").click();
      await expect(wizard).toHaveCount(0);
      await expect(page.getByTestId("pool-canvas")).toBeVisible();
      times[r.row] = (await walkSteps(page)).map((ms) => Math.round(ms));
      return;
    }

    // 1. The template.
    await wizard.locator(`[data-testid=new-pool-template][data-template="${TEMPLATE[r.template] ?? r.template}"]`).click();
    await page.getByTestId("new-pool-next").click();

    // 2. Where.
    const where = page.getByTestId("new-pool-where");
    if (r.scope === "every-project") await where.getByText("Every project").click();
    else {
      await where.getByText("These projects").click();
      const chips = page.getByTestId("new-pool-projects").getByRole("button", { name: /^Take .* out$/ });
      while ((await chips.count()) > 0) await chips.first().click();
      const add = async (name: string) => {
        await page.getByTestId("new-pool-add-project").click();
        await page.getByRole("menu").last().getByText(name, { exact: true }).click();
      };
      if (r.scope === "one-project") await add("ai-juggler");
      else if (r.scope === "several-projects") {
        await add("ai-juggler");
        await add("payments-api");
      } else await add("Sessions with no project");
    }
    const side = page.getByTestId("new-pool-side");
    await expect(side).toContainText(r.scope === "every-project" ? "Every project" : r.scope === "one-project" ? "ai-juggler" : r.scope === "several-projects" ? "payments-api" : /no project/i);

    // Who: the row's kind; then a session of the row's capability.
    const kinds = page.getByTestId("new-pool-kind");
    let label = "";
    if (r.member === "not-matched") {
      // A session it can't take as it is: greyed, its reason and its fix.
      await kinds.getByRole("radio", { name: KIND.session! }).click();
      const blocked = page.getByTestId("new-pool-candidate").filter({ has: page.locator("button[role=checkbox][disabled]") });
      await expect(blocked.first()).toBeVisible();
      await expect(blocked.first()).toContainText(/Ended|not in this pool's projects|sealed/);
      await expect(blocked.first().getByTestId("new-pool-fix")).toBeVisible();
    } else if (r.member !== "session") {
      await kinds.getByRole("radio", { name: KIND[r.member]! }).click();
      label = await pick(page, "new ones join");
    }
    await kinds.getByRole("radio", { name: KIND.session! }).click();
    const reachLabel = await pick(page, REACH[r.capability]!);
    if (r.member === "session" || r.member === "not-matched") label = reachLabel;
    await expect(side.getByText(new RegExp(`^${r.capability === "full" ? "\\d+ full" : r.capability === "shell" ? "\\d+ shell only" : "\\d+ watched"}$`))).toBeVisible();
    await page.getByTestId("new-pool-next").click();

    // 3. Roles and talk.
    const roles = page.getByTestId("new-pool-roles").locator("li").filter({ hasText: label }).first();
    await roles.getByRole("radio", { name: ROLE[r.role]!, exact: true }).click();
    await expect(roles.getByRole("radio", { name: ROLE[r.role]!, exact: true })).toHaveAttribute("aria-checked", "true");
    await page.getByTestId("new-pool-level").getByRole("radio", { name: LEVEL[r.level]!, exact: true }).click();
    if (r.own_level === "member") {
      const own = r.level === "full" ? "Level: Read" : "Level: Full";
      await roles.getByRole("button", { name: `${label}'s level` }).click();
      await page.getByRole("menu").last().getByText(own, { exact: true }).click();
      await expect(roles.getByRole("button", { name: `${label}'s level` })).toContainText(own.replace("Level: ", ""));
    }
    let changed: string | null = null;
    if (r.pair_override === "one-pair") {
      const cell = page.locator('[data-testid=new-pool-talk-cell][data-pair="member>member"]');
      const before = (await cell.textContent()) ?? "";
      await cell.click();
      changed = (await cell.textContent()) ?? "";
      expect(changed).not.toBe(before);
    }
    if (r.sealed === "on") {
      await page.getByTestId("new-pool-seal").getByRole("checkbox").check();
      await expect(page.getByTestId("new-pool-seal").getByRole("checkbox")).toBeChecked();
    }
    await page.getByTestId("new-pool-next").click();

    // 4. The review says it; Make makes it with the user's own calls.
    const review = page.getByTestId("new-pool-review");
    await expect(review).toBeVisible();
    // The changed cell, in the review's words: named as an exception, or
    // summed up when it makes every pair the same ("Everyone may message
    // everyone").
    if (changed) await expect(review).toContainText(new RegExp(`Member to Member: ${changed.toLowerCase()}|Everyone may message everyone|No one may message|asks you first`, "i"));
    const name = await page.getByTestId("new-pool-name").inputValue();
    expect(name.trim().length).toBeGreaterThan(0);
    await page.getByTestId("new-pool-next").click();
    await expect(wizard).toHaveCount(0);
    await expect(page.getByTestId("pool-page")).toBeVisible();
    const made = await calls(page);
    expect(made).toContain(`create:${name.trim()}`);
    const id = made.find((call) => call.startsWith("scope:"))?.split(":")[1] ?? "";
    expect(made.some((call) => call.startsWith(`scope:${id}:`) && (r.scope === "every-project" ? call.endsWith(":every") : !call.endsWith(":every")))).toBe(true);
    expect(made.filter((call) => call.startsWith(`join:${id}:`)).length).toBeGreaterThanOrEqual(r.member === "session" || r.member === "not-matched" ? 1 : 2);
    expect(made.some((call) => call.startsWith(`join:${id}:`) && call.includes(`:${r.role}:`))).toBe(true);
    if (r.sealed === "on") expect(made).toContain(`sealed:${id}:true`);
    if (changed) expect(made.some((call) => call.startsWith(`talk:${id}:`))).toBe(true);

    // A custom level is the pool's own, on its canvas.
    if (r.level === "custom") {
      await page.getByTestId("pool-header-level").click();
      await page.getByRole("menu").last().getByText("Custom", { exact: true }).click();
      await expect.poll(() => calls(page)).toContainEqual(expect.stringMatching(new RegExp(`^access:${id}:custom:`)));
    }

    const steps = await walkSteps(page);
    times[r.row] = steps.map((ms) => Math.round(ms));
    expect(Math.max(0, ...steps), `row ${r.row}: every click's feedback ≤ 100 ms (${steps.join(", ")})`).toBeLessThanOrEqual(100);
  });
}
