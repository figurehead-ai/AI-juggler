/**
 * AJ-E2902 gallery: Usage's cost cuts (per finished task, crashed or
 * probably stuck, Cost by account with its top project and price), the
 * wasted sessions' sheet, and Last week's review. Both themes, 1280×800
 * and 1024×700; axe 0 critical/serious on each.
 */
import { gate, test, expect } from "./axe-gallery";
import { goTo } from "./fixtures";
import { at, family, open, shot, SIZES, THEMES } from "./gallery-04";

const OUT = family("99-usage-cost");

for (const theme of THEMES) {
  for (const size of SIZES) {
    const where = `${at(size)}-${theme}`;
    test(`Cost you can act on — ${where}`, at(size) === "1280x800" ? gate("99-usage-cost") : {}, async ({ page }) => {
      await open(page, theme, size);
      await goTo(page, "Usage");
      const cuts = page.getByTestId("usage-cuts");
      await expect(page.getByTestId("tile-per-task")).toContainText("finished");
      await cuts.scrollIntoViewIfNeeded();
      await expect(page.getByTestId("cost-by-account-row").first()).toBeVisible();
      await shot(page, OUT, `cuts-${where}`);
      await page.getByTestId("usage-wasted-open").click();
      await expect(page.getByTestId("usage-wasted-row").first()).toBeVisible();
      await shot(page, OUT, `wasted-${where}`);
      await page.keyboard.press("Escape");
      await page.getByTestId("usage-review-open").click();
      await expect(page.getByTestId("review-cost")).toContainText("~$");
      await expect(page.getByTestId("cost-by-account-row").last()).toBeVisible();
      await shot(page, OUT, `review-${where}`);
    });
  }
}
