/**
 * D-38 gallery (0.1.5): apps' own icons where the rail, Home, History and
 * the palette name an app, with the profile's badge beside or at the
 * corner, never over the icon; and the letter fallback for an app whose
 * icon can't be read (ChatGPT here). At 1280 and 2000 px wide, both
 * themes for the rail and Home; axe 0 critical/serious on each.
 *
 * The icons are made-up test images drawn below as `data:` URIs: plain
 * shapes on colour, nobody's logo. No app's real icon is in the
 * repository (D-38).
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/95-app-icons");
const T0 = new Date(2026, 9, 25, 15, 0, 0).getTime();

/** A test icon: a macOS-shaped square in `from`→`to`, with `glyph` (SVG
 * markup on a 100-unit grid) in white. */
function testIcon(from: string, to: string, glyph: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${from}"/><stop offset="1" stop-color="${to}"/></linearGradient></defs><rect x="9" y="9" width="82" height="82" rx="19" fill="url(#g)"/><rect x="9.5" y="9.5" width="81" height="81" rx="18.5" fill="none" stroke="#000" stroke-opacity=".12"/>${glyph}</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const ICONS: Record<string, string> = {
  // A desktop app: a ring and a dot.
  "claude-app": testIcon("#7a8cff", "#4b5bd6", '<circle cx="50" cy="50" r="20" fill="none" stroke="#fff" stroke-width="8"/><circle cx="50" cy="50" r="6" fill="#fff"/>'),
  // A terminal: a prompt.
  terminal: testIcon("#3a4152", "#1b1f29", '<path d="M30 38l14 12-14 12" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/><path d="M50 64h20" stroke="#fff" stroke-width="7" stroke-linecap="round"/>'),
  // AI Juggler's own place.
  "ai-juggler": testIcon("#2fb7c9", "#167a8a", '<circle cx="34" cy="58" r="8" fill="#fff"/><circle cx="50" cy="40" r="8" fill="#fff"/><circle cx="66" cy="58" r="8" fill="#fff"/>'),
  // ChatGPT ("codex-app") is left out: its letters stand in.
};

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "codex", provider: "codex", label: "Codex", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

function own(id: string, over: Record<string, unknown> = {}) {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: "opus", launch_model: "opus",
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "feat/read-model", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: T0 - 600_000,
    last_activity_ms: T0 - 60_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

const FIXTURE = {
  accounts: ACCOUNTS,
  projects: PROJECTS,
  sessions: [own("sess-1"), own("sess-2", { provider: "codex", account_id: "codex", account_label: "Codex", git_branch: "fix/limits" })],
  conversations: placesConversations(T0),
};

const WIDTHS = [1280, 2000] as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, width: number, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, icons, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_APP_ICONS__ = icons;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, icons: ICONS, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: width >= 2000 ? 1125 : 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** Every app icon on screen has settled: its icon, or its letters. */
async function settled(page: Page) {
  await expect(page.locator('[data-testid="app-icon"][data-state="loading"]')).toHaveCount(0);
}

async function shoot(page: Page, name: string, target: "page" | "rail" = "page") {
  await settled(page);
  await page.mouse.move(0, 0);
  const path = resolve(OUT, `${name}.png`);
  if (target === "rail") await page.getByTestId("rail").screenshot({ path });
  else await page.screenshot({ path });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

// The rail at the Mac's own 2× scale: its icons are small.
test.describe("the rail at 2×", () => {
  test.use({ deviceScaleFactor: 2 });
  for (const width of WIDTHS) {
    for (const theme of ["dark", "light"] as const) {
      test(`the rail with apps' icons — ${width} — ${theme}`, async ({ page }) => {
        await seed(page, width, theme);
        await expect(page.getByTestId("place-header").first()).toBeVisible();
        // Claude, Terminal and AI Juggler draw their icons; ChatGPT its letters.
        await settled(page);
        await expect(page.locator('[data-testid="app-icon"][data-app="claude-app"][data-state="icon"]').first()).toBeVisible();
        await expect(page.locator('[data-testid="app-icon"][data-app="codex-app"][data-state="letters"]').first()).toBeVisible();
        await shoot(page, `rail-${width}-${theme}`, "rail");
      });
    }
  }
});

for (const width of WIDTHS) {
  for (const theme of ["dark", "light"] as const) {
    test(`Home: your apps and what needs you — ${width} — ${theme}`, async ({ page }) => {
      await seed(page, width, theme);
      await expect(page.getByTestId("home-app-tile").first()).toBeVisible();
      await shoot(page, `home-${width}-${theme}`);
    });
  }

  test(`History's rows with their apps — ${width}`, async ({ page }) => {
    await seed(page, width, "dark");
    await page.locator("header").getByRole("button", { name: "History", exact: true }).click();
    await expect(page.getByTestId("page-session-row").first()).toBeVisible();
    await shoot(page, `history-${width}-dark`);
  });

  test(`the palette with its sessions' apps — ${width}`, async ({ page }) => {
    await seed(page, width, "dark");
    await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
    await page.keyboard.press("Meta+k");
    await expect(page.getByTestId("goto-palette")).toBeVisible();
    await shoot(page, `palette-${width}-dark`);
  });
}
