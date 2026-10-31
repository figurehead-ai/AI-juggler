/**
 * E14 gallery (AJ-E1407): the New session sheet's "Has this Mac got
 * room?" — OK (learned), not measured yet, Tight (with what would make
 * room) and Not now — light and dark, each scanned with axe.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/73-launch-advisor");
const T0 = 1_728_800_000_000;

/** sess-2 is heavy, so Tight and Not now can name what would make room. */
const heavy = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((s) => (s.id === "sess-2" ? { ...s, cpu_pct: 96, rss_bytes: 3_200_000_000, child_count: 5 } : s)),
};

const STATES = [
  { name: "ok", machine: "plenty", expect: "This Mac can take another session", verdict: "ok" },
  { name: "not-measured", machine: "plenty", advice: "unmeasured", expect: "not measured yet", verdict: "ok" },
  { name: "tight", machine: "tight", expect: "Closing feat/read-model frees about 3.0 GB.", verdict: "tight" },
  { name: "no", machine: "pressure", expect: "You can still start it.", verdict: "no" },
] as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, machine: string, unmeasured: boolean, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, machine, unmeasured, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_MACHINE__ = machine;
      if (unmeasured) {
        w.__AIJ_ADVICE__ = (fixture as { profiles: Array<{ id: string }> }).profiles.map((p) => ({
          recipe_id: p.id,
          verdict: "ok",
          words: "This Mac can take another session",
          reasons: [],
          projected_bytes: 1_000_000_000,
          based_on: 0,
          relief_session_id: null,
          relief_bytes: null,
        }));
      }
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: heavy, machine, unmeasured, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await page.getByRole("button", { name: "New session" }).first().click();
  await expect(page.getByTestId("new-session-sheet")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  for (const state of STATES) {
    test(`launch advisor — ${state.name} — ${theme}`, gate("73-launch-advisor"), async ({ page }) => {
      await open(page, state.machine, "advice" in state, theme);
      const line = page.getByTestId("machine-advice");
      await expect(line).toHaveAttribute("data-verdict", state.verdict);
      await expect(line).toContainText(state.expect);
      // Warns, never blocks.
      await expect(page.getByTestId("launch-now")).toBeEnabled();
      await page.screenshot({ path: resolve(OUT, `${state.name}-1280x800-${theme}.png`) });
      const results = await new AxeBuilder({ page }).analyze();
      const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
      expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`)).toEqual([]);
    });
  }
}
