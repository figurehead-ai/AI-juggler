/**
 * 0.4 (D-62, AJ-E3203) gallery: Claude app profiles found when they
 * aren't running. The ask on a running profile's App page ("Find this
 * app's other profiles", naming the folder it reads), what a yes found,
 * the rail's Inactive apps and Home's tiles with the found profiles, a
 * found profile's own page (Start), and Settings › Desktop apps' switch.
 * Both themes, axe 0 critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { goTo } from "./fixtures";
import { foundProfilesPreview, knownPlacesPreview, placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-app-profiles");
const T0 = new Date(2026, 10, 2, 15, 0, 0).getTime();

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", width = 1280) {
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
    {
      fixture: { accounts: ACCOUNTS, projects: PROJECTS, sessions: [], conversations: placesConversations(T0) },
      known: { ...knownPlacesPreview(T0), profiles: foundProfilesPreview(T0) },
      theme,
      t0: T0,
    },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

/** Opens a running profile's page from the rail's App header. */
async function openWork(page: Page) {
  await page.locator('[data-testid="place-page"][title="Open Claude app · Work Max\'s page"]').click();
  await expect(page.getByTestId("page-name")).toHaveText("Claude app · Work Max");
}

/** Says yes on the App page, and waits for what it found. */
async function findThem(page: Page) {
  await page.getByTestId("app-find-profiles-yes").click();
  // D-84 (the A6 walk): it lists what it offers, each with Add and Not now.
  const done = page.getByTestId("app-find-profiles-done");
  await expect(done).toContainText("Found 2 more profiles");
  await expect(done).toContainText("Claude app · Design");
  await expect(done).toContainText("Claude app · Research");
  // Added on the user's yes, each (nothing is added by itself).
  for (const name of ["Design", "Research"]) {
    const add = done.getByRole("button", { name: new RegExp(`^Add( Claude app · ${name})?$`) }).first();
    await add.click();
    await expect(done.getByText(`Claude app · ${name}`)).toHaveCount(0);
  }
}

/** The rail scrolled to its Inactive apps. */
async function toInactive(page: Page) {
  // Inactive apps starts folded; found ones come last, after the six shown.
  const fold = page.getByRole("button", { name: /^Inactive apps/ });
  if ((await fold.getAttribute("aria-expanded")) !== "true") await fold.click();
  const more = page.getByTestId("rail-idle-more");
  if ((await more.count()) > 0) await more.click();
  const head = page.getByTestId("rail-inactive-apps");
  await head.evaluate((element) => {
    let box = element.parentElement;
    while (box && !(box.scrollHeight > box.clientHeight && /auto|scroll/.test(getComputedStyle(box).overflowY))) box = box.parentElement;
    if (box) box.scrollTop += element.getBoundingClientRect().top - box.getBoundingClientRect().top - 8;
  });
}

for (const theme of ["dark", "light"] as const) {
  test(`the ask on a running profile's page — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWork(page);
    const ask = page.getByTestId("app-find-profiles");
    await expect(ask).toContainText("Find this app's other profiles");
    await expect(ask).toContainText("~/Profiles");
    await shoot(page, "ask", theme);
    await page.setViewportSize({ width: 1024, height: 700 });
    await shoot(page, "ask-1024", theme);
  });

  test(`a yes: what it found, and the found profiles in the rail — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWork(page);
    await findThem(page);
    await toInactive(page);
    const idle = page.getByTestId("rail-idle-app");
    await expect(idle.filter({ hasText: "Design" }).getByTestId("rail-idle-action")).toHaveAttribute("data-action", "Start");
    await expect(idle.filter({ hasText: "Research" })).toBeVisible();
    await shoot(page, "found", theme);
  });

  test(`a found profile's own page: Start — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWork(page);
    await findThem(page);
    await toInactive(page);
    await page.locator('[title="Open Claude app · Design\'s page"]').first().click();
    await expect(page.getByTestId("page-name")).toHaveText("Claude app · Design");
    await expect(page.getByTestId("app-go")).toHaveText("Start");
    await expect(page.getByTestId("page-facts")).toContainText("Found with your other profiles");
    await shoot(page, "found-app", theme);
  });

  test(`Home: the found profiles' tiles — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWork(page);
    await findThem(page);
    await goTo(page, "Home");
    // Found, never used: after the apps you've used, behind "Show 2 more".
    await page.getByTestId("home-apps-more").click();
    const closed = page.locator('[data-testid="home-app-closed"]', { hasText: "Design" });
    // The 0.4 design review (62): no "Not running" on a tile that offers Start.
    await expect(closed).toContainText("Found with your other profiles");
    await closed.scrollIntoViewIfNeeded();
    await shoot(page, "home", theme);
  });

  test(`Settings › Desktop apps: the switch — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWork(page);
    await findThem(page);
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "keep-alive" } })));
    const row = page.getByTestId("desktop-find-profiles");
    await expect(row).toContainText("~/Profiles");
    await expect(row.getByRole("switch")).toHaveAttribute("aria-checked", "true");
    await row.scrollIntoViewIfNeeded();
    await shoot(page, "settings", theme);
  });
}
