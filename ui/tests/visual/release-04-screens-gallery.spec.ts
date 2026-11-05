/**
 * 0.4's other new screens, one family each: the switch's "leave out the
 * start" choice (D-76), Report a bug (AJ-E2911), Debug mode (AJ-E2912),
 * Notifications › When to tell me (AJ-E2904), an ended session's Resume
 * (AJ-E2908), Home's chips (AJ-E2910) and the rail's edge from the
 * keyboard (AJ-E2909). Both themes, 1280×800 and 1024×700; axe 0
 * critical/serious on each.
 */
import { gate, test, expect } from "./axe-gallery";
import { allStatuses, goTo, openSwitch, openUserMenu, openWorkMaxSession, settingsPane, switchRows } from "./fixtures";
import { at, family, open, shot, SIZES, THEMES } from "./gallery-04";

const CARRY = family("99-carry-choice");
const REPORT = family("99-report-bug");
const DEBUG = family("99-debug-mode");
const WHEN = family("99-when-to-tell");
const RESUME = family("99-resume");
const CHIPS = family("99-home-chips");
const EDGE = family("99-rail-splitter");

for (const theme of THEMES) {
  for (const size of SIZES) {
    const where = `${at(size)}-${theme}`;
    const first = at(size) === "1280x800";

    test(`Carry choice — ${where}`, first ? gate("99-carry-choice") : {}, async ({ page }) => {
      await page.addInitScript(() => {
        (window as unknown as Record<string, unknown>).__AIJ_SWITCH_CUT__ = (cut: { keep_last: number | null } | null) => {
          const left = cut?.keep_last ? 1000 - cut.keep_last : 0;
          return {
            supported: true, turns: 1000, first_ms: Date.UTC(2024, 6, 1), last_ms: Date.UTC(2024, 9, 12), bytes: 42_000_000,
            turns_left_out: left, bytes_left_out: left * 42_000, from_ms: left ? Date.UTC(2024, 9, 1) : null, at_compaction: left > 0,
            words: left ? `Carries the last ${cut?.keep_last} turns (8.4 MB) and leaves out ${left} turns.` : "Carries the whole conversation: 1,000 turns (42 MB).",
            loses: left ? "The new session won't know what was said or decided before 1 October." : null,
          };
        };
      });
      await open(page, theme, size);
      await openWorkMaxSession(page);
      await openSwitch(page);
      await switchRows(page).first().click();
      const carry = page.getByTestId("switch-carry-choice");
      // The wizard shows it on its last step, Confirm.
      await expect(page.getByTestId("switch-flow")).toBeVisible();
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-steps")).toContainText("Step 2 of 3");
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-step-confirm")).toBeVisible();
      await expect(carry).toBeVisible();
      await carry.locator("summary").click();
      await page.getByTestId("carry-last").check();
      await expect(page.getByTestId("carry-loses")).toBeVisible();
      await carry.scrollIntoViewIfNeeded();
      await shot(page, CARRY, `last-${where}`);
    });

    test(`Report a bug — ${where}`, first ? gate("99-report-bug") : {}, async ({ page }) => {
      await open(page, theme, size);
      await openUserMenu(page);
      await page.getByRole("menu", { name: "Your menu" }).getByRole("menuitem", { name: /Report a bug/ }).click();
      await expect(page.getByTestId("report-bug")).toBeVisible();
      await page.getByTestId("report-title").fill("The switch stopped at its second step");
      await shot(page, REPORT, `sheet-${where}`);
    });

    test(`Debug mode — ${where}`, first ? gate("99-debug-mode") : {}, async ({ page }) => {
      await open(page, theme, size);
      await goTo(page, "Settings");
      await settingsPane(page, "Advanced");
      const section = page.locator("#settings-debug-mode");
      await section.scrollIntoViewIfNeeded();
      await expect(section).toBeVisible();
      await shot(page, DEBUG, `section-${where}`);
    });

    test(`When to tell me — ${where}`, first ? gate("99-when-to-tell") : {}, async ({ page }) => {
      await open(page, theme, size);
      await goTo(page, "Settings");
      await settingsPane(page, "Notifications");
      const when = page.getByTestId("when-to-tell");
      await when.scrollIntoViewIfNeeded();
      await expect(when).toBeVisible();
      await shot(page, WHEN, `section-${where}`);
    });

    test(`Resume — ${where}`, first ? gate("99-resume") : {}, async ({ page }) => {
      await open(page, theme, size);
      const recent = page.getByTestId("home-recent");
      await expect(recent).toBeVisible();
      await recent.locator("[data-home-row]").first().click();
      await expect(page.getByTestId("header-resume")).toBeVisible();
      await shot(page, RESUME, `ended-page-${where}`);
    });

    test(`Home's chips — ${where}`, first ? gate("99-home-chips") : {}, async ({ page }) => {
      await open(page, theme, size, { __AIJ_FIXTURE__: allStatuses });
      const chips = page.getByTestId("home-chips");
      await expect(chips).toBeVisible();
      await shot(page, CHIPS, `home-${where}`);
      // One chip picked: the list narrows, the counts elsewhere don't.
      await chips.getByRole("radio").nth(1).click();
      await shot(page, CHIPS, `narrowed-${where}`);
    });

    test(`The rail's edge — ${where}`, first ? gate("99-rail-splitter") : {}, async ({ page }) => {
      // At 1024 the rail folds to icons and has no edge to drag.
      test.skip(!first, "the folded rail has no edge");
      await open(page, theme, size);
      const edge = page.getByTestId("rail-splitter");
      await edge.focus();
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("ArrowRight");
      await expect(edge).toBeFocused();
      await shot(page, EDGE, `focused-${where}`);
    });
  }
}
