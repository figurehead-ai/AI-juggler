/**
 * E14 gallery (AJ-E1404, AJ-E1405): Home's This Mac in each state, a heavy
 * session under Needs you, on the rail and in the session header — light
 * and dark, desktop and phone width, each scanned with axe (0
 * critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/72-this-mac");
const T0 = 1_728_800_000_000;

/** sess-2 (running, Work Max) is working the Mac hard. */
const heavy: FixtureSet = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((s) => (s.id === "sess-2" ? { ...s, cpu_pct: 96, rss_bytes: 3_200_000_000, child_count: 5 } : s)),
};
const calm: FixtureSet = {
  ...allStatuses,
  sessions: allStatuses.sessions.filter((s) => s.status === "running" && !s.attention),
};
const nothing: FixtureSet = { ...allStatuses, sessions: allStatuses.sessions.filter((s) => s.status === "finished") };

interface State {
  name: string;
  fixture: FixtureSet;
  machine: "plenty" | "tight" | "pressure" | "others";
  observed?: unknown[];
  leftovers?: [number, number];
  expect: string;
}

const STATES: State[] = [
  { name: "plenty", fixture: calm, machine: "plenty", expect: "Plenty of room" },
  { name: "nothing-running", fixture: nothing, machine: "plenty", observed: [], expect: "Nothing running — your Mac has room" },
  { name: "tight-heavy-leftovers", fixture: heavy, machine: "tight", leftovers: [2, 1], expect: "Getting tight" },
  { name: "pressure", fixture: calm, machine: "pressure", expect: "Under pressure" },
  // The 0.1.4 review: pressure while other apps hold most of the memory.
  { name: "pressure-from-other-apps", fixture: calm, machine: "others", expect: "Under memory pressure" },
];
const VIEWPORTS = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "390x844", width: 390, height: 844 },
] as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, state: Omit<State, "name" | "expect">, theme: "dark" | "light", vp: { width: number; height: number }) {
  await page.addInitScript(
    ({ state, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = state.fixture;
      w.__AIJ_MACHINE__ = state.machine;
      if (state.observed) w.__AIJ_OBSERVED__ = state.observed;
      if (state.leftovers) w.__AIJ_LEFTOVERS__ = state.leftovers;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { state, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const vp of VIEWPORTS) {
    for (const state of STATES) {
      test(`this mac — ${state.name} — ${vp.name} — ${theme}`, gate("72-this-mac"), async ({ page }) => {
        await seed(page, state, theme, vp);
        // AJ-E2105: This Mac's line is the top bar's; its page holds the rest.
        await page.getByTestId("machine-status").click();
        await expect(page.getByRole("main")).toContainText(state.expect);
        await page.screenshot({ path: resolve(OUT, `${state.name}-${vp.name}-${theme}.png`) });
        await axeClean(page, `this mac ${state.name}`);
      });
    }
  }

  test(`heavy session — Needs you, rail, session header — ${theme}`, async ({ page }) => {
    await seed(page, { fixture: heavy, machine: "plenty" }, theme, { width: 1280, height: 800 });
    const item = page.getByTestId("home-need-heavy");
    await expect(item).toContainText("Working your Mac hard — CPU 96% · 3.2 GB");
    // The rail shows one figure, only on the heavy row.
    await expect(page.getByTestId("rail-heavy")).toHaveCount(1);
    await expect(page.getByTestId("rail-heavy")).toContainText("3.2 GB");
    await item.getByRole("button", { name: /Take a look/ }).click();
    const header = page.getByTestId("session-resources");
    await expect(header).toContainText("CPU 96% · 3.2 GB · 5 more processes");
    await expect(header).toContainText("working your Mac hard");
    await page.screenshot({ path: resolve(OUT, `session-header-heavy-1280x800-${theme}.png`) });
    await axeClean(page, "session header heavy");
  });

  test(`Clean up leads to Processes — ${theme}`, async ({ page }) => {
    await seed(page, { fixture: calm, machine: "plenty", leftovers: [1, 0] }, theme, { width: 1280, height: 800 });
    // AJ-E2105: left-behind processes are a Needs you item on Home.
    await expect(page.getByTestId("home-need-leftovers")).toContainText("1 process left running");
    await page.getByTestId("home-need-leftovers").getByRole("button", { name: /^Clean up/ }).click();
    await expect(page.getByRole("heading", { name: "Processes", exact: false }).first()).toBeVisible();
  });
}
