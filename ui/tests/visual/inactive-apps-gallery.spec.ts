/**
 * 0.3.4 (D-64) gallery: grouped by App, the rail is about apps (the
 * maintainer: a Claude app that quit overnight vanished from the rail;
 * "23 inactive" listed sessions from all over the place). Under each open
 * app, "Last 3 inactive"; after Open now, "Inactive apps" (desktop and
 * CLI) with Start, Bring forward or New session, each unfolding its last
 * ones; its menu; Home's tiles for every closed app. Both themes, axe 0
 * critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-inactive-apps");
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
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

/** The rail scrolled to its Inactive apps. */
async function toInactive(page: Page) {
  const head = page.getByTestId("rail-inactive-apps");
  // The rail's list only: the window itself never scrolls.
  await head.evaluate((element) => {
    let box = element.parentElement;
    while (box && !(box.scrollHeight > box.clientHeight && /auto|scroll/.test(getComputedStyle(box).overflowY))) box = box.parentElement;
    if (box) box.scrollTop += element.getBoundingClientRect().top - box.getBoundingClientRect().top - 8;
  });
  return head;
}

for (const theme of ["dark", "light"] as const) {
  test(`the rail: an open app's Last 3 inactive, then Inactive apps — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await expect(page.getByTestId("rail-recent").first()).toBeVisible();
    const head = await toInactive(page);
    await expect(head).toContainText("Inactive apps");
    const apps = page.getByTestId("rail-idle-app");
    await expect(apps.filter({ hasText: "Lab" }).getByTestId("rail-idle-action")).toHaveAttribute("data-action", "Start");
    await expect(apps.filter({ hasText: "Side project" }).getByTestId("rail-idle-action")).toHaveAttribute("data-action", "New session");
    await expect(page.getByTestId("rail-also")).not.toContainText("inactive");
    await shoot(page, "rail-inactive-apps", theme);
  });

  test(`the rail: Last 3 unfolded under an open app and an inactive one — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const recent = page.getByTestId("rail-recent").first();
    await recent.click();
    await expect(recent).toHaveAttribute("aria-expanded", "true");
    await toInactive(page);
    await page.getByTestId("rail-idle-app").filter({ hasText: "Side project" }).getByTestId("rail-fold").click();
    await expect(page.locator('[data-conversation-id="side-1"]').first()).toBeVisible();
    await shoot(page, "rail-last-3", theme);
  });

  test(`the rail: an inactive app's menu — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await toInactive(page);
    await page.getByTestId("rail-idle-app").filter({ hasText: "Lab" }).click({ button: "right" });
    await expect(page.getByTestId("rail-idle-menu-act")).toHaveText("Start");
    await expect(page.getByTestId("rail-idle-menu-keep")).toHaveAttribute("aria-checked", "true");
    await shoot(page, "rail-idle-menu", theme);
  });

  test(`Home: every closed app with Start — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const closed = page.getByTestId("home-app-closed");
    await expect(closed).toHaveCount(2);
    await expect(closed.first()).toContainText("keep alive starts it");
    await closed.last().scrollIntoViewIfNeeded();
    await shoot(page, "home-closed-apps", theme);
  });
}
