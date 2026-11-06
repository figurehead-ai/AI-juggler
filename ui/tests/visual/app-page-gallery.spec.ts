/**
 * 0.4 (D-62, AJ-E3204) gallery: an App's page. A running Claude app
 * profile (Overview at 1280 and 1024, Sessions, Settings), one that's
 * down (Start), and a CLI App (New session, its folder's limits and
 * settings). Opened from the rail's App header and Home's tiles, as a
 * user would. Both themes, axe 0 critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-app-page");
const T0 = new Date(2026, 10, 1, 15, 0, 0).getTime();
const H = 3_600_000;

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-side", provider: "claude-code", label: "Side project", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

/** The places desktop, plus a CLI app with nothing open: two sessions on
 * the Side project folder, ended yesterday and two days ago. */
function conversations() {
  const rows = placesConversations(T0);
  const ended = (id: string, title: string, hours: number) => ({
    ...rows[0]!,
    id,
    title,
    account_id: "claude-side",
    account_label: "Side project",
    live: false,
    places: [],
    pid: null,
    state: null,
    viewed_ms: null,
    last_activity_ms: T0 - hours * H,
  });
  return [...rows, ended("side-1", "Sketch the landing page", 20), ended("side-2", "Try the new chart library", 50)];
}

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, known, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PLACES__ = known;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, projects: PROJECTS, sessions: [], conversations: conversations() }, known: knownPlacesPreview(T0), theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string) {
  await expect(page.getByTestId("app-page")).toBeVisible();
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

/** Opens an App's page from the rail's header (its name's button). */
async function openFromRail(page: Page, name: string) {
  await page.locator(`[data-testid="place-page"][title="Open ${name}'s page"]`).click();
  await expect(page.getByTestId("page-name")).toHaveText(name);
}

async function tab(page: Page, label: RegExp) {
  await page.getByRole("tab", { name: label }).click();
}

for (const theme of ["dark", "light"] as const) {
  test(`a running Claude app profile: its Overview — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openFromRail(page, "Claude app · Work Max");
    await expect(page.getByTestId("app-go")).toHaveText("Go to it");
    await expect(page.getByTestId("app-open").getByTestId("page-session-item").first()).toBeVisible();
    await shoot(page, "app-overview", theme);
    await page.setViewportSize({ width: 1024, height: 700 });
    await shoot(page, "app-overview-1024", theme);
  });

  test(`its Sessions and its Settings — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openFromRail(page, "Claude app · Work Max");
    await tab(page, /^Sessions/);
    await expect(page.getByTestId("page-sessions-list")).toBeVisible();
    await shoot(page, "app-sessions", theme);
    await tab(page, /^Settings/);
    await expect(page.getByTestId("app-name-row")).toBeVisible();
    await shoot(page, "app-settings", theme);
  });

  test(`a profile that's down, from Home's tile — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await page.locator('[data-testid="home-app-page"][title="Open Claude app · Lab\'s page"]').click();
    await expect(page.getByTestId("app-go")).toHaveText("Start");
    await shoot(page, "app-closed", theme);
  });

  test(`a CLI App: New session, its folder's limits — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openFromRail(page, "Claude Code · Work Max");
    await expect(page.getByTestId("app-new-session")).toBeVisible();
    await expect(page.getByTestId("account-limits")).toBeVisible();
    await shoot(page, "app-cli", theme);
  });
}
