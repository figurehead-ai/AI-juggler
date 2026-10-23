/**
 * AJ-E1529 — the journey on the replica, with what the backend's own
 * readers returned (tests/visual/fixtures/replica.json, made by
 * `just replica-fixture`, never by hand): every live session started
 * outside AI Juggler is listed and counted, the same number everywhere —
 * Home, the rail and its Running chip, each Accounts card, the go-to
 * palette — and a project's page lists its conversations.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

type Conversation = { id: string; account_id: string; project_id: string | null; live: boolean; session_id: string | null };
type Fixture = {
  accounts: { id: string; label: string }[];
  projects: { id: string; name: string }[];
  conversations: Conversation[];
  generated_ms: number;
};

const FIXTURE: Fixture = JSON.parse(readFileSync(resolve(process.cwd(), "tests/visual/fixtures/replica.json"), "utf8"));
const LIVE = FIXTURE.conversations.filter((c) => c.live && c.session_id === null);

async function seed(page: Page) {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, t0: FIXTURE.generated_ms },
  );
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

test("the replica has live sessions started outside AI Juggler to show", () => {
  expect(LIVE.length).toBeGreaterThan(0);
  expect(FIXTURE.projects.length).toBeGreaterThanOrEqual(13);
});

test("every live outside session: on Home, in the rail and its Running chip, in the palette", async ({ page }) => {
  await seed(page);
  await expect(page.getByTestId("home-outside")).toHaveCount(LIVE.length);
  await expect(page.getByText(`${LIVE.length} running`).first()).toBeVisible();
  const railLive = page.getByTestId("rail-outside").filter({ hasText: "Running" });
  await expect(railLive).toHaveCount(LIVE.length);
  await expect(page.getByRole("group", { name: "Show" }).getByRole("button", { name: new RegExp(`^Running ${LIVE.length}`) })).toBeVisible();
  await page.keyboard.press("Meta+k");
  await expect(page.locator('[data-testid^="goto-row-outside::"]')).toHaveCount(LIVE.length);
  await page.keyboard.press("Escape");
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((v) => v.impact === "critical" || v.impact === "serious").map((v) => v.id)).toEqual([]);
});

test("each account's card counts its live outside sessions", async ({ page }) => {
  await seed(page);
  await goTo(page, "Accounts");
  for (const account of FIXTURE.accounts) {
    const live = LIVE.filter((c) => c.account_id === account.id).length;
    const label = page.getByTestId("account-label").filter({ hasText: new RegExp(`^${account.label}$`) });
    const card = page.locator(".rounded-card", { has: label }).getByTestId("account-activity");
    if ((await card.count()) === 0) continue; // a card not signed in shows its sign-in instead
    await expect(card.first()).toHaveText(live > 0 ? new RegExp(`^${live} running`) : /Nothing running/);
  }
});

test("a project's page lists its conversations started outside AI Juggler", async ({ page }) => {
  await seed(page);
  const project = FIXTURE.projects.find((p) => FIXTURE.conversations.some((c) => c.project_id === p.id && c.session_id === null))!;
  const count = FIXTURE.conversations.filter((c) => c.project_id === project.id && c.session_id === null).length;
  await page.getByTestId("rail").getByRole("button", { name: new RegExp(`^${project.name}`) }).first().click();
  // AJ-E1533: the project's body (its Sessions) lists them beside its own.
  const body = page.getByTestId("page-panel-sessions");
  await expect(body.getByTestId("page-sessions")).toBeVisible();
  const all = body.getByTestId("page-sessions-all");
  if (await all.count()) await all.click();
  await expect(body.locator('[data-testid="page-session-row"][data-outside="true"]')).toHaveCount(count);
  await expect(body.locator('[data-testid="page-session-row"][data-outside="true"]').first()).toContainText("outside AI Juggler");
});
