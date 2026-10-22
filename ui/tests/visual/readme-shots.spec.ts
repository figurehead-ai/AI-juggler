/**
 * The README's screenshots (`just readme-shots`), both themes, from the
 * replica only: the fixture the backend's own readers made from the
 * replica HOME (tests/visual/fixtures/replica.json, `just
 * replica-fixture`, which refuses any other HOME). Never real data.
 * Skipped unless AIJ_README_SHOTS=1, so the suite never rewrites them.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/assets/readme");
const FIXTURE = JSON.parse(readFileSync(resolve(process.cwd(), "tests/visual/fixtures/replica.json"), "utf8"));

test.skip(process.env.AIJ_README_SHOTS !== "1", "README shots run only through `just readme-shots`");
test.beforeAll(() => {
  // The replica's accounts only: its folders live under the replica HOME.
  for (const account of FIXTURE.accounts as { label: string; import_source_path: string | null }[]) {
    expect(account.import_source_path ?? "", `not the replica's: ${account.label}`).toMatch(/aij-replica|\/replica/);
  }
  mkdirSync(OUT, { recursive: true });
});

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, theme, t0: FIXTURE.generated_ms },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
}

for (const theme of ["dark", "light"] as const) {
  test(`README: Home and the rail — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await expect(page.getByTestId("home-outside").first()).toBeVisible();
    await shoot(page, "home", theme);
  });

  test(`README: a project — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await page.getByTestId("rail").getByTestId("rail-project").first().click();
    await expect(page.getByTestId("page-header")).toBeVisible();
    await shoot(page, "project", theme);
  });

  test(`README: History — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "History");
    await shoot(page, "history", theme);
  });

  test(`README: Accounts — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Accounts");
    await expect(page.getByTestId("account-label").first()).toBeVisible();
    await shoot(page, "accounts", theme);
  });
}
