/**
 * 0.3.3 (D-63) gallery: the apps and profiles AI Juggler remembers. Home
 * with a kept-alive profile that's down and paused, and "kept alive" on a
 * running one; Settings › Desktop apps' Your apps and profiles (Start,
 * Open, Forget, Keep alive, a paused one); an ended session's Open in
 * Claude; the rail's place menu with Keep alive ticked. Both themes, axe
 * 0 critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-keep-alive");
const T0 = new Date(2026, 9, 31, 15, 0, 0).getTime();

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

/** The places desktop's apps, with the Lab profile paused after 3 stops. */
function places(paused: boolean) {
  const known = knownPlacesPreview(T0);
  if (paused) {
    const lab = known.places.find((place) => place.instance === "Lab")!;
    lab.paused = "it stopped 3 times in 10 minutes";
  }
  return known;
}

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", paused = false) {
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
    { fixture: { accounts: ACCOUNTS, projects: PROJECTS, sessions: [], conversations: placesConversations(T0) }, known: places(paused), theme, t0: T0 },
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

for (const theme of ["dark", "light"] as const) {
  test(`Home: a kept-alive profile that's down, and one kept alive running — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await expect(page.getByTestId("home-app-closed").first()).toContainText("Not running · keep alive starts it");
    await expect(page.getByTestId("home-app-kept-mark")).toHaveText("kept alive");
    await shoot(page, "home-kept", theme);
  });

  test(`Home: keep alive paused after 3 stops — ${theme}`, async ({ page }) => {
    await seed(page, theme, true);
    await expect(page.getByTestId("home-app-closed").first()).toContainText("Keep alive paused: it stopped 3 times in 10 minutes");
    await shoot(page, "home-paused", theme);
  });

  test(`Settings › Desktop apps: your apps and profiles — ${theme}`, async ({ page }) => {
    await seed(page, theme, true);
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "keep-alive" } })));
    const lab = page.getByTestId("desktop-place-claude-app-Lab");
    await expect(lab).toContainText("Open it to start it and keep it alive again.");
    await expect(page.getByTestId("desktop-place-claude-app-Old").getByTestId("desktop-place-forget")).toBeVisible();
    await lab.scrollIntoViewIfNeeded();
    await shoot(page, "settings-your-apps", theme);
  });

  test(`An ended session: Open in Claude — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await page.getByRole("searchbox").fill("is:ended pagination");
    const row = page.locator("[data-rail-row]", { has: page.locator('[data-conversation-id="outside-2"]') }).first();
    await row.locator("[data-rail-item]").click({ position: { x: 60, y: 10 } });
    const open = page.getByTestId("conversation-open-in");
    await expect(open).toHaveText("Open in Claude");
    await expect(open).toHaveAttribute("aria-label", "Open in Claude · Personal");
    await shoot(page, "session-open-in", theme);
  });

  test(`The rail's place menu: Keep alive ticked — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const header = page.getByTestId("place-profile").filter({ hasText: "Work" }).first();
    await header.click({ button: "right" });
    await expect(page.getByTestId("place-header-menu-keep")).toHaveAttribute("aria-checked", "true");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `rail-menu-keep-${theme}.png`) });
    const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
    expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), "axe on rail-menu-keep").toEqual([]);
  });
}
