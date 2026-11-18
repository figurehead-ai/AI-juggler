/**
 * L3 (the 0.4 validation plan, §4; AJ-E3304): New pool, one journey per
 * row of the all-pairs array `docs/qa/pairwise/wizard.csv`, so every pair
 * of template, scope, member kind, capability, role, level, a member's own
 * level, a pair's rule, sealing, theme and width is walked at least once.
 *
 * Each row walks the setup in its order (AJ-E3011, D-126): How to start
 * (the template, or Start from scratch for the table's "by hand"); Where
 * (every project, one, several, or sessions with no project, each a
 * card); Who's in: a member of the row's kind (a whole App, Account, Tool
 * or Mac; or every new session, its switch) and, beside it, a session of
 * the row's capability (full tools, shell only, watched); "not matched":
 * a session the pool can't take as it is shows greyed, with its reason and
 * its fix; Who leads (a session, an App's next session, or none); Who
 * builds, who checks: the member's role and its own choice (or the
 * pool's); Who may message whom: one cell changed ("one-pair"); What they
 * may do on their own, and the seal. The review says it; Make makes it
 * with the user's own calls and opens its canvas; a "custom" choice is set
 * there, in the header (the setup offers the three). A pair of two
 * members set apart is the canvas table's (`canvas.csv`, edge overridden).
 * Every click's feedback is timed (≤ 100 ms), written to
 * `docs/qa/evidence/l3/wizard.json`.
 */
import { type Locator, type Page } from "@playwright/test";
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
// D-126: everyone in scope is Who's in's switch, not a kind.
const KIND: Record<string, RegExp> = { session: /^Sessions/, app: /^Apps/, account: /^Accounts/, tool: /^Tools/, mac: /^Macs/ };
const REACH: Record<string, string> = { full: "full tools", shell: "shell only", watched: "watched" };
// AJ-E3011: the words for what sessions may do on their own ("Level" is gone).
const LEVEL: Record<string, string> = { read: "Look only", "work-together": "Work together", full: "Work together and wake each other", custom: "Work together" };
/** D-126: each of them a card, by its level's id. */
const LEVEL_ID: Record<string, string> = { read: "read", "work-together": "work_together", full: "full", custom: "work_together" };

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

/** AJ-E3011 round 2 (#8): Apps and Sessions are segments; the rarer
 * kinds are under More, picked from its menu. */
async function chooseKind(page: Page, member: string): Promise<void> {
  const kinds = page.getByTestId("new-pool-kind");
  if (member === "session" || member === "app") return kinds.getByRole("radio", { name: KIND[member]! }).click();
  await kinds.getByRole("radio", { name: "More" }).click();
  await page.getByTestId("new-pool-kind-picker").click();
  await page.getByRole("menuitemradio", { name: KIND[member]! }).click();
}

/** Picks the first row that can join, of the kind shown, whose reach
 * says `reach` (any when null): its ref, and its name (an App's card
 * names it in two lines, so its name is read where Who leads offers it). */
async function pick(page: Page, reach: string | null): Promise<{ ref: string; label: string }> {
  // D-126: an App is a logo card (a checkbox in its label); the other
  // kinds are rows (a checkbox button).
  const rows = page.getByTestId("new-pool-candidate").filter({ has: page.locator("button[role=checkbox]:not([disabled]), input[type=checkbox]:not([disabled])") });
  const row = reach ? rows.filter({ hasText: reach }).first() : rows.first();
  const target = row.locator("label, button[role=checkbox]").first();
  await expect(target).toBeVisible();
  const ref = (await row.getAttribute("data-ref")) ?? "";
  const card = (await row.locator("label").count()) > 0;
  const label = card ? "" : ((await row.locator("button span.flex-1 span").first().textContent()) ?? "");
  if (!(await row.getByRole("checkbox").isChecked())) await target.click();
  await expect(row.getByRole("checkbox")).toBeChecked();
  return { ref, label: label.trim() };
}

/** A choice card (a checkbox in its label) set on or off. */
async function setCard(card: Locator, on: boolean): Promise<void> {
  if ((await card.getByRole("checkbox").isChecked()) !== on) await card.click();
  await expect(card.getByRole("checkbox")).toBeChecked({ checked: on });
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

    // AJ-E3011: "Set up by hand" became Start from scratch, the first card.
    const template = r.template === "by-hand" ? "blank" : (TEMPLATE[r.template] ?? r.template);
    // 1. How to start.
    await wizard.locator(`[data-testid=new-pool-template][data-template="${template}"]`).click();
    await page.getByTestId("new-pool-next").click();

    // 2. Where. D-126: each project a card (checked when it's in), then
    // Sessions with no project and Every project.
    const where = page.getByTestId("new-pool-where");
    const every = where.getByTestId("new-pool-project-every");
    if (r.scope === "every-project") await setCard(every, true);
    else {
      await setCard(every, false);
      // Start empty: the project a template starts in comes out first.
      const chosen = where.locator('[data-testid="new-pool-project"]:has(input:checked), [data-testid="new-pool-project-none"]:has(input:checked)');
      while ((await chosen.count()) > 0) await chosen.first().click();
      const project = (name: string) => where.getByTestId("new-pool-project").filter({ hasText: name });
      if (r.scope === "one-project") await setCard(project("ai-juggler"), true);
      else if (r.scope === "several-projects") {
        await setCard(project("ai-juggler"), true);
        await setCard(project("payments-api"), true);
      } else await setCard(where.getByTestId("new-pool-project-none"), true);
    }
    // The preview beside says where, as the steps will.
    const previewValue = (row: string) => page.locator(`[data-testid="new-pool-preview-row"][data-row="${row}"] [data-testid="new-pool-preview-value"]`);
    await expect(previewValue("where")).toContainText(r.scope === "every-project" ? "every project" : r.scope === "one-project" ? "ai-juggler" : r.scope === "several-projects" ? "payments-api" : /no project/i);
    await page.getByTestId("new-pool-next").click();

    // Who: the row's kind; then a session of the row's capability.
    let label = "";
    let member = { ref: "", label: "" };
    if (r.member === "not-matched") {
      // A session it can't take as it is: greyed, its reason and its fix.
      await chooseKind(page, "session");
      const blocked = page.getByTestId("new-pool-candidate").filter({ has: page.locator("button[role=checkbox][disabled]") });
      await expect(blocked.first()).toBeVisible();
      await expect(blocked.first()).toContainText(/Ended|not in this pool's projects|sealed/);
      await expect(blocked.first().getByTestId("new-pool-fix")).toBeVisible();
    } else if (r.member === "everyone-in-scope") {
      // D-126: every new session in its projects is Who's in's switch.
      const all = page.getByTestId("new-pool-everyone-switch");
      if ((await all.getAttribute("aria-checked")) !== "true") await all.click();
      await expect(all).toHaveAttribute("aria-checked", "true");
    } else if (r.member !== "session") {
      await chooseKind(page, r.member);
      member = await pick(page, r.member === "app" ? null : "new ones join");
      label = member.label;
    }
    await chooseKind(page, "session");
    const session = await pick(page, REACH[r.capability]!);
    const reachLabel = session.label;
    if (r.member === "session" || r.member === "not-matched") label = reachLabel;
    // Round 2 (#6, #7): the preview beside names who's in, in words.
    await expect(previewValue("who")).toContainText(reachLabel.split(" · ")[0]!);
    await page.getByTestId("new-pool-next").click();

    // 3. Who leads. D-92: one session leads; an App leads from its next
    // session (AJ-E3011), and an Account, Tool, Mac or everyone can't.
    // D-126: each choice a card, its value the choice's own.
    await expect(page.getByTestId("new-pool-step-lead")).toBeVisible();
    const broad = r.member !== "session" && r.member !== "not-matched";
    const nextLead = r.role === "lead" && r.member === "app";
    const leadCard = (value: string) => page.locator(`[data-testid="new-pool-lead-option"][data-value="${value}"]`);
    if (r.member === "app") {
      // The App's name, as its next session's card says it (the card's title names its radio).
      const title = await leadCard(`next|${member.ref}`).getByRole("radio").getAttribute("aria-labelledby");
      label = ((await page.locator(`[id="${title}"]`).textContent()) ?? "").replace(/'s next session$/, "");
      expect(label.length).toBeGreaterThan(0);
    }
    if (nextLead) {
      await leadCard(`next|${member.ref}`).click();
      await expect(page.getByTestId("new-pool-lead-sentence")).toContainText(`${label} takes charge as soon as one of its sessions starts`);
    } else if (!broad && r.role === "lead") {
      await leadCard(`session|${session.ref}`).click();
      await expect(page.getByTestId("new-pool-lead-sentence")).toContainText(`${reachLabel} leads`);
    } else if (!broad) {
      // A template may have made it the lead; this row's role is another.
      const its = leadCard(`session|${session.ref}`);
      if ((await its.count()) > 0 && (await its.getByRole("radio").isChecked())) await leadCard("none").click();
    } else if (r.role === "lead" && member.ref) {
      // A wider group can't lead: no card offers it.
      await expect(page.locator(`[data-testid="new-pool-lead-option"][data-value$="|${member.ref}"]`)).toHaveCount(0);
    }
    await page.getByTestId("new-pool-next").click();

    // 4. Who builds, who checks: the member's role and its own choice.
    // D-126: the role is a segmented control (Member, Reviewer, Observer);
    // who leads is Who leads' own.
    const everyone = r.member === "everyone-in-scope";
    const roles = everyone ? page.locator('[data-testid="new-pool-member-row"][data-everyone]') : page.getByTestId("new-pool-member-row").filter({ hasText: label }).first();
    const role = broad && r.role === "lead" ? "member" : r.role;
    if (role === "lead") {
      // The session chosen in Who leads: said, no control here.
      await expect(roles.getByTestId("new-pool-member-line")).toContainText("Leads the team");
      await expect(roles.getByTestId("new-pool-member-role")).toHaveCount(0);
    } else {
      // Neither everyone nor a wider group is offered Lead.
      await expect(roles.getByTestId("new-pool-member-role-lead")).toHaveCount(0);
      // Round 3 (#1): an App's Lead is its next session's; its control sets its other sessions'.
      if (nextLead) await expect(roles).toHaveAttribute("data-leads-next", "true");
      await roles.getByTestId(`new-pool-member-role-${role}`).click();
      await expect(roles.getByTestId(`new-pool-member-role-${role}`)).toHaveAttribute("aria-checked", "true");
    }
    // Everyone in scope follows the pool's own choice (no own picker for it).
    if (r.own_level === "member" && !everyone) {
      const own = r.level === "full" ? "Look only" : "Work together and wake each other";
      const fold = page.getByTestId("new-pool-own-fold-toggle");
      if ((await fold.getAttribute("aria-expanded")) !== "true") await fold.click();
      const level = page.getByTestId("new-pool-own-fold").locator("li").filter({ hasText: label }).first().getByTestId("new-pool-member-level");
      await level.click();
      await page.getByRole("menuitemradio", { name: new RegExp(`^${own}`) }).click();
      await expect(level).toContainText(own);
    }
    await page.getByTestId("new-pool-next").click();

    // 5. Who may message whom.
    let changed: string | null = null;
    if (r.pair_override === "one-pair") {
      const cell = page.locator('[data-testid=new-pool-talk-cell][data-pair="member>member"]');
      const before = (await cell.getAttribute("data-rule")) ?? "";
      await cell.click();
      changed = (await cell.getAttribute("data-rule")) ?? "";
      expect(changed).not.toBe(before);
      changed = changed === "ask" ? "Ask first" : changed === "blocked" ? "Blocked" : "Allowed";
    }
    await page.getByTestId("new-pool-next").click();

    // 6. What they may do on their own (D-126: a card each), then the
    // seal (a switch under Optional).
    await page.getByTestId(`new-pool-level-choice-${LEVEL_ID[r.level]!}`).click();
    await expect(page.getByTestId("new-pool-level").getByRole("radio", { name: LEVEL[r.level]!, exact: true })).toBeChecked();
    if (r.sealed === "on") {
      const seal = page.getByTestId("new-pool-seal-switch");
      if ((await seal.getAttribute("aria-checked")) !== "true") await seal.click();
      await expect(seal).toHaveAttribute("aria-checked", "true");
    }
    await page.getByTestId("new-pool-next").click();
    // D-30's question, its own step while unanswered (round 4): left for later.
    if (await page.getByTestId("new-pool-step-tools").isVisible()) await page.getByTestId("new-pool-next").click();

    // 4. The review says it; Make makes it with the user's own calls.
    const review = page.getByTestId("new-pool-review");
    await expect(review).toBeVisible();
    // The changed cell, in the review's words: named as an exception, or
    // summed up when it makes every pair the same ("Everyone may message
    // everyone").
    if (changed) await expect(review).toContainText(new RegExp(`Member to Member: ${changed.toLowerCase()}|Everyone may message everyone|No one may message|asks you first`, "i"));
    // The name: typed, or the one Review says it gets.
    const name = ((await review.textContent()) ?? "").match(/«([^»]+)»/)?.[1] ?? "";
    expect(name.trim().length).toBeGreaterThan(0);
    await page.getByTestId("new-pool-next").click();
    await expect(wizard).toHaveCount(0);
    await expect(page.getByTestId("pool-page")).toBeVisible();
    const made = await calls(page);
    expect(made).toContain(`create:${name.trim()}`);
    const id = made.find((call) => call.startsWith("scope:"))?.split(":")[1] ?? "";
    expect(made.some((call) => call.startsWith(`scope:${id}:`) && (r.scope === "every-project" ? call.endsWith(":every") : !call.endsWith(":every")))).toBe(true);
    expect(made.filter((call) => call.startsWith(`join:${id}:`)).length).toBeGreaterThanOrEqual(r.member === "session" || r.member === "not-matched" ? 1 : 2);
    expect(made.some((call) => call.startsWith(`join:${id}:`) && call.includes(`:${role}:`))).toBe(true);
    if (nextLead) expect(made.some((call) => call.startsWith(`lead_next:${id}:`) && !call.endsWith(":clear"))).toBe(true);
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
