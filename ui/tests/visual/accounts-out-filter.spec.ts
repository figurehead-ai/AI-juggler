/**
 * 0.4.2: the top bar's "N accounts out" lands on what it says. A click
 * opens Accounts on its Accounts tab (even when Apps was the tab last
 * chosen), showing only the out Accounts under an "Out · N" chip, each
 * saying when it's back (the Out mark's words) and its best action; ×
 * shows every Account again, the out ones first. No captures: a flow
 * check only.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses } from "./fixtures";

const T0 = 1_728_800_000_000;
const reading = (account: string, left: number) => ({
  account,
  five_hour: { state: left === 0 ? "exhausted" : "tracking", percentage: 100 - left, resets_at_ms: T0 + 2 * 3_600_000 + 14 * 60_000, reset_basis: "measured", measured_at_ms: T0 - 60_000, badge: "measured", burned_tokens: 0 },
  weekly: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "measured" },
  unavailable_reason: null,
  credits: null,
});

async function open(page: Page) {
  await page.addInitScript(
    ({ fixture, displays, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_QUOTA_DISPLAY__ = displays;
      Date.now = () => t0;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      // The tab last chosen: Apps. The count's click lands on Accounts all the same.
      localStorage.setItem("aij.accounts.view", "apps");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: allStatuses, displays: [reading("claude-work", 0), reading("claude-personal", 70)], t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

test("the top bar's accounts-out count opens Accounts filtered to the out ones; × shows all, out first", async ({ page }) => {
  await open(page);
  const count = page.getByTestId("header-room-count");
  await expect(count).toHaveAttribute("data-kind", "out");
  await count.click();
  const chip = page.getByTestId("accounts-filter");
  await expect(chip).toHaveText(/Out · 1/);
  await expect(page.getByRole("radio", { name: /^Accounts/ })).toHaveAttribute("aria-checked", "true");
  const cards = page.getByTestId("account-card");
  await expect(cards).toHaveCount(1);
  await expect(cards.first().getByTestId("account-out-mark")).toHaveText(/^Out · resets /);
  await expect(cards.first().getByTestId("account-out-action")).toHaveText(/^New session on /);
  // × clears it: every Account, the out one first.
  await chip.getByRole("button", { name: "Clear the Out filter: show every account" }).click();
  await expect(chip).toHaveCount(0);
  await expect(cards.nth(1)).toBeVisible();
  await expect(cards.first().getByTestId("account-out")).toBeVisible();
  // Opened from the top bar again while Accounts is open: filtered again.
  await count.click();
  await expect(page.getByTestId("accounts-filter")).toHaveText(/Out · 1/);
  await expect(cards).toHaveCount(1);
});
