/**
 * 0.1.5 — the visual usage gallery: Usage, a project's Overview and
 * Usage, a profile's page and the Accounts list, on the usage world
 * (tests/visual/fixtures/usage-world.ts: seven profiles on four accounts,
 * 120 days of usage with a weekly rhythm, every runway state). At 1280,
 * 1680 and 2000 px, dark and light, each as the window shows it and in
 * full; and at 200 % zoom (a 640 px viewport at scale 2) with no sideways
 * scrolling. axe 0 critical/serious on each; the first paint draws the
 * frame and the time to the first figures is measured.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";
import { PROJECT_GIT, QUOTA_DISPLAY, T0, USAGE_WORLD } from "./fixtures/usage-world";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/94-usage");
const VIEWPORTS = [
  { width: 1280, height: 800 },
  { width: 1680, height: 1050 },
  { width: 2000, height: 1100 },
] as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", viewport: { width: number; height: number }, project = false) {
  await page.addInitScript(
    ({ world, quota, git, theme, t0, project }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = world;
      w.__AIJ_QUOTA_DISPLAY__ = quota;
      w.__AIJ_PROJECT_GIT__ = git;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      if (project) localStorage.setItem("aij.rail.view.v1", JSON.stringify({ group: "project" }));
      Date.now = () => t0;
    },
    { world: USAGE_WORLD, quota: QUOTA_DISPLAY, git: PROJECT_GIT, theme, t0: T0, project },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(viewport);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** The figures are in (not the "—" of a frame still reading). */
async function settled(page: Page, root: string) {
  await expect(page.getByTestId(root).getByTestId("stat-value").first()).not.toHaveText("—");
  await expect(page.getByTestId(root).getByTestId("chart-empty")).toHaveCount(0);
}

/** The window as shown, then the whole page (the window made as tall as
 * its content), then axe. */
async function shoot(page: Page, name: string, theme: string, viewport: { width: number; height: number }) {
  await page.mouse.move(0, 0);
  const base = `${name}-${viewport.width}x${viewport.height}-${theme}`;
  await page.screenshot({ path: resolve(OUT, `${base}.png`) });
  const tall = await page.evaluate(() => {
    const scrollers = [...document.querySelectorAll<HTMLElement>("main *")].filter((node) => node.scrollHeight > node.clientHeight + 4 && getComputedStyle(node).overflowY !== "visible");
    return Math.max(window.innerHeight, ...scrollers.map((node) => node.scrollHeight + node.getBoundingClientRect().top + 40));
  });
  await page.setViewportSize({ width: viewport.width, height: Math.min(6000, Math.ceil(tall)) });
  await page.waitForTimeout(150);
  await page.screenshot({ path: resolve(OUT, `${base}-full.png`) });
  await page.setViewportSize(viewport);
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

async function openProject(page: Page, tab: "Overview" | "Usage") {
  await page.getByTestId("rail").getByTestId("rail-project").filter({ hasText: "web-app" }).first().click();
  await expect(page.getByTestId("page-type")).toHaveText("Project");
  await page.getByRole("tab", { name: tab }).click();
}

async function openProfile(page: Page) {
  await goTo(page, "Accounts");
  await page.getByTestId("account-label").filter({ hasText: /^Work$/ }).first().click();
  await expect(page.getByTestId("page-type")).toHaveText("Account");
}

for (const viewport of VIEWPORTS) {
  for (const theme of ["dark", "light"] as const) {
    const at = `${viewport.width} — ${theme}`;

    test(`Usage — ${at}`, viewport.width === 1280 && theme === "dark" ? gate("94-usage") : {}, async ({ page }) => {
      await seed(page, theme, viewport);
      await goTo(page, "Usage");
      await settled(page, "usage-dashboard");
      await expect(page.getByTestId("usage-account")).toHaveCount(4);
      await shoot(page, "usage", theme, viewport);
    });

    test(`a project's Overview — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport, true);
      await openProject(page, "Overview");
      await settled(page, "project-visual-overview");
      await expect(page.getByTestId("project-uncommitted-callout")).toContainText("181 files not committed");
      await shoot(page, "project-overview", theme, viewport);
    });

    test(`a project's Usage — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport, true);
      await openProject(page, "Usage");
      await settled(page, "project-usage-dashboard");
      await shoot(page, "project-usage", theme, viewport);
    });

    test(`a profile's page — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await openProfile(page);
      await settled(page, "account-usage-dashboard");
      await expect(page.getByTestId("account-limits").getByTestId("window-gauge")).toHaveCount(2);
      await shoot(page, "profile", theme, viewport);
    });

    test(`the Accounts list — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await goTo(page, "Accounts");
      await expect(page.getByTestId("account-card")).toHaveCount(4);
      await expect(page.getByTestId("account-runway").first()).toContainText("5-hour window");
      await shoot(page, "accounts", theme, viewport);
    });
  }
}

test.describe("200 % zoom", () => {
  test.use({ viewport: { width: 640, height: 400 }, deviceScaleFactor: 2 });
  for (const theme of ["dark", "light"] as const) {
    test(`Usage and Accounts reflow with no sideways scrolling — ${theme}`, async ({ page }) => {
      await seed(page, theme, { width: 640, height: 400 });
      await goTo(page, "Usage");
      await settled(page, "usage-dashboard");
      // WCAG 1.4.10: no page-level sideways scrolling (a wide table may scroll in its own box).
      const wide = await page.evaluate(() => {
        const boxed = (node: Element | null): boolean => !!node && (getComputedStyle(node).overflowX !== "visible" || boxed(node.parentElement));
        return [...document.querySelectorAll<HTMLElement>("[data-testid='usage-screen'] *")].filter((node) => node.getBoundingClientRect().right > window.innerWidth + 1 && !boxed(node.parentElement)).length;
      });
      expect(wide, "nothing reaches past the window").toBe(0);
      await page.screenshot({ path: resolve(OUT, `usage-zoom200-${theme}.png`) });
      // The rail folds to its icons at this width: Accounts from the header.
      await goTo(page, "Accounts");
      await expect(page.getByTestId("account-card")).toHaveCount(4);
      await page.screenshot({ path: resolve(OUT, `accounts-zoom200-${theme}.png`) });
    });
  }
});

/** Clicks Usage in the More menu from inside the page and times, from
 * that click, the frame (the figures' tiles) and the first figures. */
async function timeOpen(page: Page): Promise<{ frame: number; content: number }> {
  const main = page.getByRole("navigation", { name: "Main" });
  await main.getByRole("button", { name: "More", exact: true }).click();
  const button = page.locator("#nav-more").getByRole("button", { name: /^Usage\b/ });
  await button.waitFor();
  await button.evaluate((node) => {
    const w = window as unknown as { __open?: { at: number; frame?: number; content?: number } };
    const open = { at: performance.now() } as { at: number; frame?: number; content?: number };
    w.__open = open;
    const watch = () => {
      const tile = document.querySelector("[data-testid='tile-tokens'] [data-testid='stat-value']");
      if (tile && open.frame === undefined) open.frame = performance.now() - open.at;
      if (tile && tile.textContent !== "—" && open.content === undefined) open.content = performance.now() - open.at;
      if (open.content === undefined) requestAnimationFrame(watch);
    };
    (node as HTMLElement).click();
    requestAnimationFrame(watch);
  });
  await page.waitForFunction(() => (window as unknown as { __open?: { content?: number } }).__open?.content !== undefined);
  const open = await page.evaluate(() => (window as unknown as { __open: { frame: number; content: number } }).__open);
  return { frame: Math.round(open.frame), content: Math.round(open.content) };
}

test("the first paint draws the frame; the figures follow fast", async ({ page }) => {
  await seed(page, "dark", { width: 1680, height: 1050 });
  const first = await timeOpen(page);
  await goTo(page, "Home");
  const again = await timeOpen(page);
  const timings = {
    note: "From the click on Usage, in the preview (synthetic data in the page): the frame's first frame, and the first figures. A second open shows them from memory.",
    first_open_frame_ms: first.frame,
    first_open_content_ms: first.content,
    reopen_frame_ms: again.frame,
    reopen_content_ms: again.content,
  };
  writeFileSync(resolve(OUT, "time-to-first-content.json"), JSON.stringify(timings, null, 2) + "\n");
  expect(first.frame).toBeLessThan(500);
  expect(again.content).toBeLessThan(200);
});
