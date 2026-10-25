/**
 * AJ-E1533/E1534 gallery: every type's page in the one page model, on
 * the replica (tests/visual/fixtures/replica.json, made by the backend's
 * own readers) with three of AI Juggler's own sessions on its first
 * account and project: a session (its Terminal and its Usage), a project
 * (its Sessions and its Overview), an account, a tool, and a session
 * started outside AI Juggler. 1280 and 1024 wide, both themes; axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, openOutsidePage, projectLens, settingsPane } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/79-pages");
const REPLICA = JSON.parse(readFileSync(resolve(process.cwd(), "tests/visual/fixtures/replica.json"), "utf8"));
const ACCOUNT = REPLICA.accounts[0];
const PROJECT = REPLICA.projects[0];
const FIXTURE = {
  ...REPLICA,
  sessions: allStatuses.sessions!.slice(0, 3).map((session) => ({
    ...session,
    account_id: ACCOUNT.id,
    account_label: ACCOUNT.label,
    provider: ACCOUNT.provider,
    project_id: PROJECT.id,
    cwd: PROJECT.root,
    started_at_ms: REPLICA.generated_ms - 40 * 60_000,
    last_activity_ms: REPLICA.generated_ms - 60_000,
    // On the replica's clock, not the shared fixture's.
    ...(session.attention ? { attention: { ...session.attention, since_ms: REPLICA.generated_ms - 5 * 60_000 } } : {}),
  })),
};
const VIEWPORTS = [
  { width: 1280, height: 800 },
  { width: 1024, height: 680 },
] as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", viewport: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, theme, t0: REPLICA.generated_ms },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(viewport);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string, viewport: { width: number; height: number }) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${viewport.width}x${viewport.height}-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

async function openSession(page: Page) {
  await page.locator("[data-testid='rail'] [data-instance-id]").first().click();
  await expect(page.getByTestId("page-type")).toHaveText("Session");
}

async function openProject(page: Page) {
  await projectLens(page);
  await page.getByTestId("rail").getByTestId("rail-project").filter({ hasText: PROJECT.name }).first().click();
  await expect(page.getByTestId("page-type")).toHaveText("Project");
}

for (const viewport of VIEWPORTS) {
  for (const theme of ["dark", "light"] as const) {
    const at = `${viewport.width} — ${theme}`;

    test(`a session's page: its terminal — ${at}`, gate("79-pages"), async ({ page }) => {
      await seed(page, theme, viewport);
      await openSession(page);
      await page.getByRole("tab", { name: "Terminal" }).click();
      await shoot(page, "session-terminal", theme, viewport);
    });

    test(`a session's page: its usage, quality in it — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await openSession(page);
      await page.getByRole("tab", { name: "Usage" }).click();
      await expect(page.getByTestId("instance-usage")).toBeVisible();
      await shoot(page, "session-usage", theme, viewport);
    });

    test(`a project's page: its sessions — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await openProject(page);
      await page.getByRole("tab", { name: "Sessions" }).click();
      // The Sessions tab's list (Overview's Open sessions pages one too).
      await expect(page.getByTestId("page-panel-sessions").getByTestId("page-sessions")).toBeVisible();
      await shoot(page, "project-sessions", theme, viewport);
    });

    test(`a project's page: its overview — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await openProject(page);
      await page.getByRole("tab", { name: "Overview" }).click();
      await expect(page.getByTestId("project-overview")).toBeVisible();
      await shoot(page, "project-overview", theme, viewport);
    });

    test(`an account's page, from its name on Accounts — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await goTo(page, "Accounts");
      await page.getByTestId("account-label").filter({ hasText: new RegExp(`^${ACCOUNT.label}$`) }).first().click();
      await expect(page.getByTestId("page-type")).toHaveText("Account");
      await shoot(page, "account", theme, viewport);
    });

    // 0.1.5: the tool page folded into Settings › Accounts (Tools on this Mac).
    test(`a tool, in Settings › Accounts — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      await goTo(page, "Settings");
      await settingsPane(page, "Accounts");
      await expect(page.getByTestId("settings-tool").first()).toBeVisible();
      await shoot(page, "tool", theme, viewport);
    });

    test(`a session started outside AI Juggler — ${at}`, async ({ page }) => {
      await seed(page, theme, viewport);
      // A rail row opens its page here; its ↗ goes to its app.
      await openOutsidePage(page);
      await expect(page.getByTestId("conversation-page")).toBeVisible();
      await shoot(page, "outside-session", theme, viewport);
    });
  }
}
