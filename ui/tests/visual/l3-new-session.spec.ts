/**
 * L3 (the 0.4 validation plan, §4): New session, one journey per row of
 * the all-pairs array `docs/qa/pairwise/new_session.csv`, so every pair of
 * history, room, folder, tool, rare option, start key, theme and width
 * is walked at least once. Each row: the usual made and said for its
 * history; the folder and App picked; the room said before starting
 * (low, out — never switched silently, D-31); a rare option changed (and
 * not remembered next time, N3); started by the button or ⌘↩; the session
 * started with that App in that folder (the preview records the start,
 * then says no backend is connected). Every click's feedback is timed
 * by the walk timer (≤ 100 ms), and the rows' times go to
 * `docs/qa/evidence/l3/new-session.json`.
 */
import { type Page } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "./axe-gallery";

const T0 = new Date(2026, 10, 2, 15, 0, 0).getTime();
const H = 3_600_000;
const ROWS = readFileSync(resolve(process.cwd(), "../docs/qa/pairwise/new_session.csv"), "utf8")
  .trim()
  .split("\n")
  .slice(1)
  .map((line) => {
    const [row, history, room, folder, tool, rare_changed, start, theme, width] = line.split(",");
    return { row: Number(row), history, room, folder, tool, rare_changed, start, theme: theme as "dark" | "light", width: Number(width) };
  });

const TOOLS: Record<string, { account: string; label: string; name: string }> = {
  "claude-code": { account: "claude-work", label: "Work Max", name: "Claude Code" },
  codex: { account: "codex-home", label: "~/.codex", name: "Codex" },
  opencode: { account: "opencode-home", label: "OpenCode home", name: "OpenCode" },
  "grok-build": { account: "grok-home", label: "Grok home", name: "Grok Build" },
  "kimi-code": { account: "kimi-home", label: "Kimi home", name: "Kimi Code" },
};
const folderInfo = (id: string, provider: string, label: string) => ({ id, provider, label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folderInfo("claude-personal", "claude-code", "Personal"), ...Object.entries(TOOLS).map(([tool, t]) => folderInfo(t.account, tool, t.label))];
const RECIPES = ACCOUNTS.map((a, i) => ({ id: `p-${i + 1}`, name: "Main", account_id: a.id, account_label: a.label, provider: a.provider, model: null, default_cwd: null, restart_on_crash: null }));
const recipeOf = (account: string) => RECIPES.find((r) => r.account_id === account)!.id;

const project = (name: string, root: string, used: string[], at: number) => ({
  id: `proj-${name}`, root, name, last_activity_ms: used.length ? T0 - at * 60_000 : null, active_instance: null,
  accounts_used: used, instance_count: used.length ? 3 : 0, history_sessions: used.length ? 2 : 0, history_last_used_ms: null, repo: null, missing: false, pinned: false,
});
const projects = (history: string) => {
  const used = history === "none" ? [] : history === "one-usual" ? ["Work Max"] : ["Personal", "Work Max"];
  return [
    project("web-shop", "/Users/you/Dev/web-shop", used, 12),
    project("feat-checkout", "/Users/you/Dev/web-shop/.worktrees/feat-checkout", [], 0),
    project("docs-site", "/Users/you/Dev/docs-site", [], 0),
  ];
};
const room = (account: string, left: number) => ({
  account,
  five_hour: { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: T0 + 2 * H, badge: "measured", measured_at_ms: T0 - 120_000 },
  weekly: null, forecast: { state: "learning", badge: "learning" }, renewal: { state: "none", label: "", entered_by_user: false },
});
const LEFT: Record<string, number> = { fine: 58, low: 8, out: 0 };

async function walkSteps(page: Page): Promise<{ feedbackMs: number | null }[]> {
  return page.evaluate(() => [...((window as unknown as { __AIJ_WALK__?: { feedbackMs: number | null }[] }).__AIJ_WALK__ ?? [])]);
}

const times: Record<number, number[]> = {};
test.afterAll(() => {
  const out = resolve(process.cwd(), "../docs/qa/evidence/l3");
  mkdirSync(out, { recursive: true });
  const all = Object.values(times).flat().sort((a, b) => a - b);
  const p95 = all.length ? all[Math.ceil(all.length * 0.95) - 1]! : null;
  if (Object.keys(times).length === ROWS.length)
    writeFileSync(resolve(out, "new-session.json"), JSON.stringify({ table: "docs/qa/pairwise/new_session.csv", rows: ROWS.length, clicks: all.length, feedback_p95_ms: p95, feedback_max_ms: all.at(-1) ?? null, budget_ms: 100, per_row_ms: times }, null, 2) + "\n");
});

for (const r of ROWS) {
  test(`L3 New session row ${r.row}: ${r.history}, ${r.room}, ${r.folder}, ${r.tool}, ${r.rare_changed}, ${r.start}, ${r.theme} ${r.width}`, async ({ page }) => {
    const tool = TOOLS[r.tool]!;
    const quota = ACCOUNTS.map((a) => room(a.id, a.id === tool.account ? LEFT[r.room]! : 82));
    await page.addInitScript(
      ({ fixture, recipes, quota, theme, t0, picked }) => {
        const w = window as unknown as Record<string, unknown>;
        Object.assign(w, { __AIJ_FIXTURE__: fixture, __AIJ_RECIPES__: recipes, __AIJ_QUOTA_DISPLAY__: quota, __AIJ_RANKING__: [], __AIJ_ADVICE__: [], __AIJ_PICK_DIRECTORY__: picked });
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        localStorage.setItem("aij.walk-timing", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: { accounts: ACCOUNTS, projects: projects(r.history), sessions: [] }, recipes: RECIPES, quota, theme: r.theme, t0: T0, picked: "/Users/you/Downloads/notes" },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: r.theme });
    await page.setViewportSize({ width: r.width, height: r.width === 1280 ? 800 : 700 });
    await page.goto("/");
    await expect(page.getByTestId("rail")).toBeVisible();
    const openSheet = async () => {
      await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
      await expect(page.getByTestId("new-session-sheet")).toBeVisible();
    };
    await openSheet();
    const sentence = page.getByTestId("new-session-sentence");

    // The usual, made and said for this history (N1, N4).
    if (r.history === "none") await expect(sentence).not.toContainText("the last you used here");
    else await expect(sentence).toContainText(/Start in web-shop on Claude Code · (Work Max|Personal) — the last you used here/);

    // The folder.
    const folderName = r.folder === "project" ? "web-shop" : r.folder === "worktree" ? "feat-checkout" : "notes";
    const folderRoot = r.folder === "project" ? "/Users/you/Dev/web-shop" : r.folder === "worktree" ? "/Users/you/Dev/web-shop/.worktrees/feat-checkout" : "/Users/you/Downloads/notes";
    await page.getByTestId("change-folder").click();
    if (r.folder === "outside-any") {
      await page.getByRole("menu", { name: "Where to work" }).getByText("Choose another folder…").click();
    } else {
      await page.getByRole("menu", { name: "Where to work" }).getByText(folderName, { exact: true }).click();
    }
    await expect(sentence).toContainText(`Start in ${folderName}`);

    // The App.
    await page.getByTestId("change-app").click();
    // Suggested, then All: an App can be listed in both.
    await page.getByTestId(`change-app-${tool.account}`).first().click();
    await expect(sentence).toContainText(`on ${tool.name} · ${tool.label}`);

    // The room, said before starting, never switched (D-31).
    if (r.room === "out") await expect(page.getByTestId("launch-out-of-room")).toBeVisible();
    else if (r.room === "low") await expect(page.getByTestId("launch-low")).toBeVisible();
    else await expect(page.getByTestId("launch-out-of-room").or(page.getByTestId("launch-low"))).toHaveCount(0);

    // A rare option changed, for this session only (N3).
    if (r.rare_changed !== "no") {
      await page.getByTestId("new-session-more").locator("summary").click();
      await page.getByRole("button", { name: "How it runs" }).click();
      await page.getByRole("menu").getByText("Start in terminal", { exact: true }).click();
      await expect(page.getByTestId("new-session-more-values")).toContainText("runs in its terminal");
    }

    // Started by the button or ⌘↩, with that App in that folder.
    if (r.start === "button") await page.getByTestId("launch-now").click();
    else {
      await page.getByTestId("launch-now").focus();
      await page.keyboard.press("Meta+Enter");
    }
    // The preview starts nothing: it records the start and says so.
    await expect(page.getByTestId("new-session-sheet").getByRole("alert")).toHaveText("Preview mode — no backend connected.");
    const launched = await page.evaluate(() => {
      const w = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[]; __AIJ_LAUNCH_CWDS__?: (string | null)[] };
      return { recipes: w.__AIJ_LAUNCH_CALLS__ ?? [], cwds: w.__AIJ_LAUNCH_CWDS__ ?? [] };
    });
    expect(launched.recipes).toEqual([recipeOf(tool.account)]);
    expect(launched.cwds).toEqual([folderRoot]);

    // Nothing rare is remembered next time.
    if (r.rare_changed !== "no") {
      await page.getByTestId("new-session-sheet").getByRole("button", { name: "Cancel" }).click();
      await expect(page.getByTestId("new-session-sheet")).toBeHidden();
      await openSheet();
      await expect(page.getByTestId("new-session-more-values")).toContainText("runs as its launch settings say");
    }

    const steps = (await walkSteps(page)).map((s) => s.feedbackMs).filter((ms): ms is number => ms !== null);
    times[r.row] = steps.map((ms) => Math.round(ms));
    expect(steps.length).toBeGreaterThan(3);
    expect(Math.max(...steps), `row ${r.row}: every click's feedback ≤ 100 ms (${steps.join(", ")})`).toBeLessThanOrEqual(100);
  });
}
