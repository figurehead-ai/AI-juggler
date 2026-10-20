/**
 * 0.0.7 dogfood: "any actions that are clickable are not showing hand
 * mouse icon". The shared Button and a handful of rows set
 * `cursor-default`, so a mouse user never saw what was clickable. Every
 * enabled control on every main screen — anything with a click handler,
 * and every button, link, summary, select and checkbox row — must show
 * the pointing hand.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses } from "./fixtures";

const T0 = 1_728_800_000_000;

async function seed(page: Page) {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

/** Enabled, visible clickable things whose cursor isn't the hand. */
async function notPointing(page: Page): Promise<{ count: number; wrong: string[]; disabledHands: string[] }> {
  return page.evaluate(() => {
    const disabledHands = [...document.querySelectorAll<HTMLElement>("button:disabled, [aria-disabled='true'], select:disabled")]
      .filter((el) => el.getBoundingClientRect().width > 0 && getComputedStyle(el).cursor === "pointer")
      .map((el) => `${el.tagName.toLowerCase()} "${(el.getAttribute("aria-label") ?? el.innerText ?? "").trim().slice(0, 40)}"`);
    const native =
      'button, [role="button"], a[href], summary, select, label:has(> input[type="checkbox"]), label:has(> input[type="radio"])';
    const wrong: string[] = [];
    let count = 0;
    for (const el of document.querySelectorAll<HTMLElement>("body *")) {
      const props = Object.keys(el).find((key) => key.startsWith("__reactProps"));
      const handler = props ? (el as unknown as Record<string, { onClick?: unknown }>)[props]?.onClick : undefined;
      if (!handler && !el.matches(native)) continue;
      if ((el as HTMLButtonElement).disabled || el.getAttribute("aria-disabled") === "true") continue;
      if (el.closest("[inert], [aria-hidden='true']")) continue;
      const box = el.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) continue;
      // A click handler that only stops propagation on a container (a
      // dialog frame, a menu panel) isn't a control.
      if (!el.matches(native) && el.querySelector(native) && !el.getAttribute("tabindex")) continue;
      count += 1;
      if (getComputedStyle(el).cursor !== "pointer") {
        wrong.push(`${el.tagName.toLowerCase()} "${(el.getAttribute("aria-label") ?? el.innerText ?? "").trim().slice(0, 40)}"`);
      }
    }
    return { count, wrong, disabledHands };
  });
}

async function expectHands(page: Page, where: string) {
  const { count, wrong, disabledHands } = await notPointing(page);
  expect(count, `${where}: some controls were checked`).toBeGreaterThan(3);
  expect(wrong, `${where}: clickable without the hand cursor`).toEqual([]);
  expect(disabledHands, `${where}: a disabled control shows the hand`).toEqual([]);
}

test("every clickable control shows the hand — the main screens", async ({ page }) => {
  await seed(page);
  const nav = page.getByRole("navigation", { name: "Main" });
  await expect(nav).toBeVisible();
  await expectHands(page, "Home");
  for (const name of ["Accounts", "History", "Settings"]) {
    await nav.getByRole("button", { name, exact: true }).click();
    await page.waitForTimeout(150);
    await expectHands(page, name);
  }
  for (const name of ["Triage", "Overview", "Usage", "Fleet", "Cleanup"]) {
    await nav.getByRole("button", { name: "More", exact: true }).click();
    await page.locator("#nav-more").getByRole("button", { name: new RegExp(`^${name}\\b`) }).click();
    await page.waitForTimeout(150);
    await expectHands(page, name);
  }
});

test("every clickable control shows the hand — menus and the legend", async ({ page }) => {
  await seed(page);
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "More", exact: true }).click();
  await expectHands(page, "More menu");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Indicator legend" }).click();
  await expectHands(page, "legend open");
});
