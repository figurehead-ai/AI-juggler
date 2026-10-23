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
// The released version the shots show (`just readme-shots` passes the
// newest release tag); the source tree's own version is a placeholder.
const VERSION: string =
  process.env.AIJ_README_VERSION ||
  JSON.parse(readFileSync(resolve(process.cwd(), "../crates/juggler-app/tauri.conf.json"), "utf8")).version;

/** The replica lives in a temporary folder; a reader's Mac has a home.
 * Only the shown paths move (after the check below has seen the real
 * ones), from the replica's HOME to `/Users/you`. */
function asAtHome(fixture: unknown): unknown {
  const text = JSON.stringify(fixture);
  return JSON.parse(text.replace(/(\/private)?\/tmp\/aij-replica[\w-]*/g, "/Users/you"));
}

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
    ({ fixture, theme, t0, version }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_QUOTA_DISPLAY__ = (fixture as { quota?: unknown }).quota ?? [];
      w.__AIJ_RECIPES__ = (fixture as { profiles?: unknown }).profiles ?? [];
      w.__AIJ_APP_INFO__ = { version, name: "AI Juggler", edition: "main" };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      // A reader who's past the optional permissions: no reminder card.
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: asAtHome(FIXTURE), theme, t0: FIXTURE.generated_ms, version: VERSION },
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
