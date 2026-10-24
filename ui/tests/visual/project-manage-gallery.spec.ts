/**
 * AJ-E1502 gallery: sessions and projects named, pinned, opened and
 * tidied — the rail with a labelled, pinned session and a pinned
 * project; a session's and a project's row menus; the rename and forget
 * sheets; the project view's own actions (with an editor chosen);
 * Relocate… on a missing folder; Settings' editor choice. Both themes,
 * axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, projectLens } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/76-project-manage");
const T0 = 1_728_800_000_000;

/** allStatuses with a named, pinned session, a pinned project, a quiet
 * project from history and one whose folder moved. */
const MANAGE = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) =>
    session.id === "sess-6" ? { ...session, label: "the login fix", pinned: true } : session,
  ),
  projects: [
    ...allStatuses.projects.map((project) => (project.id === "proj-2" ? { ...project, pinned: true } : project)),
    { id: "proj-4", root: "/Users/dev/old-prototype", name: "old-prototype", last_activity_ms: T0 - 9 * 86_400_000, active_instance: null, accounts_used: ["Work Max"], instance_count: 0, history_sessions: 3, history_last_used_ms: T0 - 9 * 86_400_000, repo: null, missing: false },
    { id: "proj-5", root: "/Users/dev/billing", name: "billing", last_activity_ms: T0 - 20 * 86_400_000, active_instance: null, accounts_used: ["Personal"], instance_count: 0, history_sessions: 1, history_last_used_ms: T0 - 20 * 86_400_000, repo: null, missing: true },
  ],
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", editor: string | null = null) {
  await page.addInitScript(
    ({ fixture, theme, t0, editor }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_EDITORS__ = { installed: ["Visual Studio Code", "Zed"], chosen: editor };
      w.__AIJ_PICK_DIRECTORY__ = "/Users/dev/projects/billing";
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: structuredClone(MANAGE), theme, t0: T0, editor },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("instance-board")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the rail with pins and a label — ${theme}`, gate("76-project-manage"), async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    const rail = page.getByTestId("rail");
    // The pinned project's group leads; the pinned session leads its group.
    await expect(rail.getByTestId("rail-project").first()).toContainText("storefront");
    const labelled = rail.locator('[data-instance-id="sess-6"]');
    await expect(labelled).toContainText("the login fix");
    await expect(labelled.getByTestId("pin-mark")).toBeVisible();
    await rail.screenshot({ path: resolve(OUT, `rail-pins-${theme}.png`) });
    await axeClean(page, "rail with pins");
  });

  test(`a session's row menu and the rename sheet — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    const row = page.locator('[data-instance-id="sess-2"]');
    await row.hover();
    await page.getByRole("button", { name: "Session actions for feat/read-model" }).first().click();
    const menu = page.getByRole("menu");
    // AJ-E2106: getting there first (Go to it only while it runs).
    await expect(menu.getByRole("menuitem").first()).toHaveText(/^(Go to it|Open its page)$/);
    await expect(menu.getByRole("menuitem", { name: "Rename…" })).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `session-menu-1280x800-${theme}.png`) });
    await axeClean(page, "session menu");
    await menu.getByRole("menuitem", { name: "Rename…" }).click();
    await expect(page.getByTestId("rename-session-name")).toBeFocused();
    await page.getByTestId("rename-session-name").fill("read model, second pass");
    await page.screenshot({ path: resolve(OUT, `rename-session-1280x800-${theme}.png`) });
    await axeClean(page, "rename session");
    await page.getByTestId("rename-session-save").click();
    await expect(page.getByTestId("rail")).toContainText("read model, second pass");
  });

  test(`a project's row menu, rename and forget — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    const row = page.getByTestId("rail-history-project").filter({ hasText: "old-prototype" });
    await row.hover();
    await page.getByRole("button", { name: "Project actions for old-prototype" }).click();
    const menu = page.getByRole("menu", { name: "Actions for old-prototype" });
    // AJ-E1537: the registry's project menu; Forget last, on its own.
    const names = await menu.getByRole("menuitem").evaluateAll((items) => items.map((item) => item.querySelector("[data-label]")?.textContent));
    expect(names).toEqual(expect.arrayContaining(["Open its page", "New session here", "Open in Finder", "Copy path", "Rename…", "Pin to top"]));
    expect(names.at(-1)).toBe("Forget…");
    await page.screenshot({ path: resolve(OUT, `project-menu-1280x800-${theme}.png`) });
    await axeClean(page, "project menu");
    await menu.getByRole("menuitem", { name: "Forget…" }).click();
    await expect(page.getByTestId("forget-project")).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `forget-project-1280x800-${theme}.png`) });
    await axeClean(page, "forget sheet");
    await page.getByTestId("forget-project-confirm").click();
    await expect(page.getByTestId("rail-history-project").filter({ hasText: "old-prototype" })).toHaveCount(0);
  });

  // AJ-E1537: the project page's header ⋯ is the same menu as its row's.
  test(`the project page's menu, an editor chosen — ${theme}`, async ({ page }) => {
    await seed(page, theme, "Zed");
    await projectLens(page);
    await page.getByTestId("rail").getByRole("button", { name: /^ai-juggler/ }).first().click();
    await page.getByTestId("header-menu").click();
    const menu = page.getByRole("menu", { name: "Actions for ai-juggler" });
    await expect(menu.getByRole("menuitem", { name: "Open in Zed" })).toBeVisible();
    await expect(menu.getByRole("menuitem", { name: "Show in Finder" })).toBeVisible();
    await expect(menu.getByRole("menuitem", { name: "Open its page", exact: true })).toHaveCount(0);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `project-actions-1280x800-${theme}.png`) });
    await axeClean(page, "project actions");
  });

  test(`a moved folder: Relocate… on the banner — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    await page.getByTestId("rail-history-project").filter({ hasText: "billing" }).click();
    // AJ-E1533: the missing-folder banner sits on the project's Overview.
    await page.getByRole("tablist", { name: "Project tabs" }).getByRole("tab", { name: "Overview" }).click();
    const banner = page.getByTestId("project-missing-banner");
    await expect(banner.getByRole("button", { name: "Relocate…" })).toBeEnabled();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `relocate-1280x800-${theme}.png`) });
    await axeClean(page, "relocate banner");
    await banner.getByRole("button", { name: "Relocate…" }).click();
    await expect(page.getByTestId("project-missing-banner")).toHaveCount(0);
  });

  test(`Settings: the editor projects open in — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await goTo(page, "Settings");
    const card = page.getByTestId("settings-editor");
    await expect(card.getByRole("combobox", { name: "Open projects in" })).toHaveValue("");
    await card.getByRole("combobox", { name: "Open projects in" }).selectOption("Zed");
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-editor-1280x800-${theme}.png`) });
    await axeClean(page, "settings editor");
  });
}
