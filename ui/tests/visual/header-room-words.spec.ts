/**
 * 9aa04ae56's rule, held on every page: This Mac's room words in the top
 * bar ("Plenty of room") show whole or not at all, never cut mid-word.
 * Settings at 1280 read "Plenty of r…" once the ‹ › arrows (1017ca61c),
 * the clean-up count and the bell's count took their room; below the
 * width they need they give way and the dot stays, the words in its
 * title. At 1280 and 1024, on Home and in Settings.
 */
import { expect, test } from "@playwright/test";
import { allStatuses, goTo } from "./fixtures";

for (const width of [1280, 1024]) {
  for (const screen of ["Home", "Settings"]) {
    test(`room words whole or gone — ${screen} — ${width}`, async ({ page }) => {
      await page.addInitScript((fixture) => {
        (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      }, allStatuses);
      await page.setViewportSize({ width, height: 800 });
      await page.goto("/");
      if (screen !== "Home") await goTo(page, screen);
      await expect(page.getByTestId("machine-status")).toBeAttached();
      const words = page.getByTestId("machine-status-words");
      const cut = await words.evaluate((el) => getComputedStyle(el).display !== "none" && el.scrollWidth > el.clientWidth + 0.5);
      expect(cut, "the room words are cut").toBe(false);
      // Inside their own line, and clear of what follows (Settings at 1280
      // drew "Plenty of room7 to clean up": whole, but run into the count).
      const clear = await words.evaluate((el) => {
        if (getComputedStyle(el).display === "none") return true;
        const own = el.getBoundingClientRect();
        const slot = el.closest("[data-testid=header-this-mac]");
        if (!slot) return false;
        let next = slot.nextElementSibling;
        while (next && next.getBoundingClientRect().width === 0) next = next.nextElementSibling;
        const inSlot = own.right <= slot.getBoundingClientRect().right + 0.5;
        const beforeNext = !next || own.right <= next.getBoundingClientRect().left + 0.5;
        return inSlot && beforeNext;
      });
      expect(clear, "the room words run into the next item").toBe(true);
    });
  }
}
