/**
 * E14 gallery (AJ-E1408, AJ-E1409): an overload alert under Needs you and
 * as a one-time notice, and This Mac's advice from history — light and
 * dark, each scanned with axe (0 critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/74-overload");
const T0 = 1_728_800_000_000;
const calm = { ...allStatuses, sessions: allStatuses.sessions.filter((s) => s.status === "running" && !s.attention) };

const MEMORY = {
  key: "memory_pressure",
  kind: "memory_pressure",
  title: "Your Mac is under memory pressure",
  detail: "macOS reports memory pressure",
  session_id: "sess-2",
  place: null,
  subject_bytes: 3_200_000_000,
  subject_cpu: 12,
  since_ms: T0 - 45_000,
};
const CPU = {
  key: "cpu",
  kind: "cpu",
  title: "The processor has been flat out for minutes",
  detail: "97% busy for over two minutes",
  session_id: null,
  place: "desktop",
  subject_bytes: 1_600_000_000,
  subject_cpu: 140,
  since_ms: T0 - 150_000,
};
const HISTORY = [
  "This Mac comfortably runs about 5 sessions at once (about 2.1 GB each, from your last 12)",
  "“Deep work” (Work Max) is the heaviest launch setting — about 3.1 GB a session",
];

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, machine: string, alerts: unknown[], theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, machine, alerts, history, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_MACHINE__ = machine;
      w.__AIJ_ALERTS__ = alerts;
      w.__AIJ_ALERTS_FRESH__ = alerts;
      w.__AIJ_RECOMMENDATIONS__ = history;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: calm, machine, alerts, history: HISTORY, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

async function axeClean(page: Page) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`)).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`overload — memory pressure — ${theme}`, async ({ page }) => {
    await seed(page, "tight", [MEMORY], theme);
    const item = page.getByTestId("home-mac-pressure");
    await expect(item).toContainText("Your Mac is under memory pressure");
    await expect(item).toContainText("ai-juggler is using 3.2 GB");
    await expect(page.getByTestId("machine-alert")).toContainText("Your Mac is under memory pressure — macOS reports memory pressure");
    await page.screenshot({ path: resolve(OUT, `memory-pressure-1280x800-${theme}.png`) });
    await axeClean(page);
  });

  test(`overload — busy processor, the Claude app — ${theme}`, async ({ page }) => {
    await seed(page, "tight", [CPU], theme);
    await expect(page.getByTestId("machine-alert")).toContainText("The Claude app is using CPU 140%");
    await page.screenshot({ path: resolve(OUT, `busy-cpu-1280x800-${theme}.png`) });
    // Told once: dismissing clears it, and nothing else is offered.
    await page.getByTestId("machine-alert").getByRole("button", { name: "dismiss" }).click();
    await expect(page.getByTestId("machine-alert")).toHaveCount(0);
    await axeClean(page);
  });

  test(`this mac — advice from history — ${theme}`, async ({ page }) => {
    await seed(page, "plenty", [], theme);
    const advice = page.getByTestId("this-mac-recommendations");
    await expect(advice).toContainText("comfortably runs about 5 sessions at once");
    await advice.scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `recommendations-1280x800-${theme}.png`) });
    await axeClean(page);
  });
}
