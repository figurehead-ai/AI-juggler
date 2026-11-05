/**
 * AJ-E2905 gallery: Settings › Keyboard (every shortcut by area; one
 * changed, with Reset and the Reset all bar; capturing; a refused key with
 * its reason; the go-to key's own control) and Settings' search (results
 * with the other panes dimmed; nothing found). Both themes, 1280×800 and
 * 1024×700; axe 0 critical/serious on each.
 */
import { gate, test, expect } from "./axe-gallery";
import { goTo, settingsPane } from "./fixtures";
import { at, family, open, shot, SIZES, THEMES } from "./gallery-04";

const KEYS = family("99-settings-keyboard");
const SEARCH = family("99-settings-search");
const CHANGED = { "aij.walk-timing": "0" };

for (const theme of THEMES) {
  for (const size of SIZES) {
    const where = `${at(size)}-${theme}`;

    test(`Keyboard — ${where}`, at(size) === "1280x800" ? gate("99-settings-keyboard") : {}, async ({ page }) => {
      await open(page, theme, size, { __AIJ_APP_SETTINGS__: { "ui.shortcuts": JSON.stringify({ "new-project": { primary: true, shift: false, alt: true, code: "KeyJ" } }) } }, CHANGED);
      await goTo(page, "Settings");
      await settingsPane(page, "Keyboard");
      await expect(page.getByTestId("keyboard-changed")).toContainText("You changed 1 shortcut.");
      await shot(page, KEYS, `changed-${where}`);
      // Capturing, then a refused key with its reason.
      await page.getByTestId("keyboard-change-palette").click();
      await expect(page.getByTestId("keyboard-capture-palette")).toBeFocused();
      await shot(page, KEYS, `capturing-${where}`);
      await page.keyboard.press("Meta+N");
      await expect(page.getByTestId("keyboard-refused")).toContainText("Change that one first.");
      await shot(page, KEYS, `refused-${where}`);
      await page.keyboard.press("Escape");
      // The fixed ones and the go-to key, further down.
      await page.locator("[data-shortcut='go-to']").scrollIntoViewIfNeeded();
      await shot(page, KEYS, `go-to-${where}`);
    });

    test(`Settings search — ${where}`, at(size) === "1280x800" ? gate("99-settings-search") : {}, async ({ page }) => {
      await open(page, theme, size);
      await goTo(page, "Settings");
      await page.getByTestId("settings-search-field").fill("mute");
      await expect(page.getByTestId("settings-search-hit").first()).toContainText("Muted");
      await shot(page, SEARCH, `results-${where}`);
      await page.getByTestId("settings-search-field").fill("nothing like this");
      await expect(page.getByTestId("settings-search-none")).toBeVisible();
      await shot(page, SEARCH, `none-${where}`);
      await page.getByTestId("settings-search-field").fill("quiet");
      await page.keyboard.press("Enter");
      await expect(page.locator("#settings-notifications-title")).toBeFocused();
      await shot(page, SEARCH, `opened-${where}`);
    });
  }
}
