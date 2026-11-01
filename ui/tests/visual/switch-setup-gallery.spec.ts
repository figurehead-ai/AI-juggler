/**
 * Switch set-up gallery (switch-setup-items.md): the switch's list of what
 * the other account lacks, in four states, both themes, axe 0
 * critical/serious on each:
 * - used and missing, can set up (ticked, and a server that needs you);
 * - used and missing, turned on by the other account, its steps open;
 * - nothing used is missing: the rest under one closed line;
 * - nothing missing at all: one line.
 * Captured is the list itself, so the switch dialog around it can change
 * without moving these.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, expectRail, openSwitch, switchRows, openWorkMaxSession } from "./fixtures";
import { EXAMPLE_SETUP, EXAMPLE_SETUP_READY, EXAMPLE_SETUP_UNUSED } from "../../src/lib/ipc/switchSetupExamples";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/96-switch-setup");

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** A one-click switch: the list sits in its only step. */
const ONE_CLICK = {
  mechanism: "transplant",
  fidelity: "F4",
  fidelity_words: "Continues exactly",
  score: 1,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: false,
  source_keeps_running: false,
  experimental_copy: "The source stays resumable.",
  door: "one-click" as const,
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: { carried: ["chat transcript"], regenerated: [], dropped: [] },
};

async function openWith(page: Page, theme: "dark" | "light", setup: unknown) {
  await page.addInitScript(
    ({ fixture, theme: t, preflight, setup: s }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_SWITCH_PREFLIGHT__ = preflight;
      w.__AIJ_SWITCH_SETUP__ = s;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => 1_728_800_000_000;
    },
    { fixture: structuredClone(allStatuses), theme, preflight: ONE_CLICK, setup },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 1400 });
  await page.goto("/");
  await expectRail(page);
  await openWorkMaxSession(page);
  await openSwitch(page);
  await switchRows(page).first().click();
  await expect(page.getByTestId("switch-flow")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).include("[data-testid='switch-flow']").analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`used and missing, can set up — ${theme}`, gate("96-switch-setup"), async ({ page }) => {
    await openWith(page, theme, EXAMPLE_SETUP_READY);
    const list = page.getByTestId("switch-setup");
    await expect(list.getByRole("checkbox", { name: /release-notes/ })).toBeChecked();
    await expect(list.getByRole("checkbox", { name: /github/ })).not.toBeChecked();
    await list.screenshot({ path: resolve(OUT, `can-set-up-${theme}.png`) });
    await axeClean(page, "the list, can set up");
  });

  test(`used and missing, turn on — ${theme}`, async ({ page }) => {
    await openWith(page, theme, EXAMPLE_SETUP);
    const list = page.getByTestId("switch-setup");
    await list.getByRole("button", { name: "Show me how to turn on computer-use" }).click();
    await expect(list.getByTestId("turn-on-steps")).toContainText("Sign in to claude.ai as Personal.");
    await expect(list.getByTestId("switch-setup-unused")).not.toHaveAttribute("open", "");
    await list.screenshot({ path: resolve(OUT, `turn-on-${theme}.png`) });
    await axeClean(page, "the list, steps open");
  });

  test(`all unused, collapsed — ${theme}`, async ({ page }) => {
    await openWith(page, theme, EXAMPLE_SETUP_UNUSED);
    const list = page.getByTestId("switch-setup");
    await expect(list.getByTestId("switch-setup-summary")).toHaveText("Personal has everything this conversation used.");
    await expect(list.getByTestId("switch-setup-unused")).toContainText("not used in this conversation");
    await list.screenshot({ path: resolve(OUT, `unused-${theme}.png`) });
    await axeClean(page, "the list, all unused");
  });

  test(`nothing missing — ${theme}`, async ({ page }) => {
    await openWith(page, theme, { ...EXAMPLE_SETUP, offers: [] });
    // Nothing missing: no list before the switch (the screen stays
    // short); Details says the target has it all.
    await expect(page.getByTestId("switch-setup")).toHaveCount(0);
    const details = page.getByTestId("switch-details");
    if ((await details.getAttribute("open")) === null) await details.locator(":scope > summary").click();
    await expect(details).toContainText("Personal already has 3 of the 3 things this account has set up.");
    await page.getByTestId("switch-oneclick").screenshot({ path: resolve(OUT, `nothing-missing-${theme}.png`) });
    await axeClean(page, "nothing missing");
  });
}
