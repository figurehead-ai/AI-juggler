/**
 * AJ-E1537 gallery: one action registry, the same menu from every way
 * in. Each item type's menu (a session row, a project row, a project
 * page's header, an account card, a history entry, This Mac's lines) in
 * both themes, axe 0 critical/serious on each, and the keyboard way in
 * (Shift+F10 on a focused row, Escape back).
 *
 * AJ-E2106: a right-click opens the in-app menu at the pointer (not the
 * system's), icons on every line, three groups with stopping last, the
 * one submenu from the keyboard, a greyed line with its reason, and ⌘K
 * listing the selected session's actions first — both themes each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, projectLens } from "./fixtures";

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
  test(`a session row: ⋯, right-click and Shift+F10 open the same menu — ${theme}`, gate("77-menus"), async ({ page }) => {
    await seed(page, theme);
    // The row and its ⋯: the same session either way.
    const row = page.locator(".group", { has: page.locator('[data-instance-id="sess-2"]') }).first();
    await row.hover();
    await row.getByRole("button", { name: /^Session actions for / }).click();
    const fromButton = await labels(page);
    // AJ-E2106: getting there first, each line with its icon.
    expect(fromButton.slice(0, 3)).toEqual(["Go to it", "Open its page", "Show its project"]);
    expect(fromButton).toContain("Continue on another account…");
    // Stop, the session being live, is last and alone, in red.
    expect(fromButton.at(-1)).toBe("Stop this session");
    await expect(page.getByRole("menu").locator("svg[data-icon]")).toHaveCount(fromButton.length);
    await shoot(page, "session-menu", theme);
    await page.keyboard.press("Escape");
    // AJ-E2106: the right-click opens the in-app menu at the pointer,
    // with its icons and red Stop.
    const box = (await row.boundingBox())!;
    await row.click({ button: "right", position: { x: 40, y: 10 } });
    expect(await labels(page)).toEqual(fromButton);
    await expect(page.getByRole("menu").locator("svg[data-icon]")).toHaveCount(fromButton.length);
    const at = (await page.getByRole("menu").boundingBox())!;
    expect(Math.abs(at.x - (box.x + 40))).toBeLessThanOrEqual(1);
    // At the pointer, or (a menu taller than the room below it) moved up
    // just enough to stay in the window.
    const viewport = page.viewportSize()!;
    if (Math.abs(at.y - (box.y + 10)) > 1) {
      expect(at.y).toBeLessThan(box.y + 10);
      expect(Math.abs(at.y + at.height - (viewport.height - 4))).toBeLessThanOrEqual(1);
    }
    await page.keyboard.press("Escape");
    await row.getByRole("button").first().focus();
    await page.keyboard.press("Shift+F10");
    expect(await labels(page)).toEqual(fromButton);
    await expect(page.getByRole("menuitem", { name: "Go to it", exact: true })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("menu")).toHaveCount(0);
  });

  // AJ-E2106: one level of submenus — → opens it, ← closes it.
  test(`a session's submenu, from the keyboard — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const row = page.locator(".group", { has: page.locator('[data-instance-id="sess-2"]') }).first();
    await row.getByRole("button").first().focus();
    await page.keyboard.press("Shift+F10");
    const trigger = page.getByRole("menuitem", { name: "Snooze or mute" });
    await trigger.focus();
    await page.keyboard.press("ArrowRight");
    const sub = page.getByRole("menu", { name: "Snooze or mute" });
    await expect(sub).toBeVisible();
    await expect(sub.getByRole("menuitem").first()).toBeFocused();
    await shoot(page, "session-submenu", theme);
    await page.keyboard.press("ArrowLeft");
    await expect(sub).toHaveCount(0);
    await expect(trigger).toBeFocused();
  });

  // Not possible right now: greyed, with the reason under it (4.5:1).
  test(`a session starting: greyed with the reason — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const row = page.locator(".group", { has: page.locator('[data-instance-id="sess-1"]') }).first();
    await row.hover();
    await row.getByRole("button", { name: /^Session actions for / }).click();
    const copy = page.getByRole("menuitem", { name: "Copy the resume command" });
    await expect(copy).toBeDisabled();
    await expect(page.getByRole("menu").getByText("It has no conversation to resume yet")).toBeVisible();
    await shoot(page, "session-menu-greyed", theme);
  });

  // ⌘K on the selected session: its actions with their keys, then the rest.
  test(`⌘K with a session selected — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await page.locator('[data-instance-id="sess-2"]').first().click();
    await page.keyboard.press("Meta+k");
    const palette = page.getByTestId("goto-palette");
    await expect(palette).toBeVisible();
    await page.getByRole("combobox").focus();
    await expect(palette.getByRole("option").first()).toContainText("Go to it");
    await expect(palette.getByTestId("goto-row-action::stop")).toContainText("⌘.");
    await expect(palette.getByTestId("goto-row-action::continue")).toContainText("⌘⇧H");
    await shoot(page, "palette-actions", theme);
  });

  test(`a project row and its page's header — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    const header = page.getByTestId("rail-project").first();
    await header.click({ button: "right" });
    const row = await labels(page);
    expect(row[0]).toBe("Open its page");
    expect(row.at(-1)).toBe("Forget…");
    await shoot(page, "project-menu", theme);
    await page.keyboard.press("Escape");
    await header.click();
    await page.getByTestId("header-menu").click();
    // On the page: the same menu, less Open its page.
    expect(await labels(page)).toEqual(row.filter((label) => label !== "Open its page"));
    await shoot(page, "project-header-menu", theme);
  });

  test(`an account card — ${theme}`, async ({ page }) => {
    await seed(page, theme, MANAGE);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Profile actions for personal-max", exact: true }).click();
    const items = await labels(page);
    expect(items.slice(-4)).toEqual(["Rename…", "Copy settings from…", "Make the default", "Remove…"]);
    await shoot(page, "account-menu", theme);
  });

  test(`a history entry — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await goTo(page, "History");
    await page.getByTestId("page-session-item").first().click({ button: "right" });
    const items = await labels(page);
    expect(items[0]).toBe("Open its page");
    expect(items.at(-1)).toBe("Delete…");
    await shoot(page, "history-menu", theme);
  });
}
