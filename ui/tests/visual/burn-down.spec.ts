/**
 * AJ-E1512 gallery: Usage — each account's sessions left in its window
 * (with the confidence word) and the burn-down: used so far, the pace's
 * projection to running out before the reset, and the reset. Both themes
 * and both densities; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/87-burn-down");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light", density: "comfortable" | "compact") {
  await page.addInitScript(
    ([theme, density]) => {
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.density", density);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
      const now = Date.now();
      const H = 3_600_000;
      const start = now - 2 * H;
      const win = (over: Record<string, unknown>) => ({
        state: "tracking", percentage: 40, badge: "measured", stale_note_ms: null, resets_at_ms: start + 5 * H,
        reset_basis: "measured", measured_at_ms: now, burned_tokens: 400_000, burned_credits: null, headroom_tokens: null, ...over,
      });
      (window as unknown as { __AIJ_QUOTA_DISPLAY__: unknown }).__AIJ_QUOTA_DISPLAY__ = [
        {
          account: "claude-work",
          five_hour: win({
            runway: {
              series: Array.from({ length: 13 }, (_, i) => [start + i * 600_000, (i * 40) / 12]),
              starts_ms: start, resets_at_ms: start + 5 * H, used_pct: 40, pct_per_hour: 20,
              exhausts_at_ms: now + 3 * H - 30 * 60_000 > start + 5 * H ? null : now + 3 * H - 30 * 60_000,
              session_pct: 10, sessions_left: 6, sessions_counted: 5, badge: "computed",
            },
          }),
          five_hour_omitted_reason: null,
          weekly: win({
            percentage: 22, resets_at_ms: now + 3 * 24 * H,
            runway: {
              series: [[now - 4 * 24 * H, 2], [now - 2 * 24 * H, 12], [now, 22]], starts_ms: now - 4 * 24 * H,
              resets_at_ms: now + 3 * 24 * H, used_pct: 22, pct_per_hour: null, exhausts_at_ms: null,
              session_pct: null, sessions_left: null, sessions_counted: 2, badge: "computed",
            },
          }),
          weekly_omitted_reason: null,
          renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
          forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
          unavailable_reason: null,
          credits: null,
        },
      ];
    },
    [theme, density],
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const density of ["comfortable", "compact"] as const) {
    test(`Usage shows sessions left and the burn-down — ${theme}, ${density}`, gate("87-burn-down"), async ({ page }) => {
      await open(page, theme, density);
      await goTo(page, "Usage");
      const runways = page.getByTestId("burn-downs");
      await expect(runways.getByTestId("sessions-left").first()).toHaveText("About 6 usual sessions left (each about 10%)");
      await expect(runways.getByTestId("sessions-left-tier").first()).toHaveText("worked out");
      await expect(runways.getByTestId("burn-down-pace").first()).toContainText("At this pace it runs out about");
      await expect(runways.getByTestId("sessions-left").nth(1)).toHaveText("Sessions left show after a few sessions on this account");
      await expect(runways.getByTestId("burn-down-projection")).toHaveCount(1);
      await runways.scrollIntoViewIfNeeded();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `usage-1280x800-${theme}-${density}.png`) });
      await axe(page, `burn-down ${theme} ${density}`);
    });
  }
}
