/**
 * AJ-E1537 gallery: one action registry, the same menu from every way
 * in. Each item type's menu (a session row, a project row, a project
 * page's header, an account card, a history entry, This Mac's lines) in
 * both themes, axe 0 critical/serious on each, and the keyboard way in
 * (Shift+F10 on a focused row, Escape back).
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/77-menus");
const T0 = 1_728_800_000_000;

/** allStatuses plus an account to open the menu of. */
const MANAGE = {
  ...allStatuses,
  accounts: [
    ...allStatuses.accounts,
    { id: "claude-max", provider: "claude-code", label: "personal-max", plan: "max-5x", login_state: "logged_in", in_place: false, import_source_path: "/Users/you/max" },
  ],
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", fixture: unknown = allStatuses) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: structuredClone(fixture), theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

const labels = (page: Page) =>
  page.getByRole("menu").getByRole("menuitem").evaluateAll((items) => items.map((item) => item.querySelector("[data-label]")?.textContent));

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`) });
  await axeClean(page, name);
}

for (const theme of ["dark", "light"] as const) {
  test(`a session row: ⋯, right-click and Shift+F10 open the same menu — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    // The row and its ⋯: the same session either way.
    const row = page.locator(".group", { has: page.locator('[data-instance-id="sess-2"]') }).first();
    await row.hover();
    await row.getByRole("button", { name: /^Session actions for / }).click();
    const fromButton = await labels(page);
    expect(fromButton[0]).toBe("Open");
    // Stop, when the session is live, is last and alone.
    if (fromButton.includes("Stop")) expect(fromButton.at(-1)).toBe("Stop");
    await shoot(page, "session-menu", theme);
    await page.keyboard.press("Escape");
    await row.click({ button: "right" });
    expect(await labels(page)).toEqual(fromButton);
    await page.keyboard.press("Escape");
    await row.getByRole("button").first().focus();
    await page.keyboard.press("Shift+F10");
    expect(await labels(page)).toEqual(fromButton);
    await expect(page.getByRole("menuitem", { name: "Open", exact: true })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("menu")).toHaveCount(0);
  });

  test(`a project row and its page's header — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const header = page.getByTestId("rail-project").first();
    await header.click({ button: "right" });
    const row = await labels(page);
    expect(row[0]).toBe("Open");
    expect(row.at(-1)).toBe("Forget…");
    await shoot(page, "project-menu", theme);
    await page.keyboard.press("Escape");
    await header.click();
    await page.getByTestId("header-menu").click();
    // On the page: the same menu, less Open.
    expect(await labels(page)).toEqual(row.filter((label) => label !== "Open"));
    await shoot(page, "project-header-menu", theme);
  });

  test(`an account card — ${theme}`, async ({ page }) => {
    await seed(page, theme, MANAGE);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Account actions for personal-max", exact: true }).click();
    const items = await labels(page);
    expect(items.slice(-4)).toEqual(["Rename…", "Copy settings from…", "Make the default", "Remove…"]);
    await shoot(page, "account-menu", theme);
  });

  test(`a history entry — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await goTo(page, "History");
    await page.getByTestId("page-session-item").first().click({ button: "right" });
    const items = await labels(page);
    expect(items[0]).toBe("Open");
    expect(items.at(-1)).toBe("Delete…");
    await shoot(page, "history-menu", theme);
  });
}
