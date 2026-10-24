/**
 * AJ-E1501 gallery: managing accounts — the account card's menu, the
 * rename sheet (a duplicate refused in words), the remove sheet for each
 * kind of account (used where it is; a copy, with the Trash chosen; one
 * with open sessions), and the renamed account in the rail and on Usage.
 * Both themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, openFilters } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/75-account-manage");
const T0 = 1_728_800_000_000;

/** allStatuses plus an account used where it is and a copy, neither
 * with a session open. */
export const MANAGE = {
  ...allStatuses,
  accounts: [
    ...allStatuses.accounts,
    { id: "claude-mine", provider: "claude-code", label: "Mine", plan: null, login_state: "logged_in", in_place: true, import_source_path: "/Users/you/.claude" },
    { id: "claude-max", provider: "claude-code", label: "personal-max", plan: "max-5x", login_state: "logged_in", in_place: false, import_source_path: "/Users/you/Library/Application Support/ClaudeCodeProfiles/max" },
  ],
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: structuredClone(MANAGE), theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await goTo(page, "Accounts");
  await expect(page.getByTestId("account-runway").first()).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

function card(page: Page, label: string) {
  return page.getByRole("button", { name: `Account actions for ${label}`, exact: true });
}

for (const theme of ["dark", "light"] as const) {
  test(`account menu — ${theme}`, gate("75-account-manage"), async ({ page }) => {
    await seed(page, theme);
    await card(page, "personal-max").click();
    const menu = page.getByRole("menu", { name: "Actions for personal-max" });
    // AJ-E1537: the registry's account menu; destructive last.
    const names = await menu.getByRole("menuitem").evaluateAll((items) => items.map((item) => item.querySelector("[data-label]")?.textContent));
    expect(names[0]).toBe("Open its page");
    expect(names.slice(-4)).toEqual(["Rename…", "Copy settings from…", "Make the default", "Remove…"]);
    await expect(menu.getByRole("menuitem", { name: "Open its page", exact: true })).toBeFocused();
    await page.screenshot({ path: resolve(OUT, `account-menu-1280x800-${theme}.png`) });
    await axeClean(page, "account menu");
  });

  test(`rename — a duplicate refused, then saved — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await card(page, "personal-max").click();
    await page.getByRole("menuitem", { name: "Rename…" }).click();
    const name = page.getByTestId("rename-account-label");
    await expect(name).toBeFocused();
    await name.fill("work max");
    await page.getByTestId("rename-account-save").click();
    await expect(page.getByRole("alert")).toHaveText("Another account is already called «Work Max» — pick another name");
    await page.screenshot({ path: resolve(OUT, `rename-duplicate-1280x800-${theme}.png`) });
    await axeClean(page, "rename sheet");

    await name.fill("Max (personal)");
    await page.getByTestId("rename-account-save").click();
    await expect(page.getByTestId("accounts-notice")).toHaveText("Renamed to Max (personal).");
    await expect(page.getByTestId("account-label").filter({ hasText: "Max (personal)" })).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `renamed-accounts-1280x800-${theme}.png`) });

    // Everywhere it shows, with no restart: Usage, and the rail by account.
    await goTo(page, "Usage");
    await expect(page.getByTestId("usage-account").filter({ hasText: "Max (personal)" })).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `renamed-usage-1280x800-${theme}.png`) });
  });

  test(`make the default — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await card(page, "Mine").click();
    await page.getByRole("menuitem", { name: "Make the default" }).click();
    await expect(page.getByTestId("account-default")).toBeVisible();
    await expect(page.getByTestId("accounts-notice")).toContainText("New sessions start with Mine");
    await page.screenshot({ path: resolve(OUT, `default-1280x800-${theme}.png`) });
    await axeClean(page, "default account");
  });

  test(`remove — used where it is — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await card(page, "Mine").click();
    await page.getByRole("menuitem", { name: "Remove…" }).click();
    const sheet = page.getByTestId("remove-account");
    await expect(sheet).toHaveAttribute("data-kind", "in-place");
    await expect(sheet).toContainText("Your folder stays exactly as it is");
    await expect(page.getByTestId("remove-folder-trash")).toHaveCount(0);
    await page.screenshot({ path: resolve(OUT, `remove-in-place-1280x800-${theme}.png`) });
    await axeClean(page, "remove sheet (in place)");
    await page.getByTestId("remove-account-confirm").click();
    await expect(page.getByTestId("accounts-notice")).toHaveText("Removed Mine. Your folder wasn't touched.");
    await expect(card(page, "Mine")).toHaveCount(0);
  });

  test(`remove — a copy, to the Trash — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await card(page, "personal-max").click();
    await page.getByRole("menuitem", { name: "Remove…" }).click();
    await expect(page.getByTestId("remove-account")).toHaveAttribute("data-kind", "copy");
    await expect(page.getByTestId("remove-folder-keep")).toBeChecked();
    await page.getByTestId("remove-folder-trash").check();
    await page.screenshot({ path: resolve(OUT, `remove-copy-1280x800-${theme}.png`) });
    await axeClean(page, "remove sheet (copy)");
    await page.getByTestId("remove-account-confirm").click();
    await expect(page.getByTestId("accounts-notice")).toHaveText("Removed personal-max. Its folder is in the Trash.");
  });

  test(`remove — refused while sessions are open — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await card(page, "Work Max").click();
    await page.getByRole("menuitem", { name: "Remove…" }).click();
    await expect(page.getByTestId("remove-account-open")).toContainText("sessions open");
    await expect(page.getByTestId("remove-account-confirm")).toHaveCount(0);
    await page.screenshot({ path: resolve(OUT, `remove-refused-1280x800-${theme}.png`) });
    await axeClean(page, "remove sheet (refused)");
  });

  test(`the rail by account after a rename — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await card(page, "Personal").click();
    await page.getByRole("menuitem", { name: "Rename…" }).click();
    await page.getByTestId("rename-account-label").fill("Side projects");
    await page.getByTestId("rename-account-save").click();
    await expect(page.getByTestId("accounts-notice")).toHaveText("Renamed to Side projects.");
    await openFilters(page);
    await page.getByRole("combobox", { name: "Group by" }).selectOption("account");
    await expect(page.getByTestId("rail")).toContainText("Side projects");
    await page.getByTestId("rail").screenshot({ path: resolve(OUT, `renamed-rail-${theme}.png`) });
  });
}
