/**
 * 0.4.1 (walk A3): a session page's App chip keeps the part that tells
 * Apps apart. At the minimum window (1024×680), folded, the chip sits on
 * the title's line; it read "Claude Code · …", the profile cut off. The
 * tool word gives way first, whole (0.4.4, D-100: "Work Max", its mark
 * beside), the title truncates before the name does, and the chip's
 * accessible name and tooltip carry the App's full name (D-100's CLI
 * names). No captures: a layout check only.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses, expectRail, openRailList, openRailSession, terminalDrawn } from "./fixtures";

async function seed(page: Page, width: number, height: number) {
  await page.addInitScript((fixture) => {
    const w = window as unknown as Record<string, unknown>;
    w.__AIJ_FIXTURE__ = fixture;
    localStorage.setItem("aij.theme", "light");
    localStorage.setItem("aij.density", "comfortable");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, allStatuses);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "light" });
  await page.setViewportSize({ width, height });
  await page.goto("/");
  await expectRail(page);
}

/** How much of `words` shows in the chip: its text's box, against the
 * nearest box around it that clips (an ellipsis cuts what's past it). */
async function shownOf(page: Page, words: string): Promise<{ found: boolean; shown: number; width: number }> {
  return page.getByTestId("session-app-chip").evaluate((chip, words) => {
    const walker = document.createTreeWalker(chip, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const at = node.textContent?.indexOf(words) ?? -1;
      if (at < 0) continue;
      const range = document.createRange();
      range.setStart(node, at);
      range.setEnd(node, at + words.length);
      const box = range.getBoundingClientRect();
      let right = Infinity;
      for (let el = node.parentElement; el && el !== chip.parentElement; el = el.parentElement) {
        if (getComputedStyle(el).overflowX !== "visible") right = Math.min(right, el.getBoundingClientRect().right);
      }
      const shown = Math.max(0, Math.min(box.right, right) - box.left);
      return { found: true, shown, width: box.width };
    }
    return { found: false, shown: 0, width: 0 };
  }, words);
}

for (const folded of [true, false]) {
  test(`the App chip keeps its profile's name at 1024 px — ${folded ? "folded" : "unfolded"}`, async ({ page }) => {
    await seed(page, 1024, 680);
    await page.locator('[data-instance-id="sess-2"]').first().click();
    await terminalDrawn(page);
    if (folded) {
      await page.keyboard.press("Meta+Shift+I");
      await expect(page.getByTestId("right-inspector")).toHaveAttribute("data-folded", "true");
    }
    const chip = page.getByTestId("session-app-chip");
    await expect(chip).toBeVisible();
    // Its own name shows whole: no part of it cut by an ellipsis.
    const name = await shownOf(page, "Work Max");
    expect(name.found, "the chip's text holds the profile's name").toBe(true);
    expect(name.width).toBeGreaterThan(20);
    expect(name.shown, "Work Max is cut off in the chip").toBeGreaterThanOrEqual(name.width - 0.5);
    // Its accessible name and tooltip: the App's full name.
    await expect(page.getByRole("button", { name: "Open Claude Code CLI · Work Max's page", exact: true })).toBeVisible();
    await expect(page.getByTestId("session-app-page")).toHaveAttribute("title", /^Claude Code CLI · Work Max\b/);
  });
}

test("the chip's name holds while the title gives way, folded at 1024 px", async ({ page }) => {
  await seed(page, 1024, 680);
  await page.locator('[data-instance-id="sess-2"]').first().click();
  await terminalDrawn(page);
  await page.keyboard.press("Meta+Shift+I");
  await expect(page.getByTestId("right-inspector")).toHaveAttribute("data-folded", "true");
  // A long title: it truncates, the chip keeps its name.
  await page.getByTestId("page-name").evaluate((h2) => {
    h2.textContent = "Keep the old shortcuts working across every page, the palette, the rail and the folded inspector";
  });
  const title = await page.getByTestId("page-name").evaluate((h2) => ({ clipped: h2.scrollWidth > h2.clientWidth, width: h2.clientWidth }));
  expect(title.clipped).toBe(true);
  expect(title.width).toBeGreaterThan(100);
  const name = await shownOf(page, "Work Max");
  expect(name.shown, "Work Max is cut off in the chip").toBeGreaterThanOrEqual(name.width - 0.5);
});

// D-100 (0.4.4): with the list open at 1024 px, an ended session's four
// actions left its name and chips 96 px: "Keep th…" and an App chip that
// read nothing. The actions go down a line instead, their hints clear of
// the facts under them.
test("an ended session's actions go down a line before its names are cut, at 1024 px with the list open", async ({ page }) => {
  await seed(page, 1024, 680);
  await openRailList(page);
  await openRailSession(page, "sess-8");
  await expect(page.getByTestId("header-relaunch-hint")).toBeVisible();
  const whole = (id: string) => page.getByTestId(id).evaluate((node) => node.scrollWidth <= node.clientWidth);
  expect(await whole("page-name"), "the session's name is cut").toBe(true);
  expect(await whole("session-app-short"), "the App's name is cut").toBe(true);
  const box = async (id: string) => (await page.getByTestId(id).boundingBox())!;
  const chips = await box("page-chips");
  const relaunch = await box("header-relaunch");
  expect(relaunch.y, "the actions sit below the chips").toBeGreaterThanOrEqual(chips.y + chips.height);
  const hint = await box("header-relaunch-hint");
  const facts = await box("page-facts");
  expect(hint.y + hint.height, "the hint prints over the facts").toBeLessThanOrEqual(facts.y);
});

