/**
 * E14 gallery (AJ-E1408, AJ-E1409): an overload alert under Needs you and
 * as a one-time notice, and This Mac's advice from history — light and
 * dark, each scanned with axe (0 critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, openRailList, projectLens } from "./fixtures";

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
  subject_name: null,
  subject_bytes: 3_200_000_000,
  subject_estimate: false,
  // The backend sends only what's high: a memory alert carries no CPU.
  subject_cpu: null,
  subject_windows: null,
  other_apps: [],
  since_ms: T0 - 45_000,
};
/** The 0.1.4 review: other apps hold most of the memory, so the alert
 * names them and no AI tool. */
const OTHERS = {
  ...MEMORY,
  session_id: null,
  subject_bytes: null,
  other_apps: [
    { name: "next-server", rss_bytes: 6_400_000_000, processes: 1 },
    { name: "Docker", rss_bytes: 4_000_000_000, processes: 3 },
    { name: "fseventsd", rss_bytes: 1_500_000_000, processes: 1 },
  ],
};
const CPU = {
  key: "cpu",
  kind: "cpu",
  title: "The processor has been flat out for minutes",
  detail: "97% busy for over two minutes",
  session_id: null,
  // A tool AI Juggler didn't start: its This Mac group, in its words.
  place: "claude-app",
  subject_name: "The Claude app",
  // … and a CPU alert no memory.
  subject_bytes: null,
  subject_estimate: false,
  subject_cpu: 140,
  subject_windows: null,
  other_apps: [],
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

/** Opens a session from the rail: a page an overload banner once sat over. */
async function openSession(page: Page, title: RegExp) {
  // AJ-E2104: the rows named by state first are the place lens's; these
  // read the project board's. (The lens reloads the page, so the list is
  // opened after it: a narrow window folds the rail again on load.)
  await projectLens(page);
  await openRailList(page);
  await page.getByTestId("instance-board").getByRole("button", { name: title }).first().click();
}

async function axeClean(page: Page) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`)).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`overload — memory pressure — ${theme}`, gate("74-overload"), async ({ page }) => {
    await seed(page, "tight", [MEMORY], theme);
    const item = page.getByTestId("home-need-mac");
    await expect(item).toContainText("Your Mac is under memory pressure");
    await expect(item).toContainText("ai-juggler is using 3.0 GB");
    // Home says it once, in Needs you: no banner; the headline counts it
    // (AJ-E2105), its one count and then what it is (D-118).
    await expect(page.getByTestId("home-scroll").getByRole("heading", { level: 1 })).toHaveText("1 needs you: Your Mac is under memory pressure");
    await expect(page.getByTestId("machine-alert")).toHaveCount(0);
    // (The rail's Needs you lists it too, from the same count: F2.)
    await expect(page.getByTestId("home-scroll").getByText("Your Mac is under memory pressure", { exact: true })).toHaveCount(1);
    await page.screenshot({ path: resolve(OUT, `memory-pressure-1280x800-${theme}.png`) });
    await axeClean(page);
  });

  test(`overload — memory pressure from other apps — ${theme}`, async ({ page }) => {
    await seed(page, "others", [OTHERS], theme);
    const item = page.getByTestId("home-need-mac");
    await expect(item).toContainText("Your Mac is under memory pressure");
    await expect(item).toContainText("Other apps are using most of the memory: next-server 6.0 GB, Docker about 3.7 GB, fseventsd 1.4 GB");
    await expect(item).not.toContainText("Claude app");
    await page.screenshot({ path: resolve(OUT, `memory-pressure-other-apps-1280x800-${theme}.png`) });
    await axeClean(page);
  });

  // D-87 (7eb4794cf, the rc4 walk): an overload alert is never a banner
  // across a page (one came and went under load and moved the page under
  // the pointer; the 0.0.12 report's clipped the headline under it). Home's
  // Needs you and the bell tell it; a session's page starts at its own
  // head, at any width. (Its captures keep their names.)
  for (const width of [1280, 1024]) {
    test(`overload — no banner above a session, ${width} — ${theme}`, async ({ page }) => {
      await seed(page, "tight", [MEMORY], theme);
      await page.setViewportSize({ width, height: 800 });
      await openSession(page, /Working, fake-desktop on Fake Desk/);
      await expect(page.getByRole("main").getByRole("heading", { name: "Sketch the pricing page" }).first()).toBeVisible();
      await expect(page.getByTestId("machine-alert")).toHaveCount(0);
      await page.screenshot({ path: resolve(OUT, `banner-above-session-${width}x800-${theme}.png`) });
      await axeClean(page);
    });
  }

  test(`overload — busy processor, the Claude app — ${theme}`, async ({ page }) => {
    await seed(page, "tight", [CPU], theme);
    // Home's Needs you names it in its adapter's words; no banner.
    await expect(page.getByTestId("home-need-mac")).toContainText("The Claude app is using CPU 140%");
    await expect(page.getByTestId("machine-alert")).toHaveCount(0);
    await page.screenshot({ path: resolve(OUT, `busy-cpu-1280x800-${theme}.png`) });
    await axeClean(page);
  });

  test(`this mac — advice from history — ${theme}`, async ({ page }) => {
    await seed(page, "plenty", [], theme);
    // AJ-E2105: This Mac's page (its line in the top bar), not Home.
    await page.getByTestId("machine-status").click();
    const advice = page.getByTestId("this-mac-recommendations");
    await expect(advice).toContainText("comfortably runs about 5 sessions at once");
    await advice.scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `recommendations-1280x800-${theme}.png`) });
    await axeClean(page);
  });
}
