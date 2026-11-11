/**
 * L3 (the 0.4 validation plan, §4, "Links"): the item pages, one journey
 * per row of the all-pairs array `docs/qa/pairwise/pages.csv`, so every
 * pair of page, tab, size, filter, search, arriving rows, the order's
 * first state, theme and width is walked at least once.
 *
 * Each row: a page whose sessions number `size`, the first of them in
 * `state_mix`; the page opened with `tab` remembered for its kind. A tab
 * the page has opens; one it lacks opens its Overview (the memory is per
 * kind, AJ-E1534). On that tab, its own promise (counts agree with what
 * it lists). Then its Sessions list: ordered by state, headed by the
 * first state's section; past 8 rows, searchable and filterable (`filter`
 * one: State › Ended; `search` typed: "checkout"), the count line the
 * rows' own; `arriving` 3-new: three sessions start while it's read, the
 * rows hold still and "3 new" shows them at the top. Every click's
 * feedback is timed by the walk timer (≤ 100 ms) and written to
 * `docs/qa/evidence/l3/pages.json`.
 *
 * Every page kind, all 52 rows, each reached the way a user would (the
 * rail's door, the App page's Account and Tool doors, Projects, the This
 * Mac screen, Pools, an App's Sessions list). A session's page and a
 * pool's have no Sessions list of their own: their rows check the
 * remembered tab (or where it falls back) and that tab's promise; a
 * pool's members are the canvas table's.
 */
import { type Page } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "./axe-gallery";

const T0 = new Date(2026, 10, 6, 15, 0, 0).getTime();
const H = 3_600_000;
const D = 24 * H;
const WALKED = new Set(["app", "account", "project", "mac", "tool", "session", "pool"]);
const ROWS = readFileSync(resolve(process.cwd(), "../docs/qa/pairwise/pages.csv"), "utf8")
  .trim()
  .split("\n")
  .slice(1)
  .map((line) => {
    const [row, page, tab, size, filter, search, arriving, state_mix, theme, width] = line.split(",");
    return { row: Number(row), page, tab, size: size === "1000+" ? 1005 : Number(size), sizeWord: size, filter, search, arriving, state_mix, theme: theme as "dark" | "light", width: Number(width) };
  });

/** The tabs each walked page has (registry.ts), by the table's names. */
const TABS: Record<string, Record<string, string | null>> = {
  app: { overview: "overview", sessions: "sessions", projects: "projects", apps: null, pools: "pools", usage: "usage", settings: "settings" },
  account: { overview: "overview", sessions: "sessions", projects: "projects", apps: "apps", pools: "pools", usage: "usage", settings: "settings" },
  // A project's Usage shows once something is measured (`hasUsage`): "maybe".
  project: { overview: "overview", sessions: "sessions", projects: null, apps: "apps", pools: "pools", usage: "maybe", settings: null },
  mac: { overview: "overview", sessions: "sessions", projects: "projects", apps: "apps", pools: "pools", usage: null, settings: null },
  tool: { overview: "overview", sessions: "sessions", projects: "projects", apps: "apps", pools: "pools", usage: null, settings: null },
  // A session's own page: its conversation, its usage (and its processes
  // while it runs); no related lists of its own.
  session: { overview: "overview", sessions: null, projects: null, apps: null, pools: null, usage: "usage", settings: null },
};
const MEMORY: Record<string, string> = { app: "aij.page.tab.app", account: "aij.page.tab.subscription", project: "aij.page.tab.project", mac: "aij.page.tab.machine", tool: "aij.page.tab.tool", session: "aij.page.tab.conversation", pool: "aij.page.tab.pool" };
const BAND: Record<string, string> = { "needs-you": "Needs you", working: "Working", open: "Open", recent: "Last 7 days", rest: "Earlier" };

const folder = (id: string, label: string) => ({ id, provider: "claude-code", label, plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-personal", "Personal")];
const project = (id: string, name: string) => ({
  id, root: `/Users/you/Dev/${name}`, name, last_activity_ms: T0 - H, active_instance: null, accounts_used: ["Work Max"], instance_count: 3, history_sessions: 2, history_last_used_ms: null, repo: null, missing: false, pinned: false,
});
const PROJECTS = [project("proj-web", "web-shop"), project("proj-docs", "docs-site")];

type Talk = Record<string, unknown>;
function talk(id: string, title: string, over: Partial<Talk>): Talk {
  return {
    id, tool: "claude-code", account_id: "claude-work", account_label: "Work Max", project_id: "proj-web", cwd: "/Users/you/Dev/web-shop", title, git_branch: "main",
    started_ms: T0 - 3 * H, last_activity_ms: T0 - 40 * D, tokens: null, live: false, session_id: null, places: [], pid: null, state: null,
    probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}
/** The first in `mix`; the rest ended over a week ago, every fourth on
 * the checkout work, every third in docs-site. */
function talks(size: number, mix: string, oneProject = false): Talk[] {
  const first: Partial<Talk> =
    mix === "needs-you" ? { live: true, state: "asks_you", pid: 10, last_activity_ms: T0 - 60_000 }
    : mix === "working" ? { live: true, state: "working", pid: 10, last_activity_ms: T0 - 60_000 }
    : mix === "open" ? { live: true, state: "idle", pid: 10, last_activity_ms: T0 - 30 * 60_000 }
    : mix === "recent" ? { last_activity_ms: T0 - 2 * D }
    : { last_activity_ms: T0 - 9 * D };
  return Array.from({ length: size }, (_, i) =>
    talk(`t${i}`, i % 4 === 1 ? `Fix the checkout total ${i}` : `Tidy the login flow ${i}`, {
      ...(i === 0 ? first : { last_activity_ms: T0 - (10 + i) * D }),
      ...(i % 3 === 2 && !oneProject ? { project_id: "proj-docs", cwd: "/Users/you/Dev/docs-site" } : {}),
    }),
  );
}

async function walkSteps(page: Page): Promise<number[]> {
  const all = await page.evaluate(() => [...((window as unknown as { __AIJ_WALK__?: { feedbackMs: number | null }[] }).__AIJ_WALK__ ?? [])]);
  return all.map((step) => step.feedbackMs).filter((ms): ms is number => ms !== null);
}

const times: Record<number, number[]> = {};
test.afterAll(() => {
  const walked = ROWS.filter((r) => WALKED.has(r.page));
  if (Object.keys(times).length !== walked.length) return;
  const out = resolve(process.cwd(), "../docs/qa/evidence/l3");
  mkdirSync(out, { recursive: true });
  const all = Object.values(times).flat().sort((a, b) => a - b);
  const p95 = all.length ? all[Math.ceil(all.length * 0.95) - 1]! : null;
  writeFileSync(
    resolve(out, "pages.json"),
    JSON.stringify({ table: "docs/qa/pairwise/pages.csv", rows: ROWS.length, walked: walked.map((r) => r.row), pages: [...WALKED], clicks: all.length, feedback_p95_ms: p95, feedback_max_ms: all.at(-1) ?? null, budget_ms: 100, per_row_ms: times }, null, 2) + "\n",
  );
});

for (const r of ROWS) {
  const name = `L3 pages row ${r.row}: ${r.page} › ${r.tab}, ${r.sizeWord} sessions, filter ${r.filter}, search ${r.search}, ${r.arriving} arriving, first ${r.state_mix}, ${r.theme} ${r.width}`;
  if (!WALKED.has(r.page)) {
    test.fixme(name, () => {});
    continue;
  }
  test(name, async ({ page }) => {
    test.setTimeout(r.size > 100 ? 120_000 : 60_000);
    const fixture = { accounts: ACCOUNTS, projects: PROJECTS, sessions: [], conversations: talks(r.size, r.state_mix, r.page === "project") };
    await page.addInitScript(
      ({ fixture, theme, t0, memory, tab }) => {
        const w = window as unknown as Record<string, unknown>;
        // A pool of yours on web-shop (a project's own pool shows once a
        // session of AI Juggler's runs in it).
        Object.assign(w, { __AIJ_FIXTURE__: fixture, __AIJ_RANKING__: [], __AIJ_ADVICE__: [], __AIJ_POOLS__: { pools: [{ id: "pool-l3", name: "Checkout crew", project_id: "proj-web" }] } });
        localStorage.setItem("aij.theme", theme);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        localStorage.setItem("aij.walk-timing", "1");
        localStorage.setItem(memory, tab);
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture, theme: r.theme, t0: T0, memory: MEMORY[r.page]!, tab: r.tab },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: r.theme });
    await page.setViewportSize({ width: r.width, height: r.width === 1280 ? 800 : 700 });
    await page.goto("/");
    await expect(page.getByTestId("rail")).toBeVisible();

    // The page, by a user's way to it: the App's from the window's own
    // event (the rail's door); the Account's and the Tool's from the App
    // page's doors; the project's from Projects; This Mac from its screen.
    if (r.page === "pool") {
      // A pool's page, from its card on Pools ("Checkout crew", seeded):
      // its canvas, whatever tab
      // another page left remembered (a pool has Members, Tasks, Messages
      // and Agent access, none of the table's). Its members, sizes and
      // edges are the canvas table's (docs/qa/pairwise/canvas.csv).
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "pools" } })));
      await page.getByTestId("pool-card-name").filter({ hasText: "Checkout crew" }).first().click();
      await expect(page.getByTestId("pool-page")).toBeVisible();
      await expect(page.getByTestId("pool-page")).not.toBeEmpty();
      const steps = await walkSteps(page);
      times[r.row] = steps.map((ms) => Math.round(ms));
      expect(Math.max(0, ...steps), `row ${r.row}: every click's feedback ≤ 100 ms (${steps.join(", ")})`).toBeLessThanOrEqual(100);
      return;
    }
    if (r.page === "project") {
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "projects" } })));
      await page.getByTestId("projects-list").getByText("web-shop", { exact: true }).first().click();
      await expect(page.getByTestId("project-root")).toBeVisible();
    } else if (r.page === "mac") {
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "thismac" } })));
      await expect(page.getByTestId("this-mac-page")).toBeVisible();
    } else {
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-app", { detail: { id: "cli:claude-work" } })));
      await expect(page.getByTestId("app-page")).toBeVisible();
      if (r.page === "account") {
        await page.getByTestId("door-account").first().click();
        await expect(page.getByTestId("account-page")).toBeVisible();
      } else if (r.page === "tool") {
        await page.getByTestId("context-tool").first().click();
        await expect(page.getByTestId("app-page")).toHaveCount(0);
      } else if (r.page === "session") {
        // A session's page, from its App's Sessions list: the first row.
        await page.getByTestId("page-tab-sessions").click();
        const rows = page.locator("#page-panel-sessions [data-testid=page-session-item]");
        if (r.size === 0) {
          // No session to open: the list teaches instead.
          await expect(page.locator("#page-panel-sessions").getByTestId("related-list")).toBeVisible();
          await expect(rows).toHaveCount(0);
          times[r.row] = (await walkSteps(page)).map((ms) => Math.round(ms));
          return;
        }
        await rows.first().getByRole("button").first().click();
        await expect(page.getByTestId("conversation-page")).toBeVisible();
      }
    }
    const tabs = page.getByRole("tablist").first();

    // The remembered tab, or Overview where the page has none such.
    const listed = TABS[r.page]![r.tab];
    const want = listed === "maybe" ? ((await page.getByTestId(`page-tab-${r.tab}`).count()) > 0 ? r.tab : "overview") : (listed ?? "overview");
    await expect(page.getByTestId(`page-tab-${want}`)).toHaveAttribute("aria-selected", "true");
    if (TABS[r.page]![r.tab] === null) await expect(page.getByTestId(`page-tab-${r.tab}`)).toHaveCount(0);

    if (r.page === "session") {
      // Its tab's promise; a session's page lists nothing to search,
      // filter or hold (its conversation is the page's own feed).
      const panel = page.locator(`#page-panel-${want}`);
      await expect(panel).toBeVisible();
      await expect(panel).not.toBeEmpty();
      const steps = await walkSteps(page);
      times[r.row] = steps.map((ms) => Math.round(ms));
      expect(Math.max(0, ...steps), `row ${r.row}: every click's feedback ≤ 100 ms (${steps.join(", ")})`).toBeLessThanOrEqual(100);
      return;
    }

    // The Sessions tab's count is the page's sessions (the 1 005 too).
    const sessionsTab = page.getByTestId("page-tab-sessions");
    await expect(sessionsTab).toHaveText(new RegExp(`^Sessions${r.size > 0 ? ` ?${r.size}` : ""}$`));

    // That tab's own promise.
    const panel = page.locator(`#page-panel-${want}`);
    await expect(panel).toBeVisible();
    switch (want) {
      case "overview":
        if (r.page === "app" || r.page === "account") await expect(panel.getByTestId(r.page === "app" ? "app-overview" : "account-overview")).toBeVisible();
        else await expect(panel).not.toBeEmpty();
        break;
      case "projects": {
        // The projects its sessions were in, counted as listed.
        const worked = r.size === 0 ? 0 : r.size > 2 ? 2 : 1;
        if (worked > 0 && (r.page === "app" || r.page === "account")) await expect(page.getByTestId("page-tab-projects")).toHaveText(`Projects ${worked}`);
        else await expect(panel).not.toBeEmpty();
        break;
      }
      case "apps":
        // The Account's Apps: its terminal folder (one App here).
        if (r.page === "account") await expect(panel.getByTestId("account-apps-tab")).toContainText("Work Max");
        else await expect(panel).not.toBeEmpty();
        break;
      case "usage":
      case "pools":
      case "settings":
        await expect(panel).not.toBeEmpty();
        break;
      default:
        break;
    }

    // Its Sessions list.
    if (want !== "sessions") await sessionsTab.click();
    const list = page.locator("#page-panel-sessions").getByTestId("related-list");
    await expect(list).toBeVisible();
    const rows = list.getByTestId("page-session-item");
    if (r.size === 0) {
      await expect(rows).toHaveCount(0);
    } else {
      await expect(rows.first()).toBeVisible();
      // Ordered by state: the first state's section leads (headed once there are two).
      const heads = list.getByTestId("page-sessions-group");
      if (r.state_mix === "rest" || r.size === 1) await expect(heads).toHaveCount(0);
      else await expect(heads.first()).toHaveText(new RegExp(`^${BAND[r.state_mix]}`));
      await expect(rows.first()).toHaveAttribute("data-key", /\/t0$/);
    }

    // Past 8 rows it can be searched and filtered; short, it needn't be.
    const findable = r.size > 8;
    await expect(list.getByTestId("related-search")).toHaveCount(findable ? 1 : 0);
    if (findable && r.search === "typed") {
      await list.getByTestId("related-search").fill("checkout");
      const expected = Math.floor((r.size - 2) / 4) + 1; // every fourth, from the second
      await expect(list.getByTestId("page-sessions-count")).toHaveText(new RegExp(`^${expected} sessions?`));
      await expect(rows.first()).toContainText("checkout");
    }
    if (findable && r.filter === "one") {
      await list.getByTestId("related-filter-state").click();
      await page.getByRole("menuitemradio", { name: /Ended/ }).click();
      await expect(list.getByTestId("related-filter-clear")).toBeVisible();
      const live = ["needs-you", "working", "open"].includes(r.state_mix) ? 1 : 0;
      const typed = r.search === "typed";
      // Ended ones only (and the search's, when typed).
      const expected = typed ? Math.floor((r.size - 2) / 4) + 1 : r.size - live;
      await expect(list.getByTestId("page-sessions-count")).toHaveText(new RegExp(`^${expected} sessions?`));
      await list.getByTestId("related-filter-clear").click();
      await expect(list.getByTestId("related-filter-clear")).toHaveCount(0);
    } else if (findable && r.search === "typed") {
      await list.getByTestId("related-search").fill("");
    }

    // Three start while it's read: the rows hold still, "3 new" brings them.
    if (r.arriving === "3-new") {
      // Read while it's open: after its first fresh read is shown.
      await expect(list.getByTestId("page-sessions")).toHaveAttribute("data-read", "read");
      const firstKey = r.size > 0 ? await rows.first().getAttribute("data-key") : null;
      const fresh = [1, 2, 3].map((n) => talk(`new${n}`, `Just started ${n}`, { live: true, state: "working", pid: 20 + n, last_activity_ms: T0 - n * 1000, started_ms: T0 - n * 1000 }));
      await page.evaluate((fresh) => {
        const w = window as unknown as { __AIJ_FIXTURE__: { conversations: unknown[] } };
        w.__AIJ_FIXTURE__.conversations = [...fresh, ...w.__AIJ_FIXTURE__.conversations];
        window.dispatchEvent(new Event("aij:preview-conversations-changed"));
      }, fresh);
      // Working ones come in above some row shown, so they wait behind
      // "3 new"; under a lone session that needs you there's nothing below
      // them, so they just join the end (hold.ts), the first row unmoved.
      if (r.size > 0) {
        if (!(r.state_mix === "needs-you" && r.size === 1)) {
          await expect(list.getByTestId("related-waiting")).toHaveText(/3 new/);
          await expect(rows.first()).toHaveAttribute("data-key", firstKey!);
          await list.getByTestId("related-waiting").click();
        } else {
          await expect(list.getByTestId("page-session-item").filter({ hasText: "Just started" })).toHaveCount(3);
          await expect(rows.first()).toHaveAttribute("data-key", firstKey!);
        }
        await expect(list.getByTestId("related-waiting")).toHaveCount(0);
      }
      // Shown: the new ones lead (working, after any that needs you).
      await expect(list.getByTestId("page-session-item").filter({ hasText: "Just started" })).toHaveCount(3);
      await expect(sessionsTab).toHaveText(`Sessions ${r.size + 3}`);
    }

    const steps = await walkSteps(page);
    times[r.row] = steps.map((ms) => Math.round(ms));
    // A row that only opens the page clicks nothing to time.
    expect(Math.max(0, ...steps), `row ${r.row}: every click's feedback ≤ 100 ms (${steps.join(", ")})`).toBeLessThanOrEqual(100);
  });
}
