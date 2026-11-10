/**
 * AJ-E3306 (the rc26 walk, X2): on Settings › Advanced, "Keyboard" looked
 * selected together with "Advanced". The pointer rested on Keyboard, and
 * its hover fill (surface-2) was all but the selected item's (accent-soft
 * over the page, same ink text, no other mark), so a hover read as a
 * second selection. Only the selected item may look selected: it carries
 * the accent (text and edge) a hovered item never has, in both themes.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { allStatuses, goTo, settingsPane } from "./fixtures";

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.theme", theme);
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: allStatuses, theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await goTo(page, "Settings");
}

/** What makes an item look chosen: its fill, its text, its edge. */
async function look(item: Locator) {
  return item.evaluate((el) => {
    const style = getComputedStyle(el);
    return { fill: style.backgroundColor, text: style.color, edge: style.boxShadow };
  });
}

for (const theme of ["dark", "light"] as const) {
  test(`Settings' nav: only the selected item looks selected, a hover never does — ${theme}`, async ({ page }) => {
    await open(page, theme);
    const nav = page.getByRole("navigation", { name: "Settings" });
    await settingsPane(page, "Advanced");
    const advanced = nav.getByRole("button", { name: "Advanced", exact: true });
    const keyboard = nav.getByRole("button", { name: "Keyboard", exact: true });
    await expect(advanced).toHaveAttribute("aria-current", "page");

    // The pointer rests on the item above, as in the walk's capture.
    await keyboard.hover();
    await page.waitForTimeout(250); // past the colour transition
    const selected = await look(advanced);
    const hovered = await look(keyboard);
    expect(hovered.text, "a hovered item keeps the plain ink; the accent is the selection's").not.toBe(selected.text);
    expect(selected.edge, "the selected item has its accent edge").not.toBe("none");
    expect(hovered.edge, "a hovered item has no edge").toBe("none");

    // The pointer away: exactly one item has a fill.
    await page.mouse.move(1270, 790);
    await page.waitForTimeout(250);
    const filled = await nav
      .getByRole("button")
      .evaluateAll((items) =>
        items.filter((el) => getComputedStyle(el).backgroundColor !== "rgba(0, 0, 0, 0)").map((el) => el.textContent?.trim()),
      );
    expect(filled).toEqual(["Advanced"]);
  });
}
