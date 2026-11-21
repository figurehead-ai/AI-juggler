/**
 * VALIDATION N22 (D-142, D-143): what scrolls and what stays put. The
 * maintainer, 2026-11-21: "make sure things are properly scrolling in
 * areas, and which content scrolls and which stays static".
 *
 * At 1024×680 and 1280×800, dark and light, with the mouse wheel and the
 * keyboard:
 *  1. A dialog's body scrolls under the wheel and Page Down; its header and
 *     footer don't move (N1 measures their boxes; here the input reaches it).
 *  2. A session page: Switch account… stays in its slot whatever scrolls;
 *     the side column's cards scroll as one region under the wheel, and the
 *     page around them doesn't; "More below" shows only while content is
 *     hidden; the wheel over the terminal never scrolls the column.
 *  3. The rail: its lists scroll inside it; New session stays at its foot.
 *  4. No scroll box outside a dialog keeps the wheel from an outer one that
 *     can still scroll (overscroll-behavior other than auto inside a
 *     scrolling ancestor): the earlier pool panel trapped it.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { open } from "./gallery-04";
import { allStatuses, openRailSession, openSwitch, switchRows } from "./fixtures";

const SIZES = [
  { width: 1024, height: 680 },
  { width: 1280, height: 800 },
];

/** Two frames: what a scroll moved has been drawn. */
async function settle(page: Page): Promise<void> {
  await page.evaluate(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));
}

async function boxOf(locator: Locator): Promise<{ x: number; y: number; width: number; height: number }> {
  const box = await locator.boundingBox();
  expect(box, "the element is drawn").not.toBeNull();
  return box!;
}

function same(a: { y: number; height: number }, b: { y: number; height: number }): boolean {
  return Math.abs(a.y - b.y) <= 1 && Math.abs(a.height - b.height) <= 1;
}

async function scrollState(locator: Locator): Promise<{ top: number; height: number; client: number }> {
  return locator.evaluate((el) => ({ top: el.scrollTop, height: el.scrollHeight, client: el.clientHeight }));
}

/** Every scroll box outside a dialog that keeps the wheel from a scrolling
 * ancestor (rule 4), named by its test id or tag. */
async function traps(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const scrolls = (el: Element) => /(auto|scroll)/.test(getComputedStyle(el).overflowY) && el.scrollHeight > el.clientHeight + 1;
    const found: string[] = [];
    for (const el of Array.from(document.querySelectorAll<HTMLElement>("body *"))) {
      if (!scrolls(el) || el.closest("[role='dialog'], [role='alertdialog'], .xterm")) continue;
      if (getComputedStyle(el).overscrollBehaviorY === "auto") continue;
      let up = el.parentElement;
      while (up && up !== document.body) {
        if (scrolls(up)) {
          found.push(el.dataset.testid ?? el.tagName.toLowerCase());
          break;
        }
        up = up.parentElement;
      }
    }
    return found;
  });
}

for (const theme of ["dark", "light"] as const) {
  for (const size of SIZES) {
    const at = `${size.width}x${size.height} — ${theme}`;

    test(`a session page: Switch account… stays, the side column scrolls as one, the terminal on its own — ${at}`, async ({ page }) => {
      await open(page, theme, size, { __AIJ_FIXTURE__: allStatuses });
      await openRailSession(page, "sess-2");
      const button = page.getByTestId("switch-account");
      const side = page.getByTestId("session-side");
      await expect(button).toBeVisible();
      await settle(page);
      const first = await boxOf(button);
      expect(first.y + first.height, "Switch account… whole on open").toBeLessThanOrEqual(size.height);
      const start = await scrollState(side);
      const hidden = start.height > start.client + 1;
      const more = page.getByTestId("session-side-more");
      if (hidden) await expect(more).toBeVisible();
      else await expect(more).toHaveCount(0);
      // The wheel over the column's last visible card scrolls the column.
      const column = await boxOf(side);
      await page.mouse.move(column.x + column.width / 2, column.y + column.height - 24);
      await page.mouse.wheel(0, 600);
      await settle(page);
      const after = await scrollState(side);
      if (hidden) expect(after.top, "the wheel scrolls the side column").toBeGreaterThan(start.top);
      expect(same(first, await boxOf(button)), "Switch account… didn't move").toBe(true);
      expect(await page.evaluate(() => document.scrollingElement?.scrollTop ?? 0), "the window itself never scrolls").toBe(0);
      // At its end "More below" goes.
      await side.evaluate((el) => el.scrollTo({ top: el.scrollHeight }));
      await settle(page);
      await expect(more).toHaveCount(0);
      // The wheel over the terminal scrolls the terminal, never the column.
      const terminal = page.getByTestId("instance-terminal");
      if ((await terminal.count()) > 0 && (await terminal.isVisible())) {
        const held = (await scrollState(side)).top;
        const box = await boxOf(terminal);
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.wheel(0, -400);
        await settle(page);
        expect((await scrollState(side)).top, "the column stays while the terminal scrolls").toBe(held);
      }
      expect(await traps(page)).toEqual([]);
    });

    test(`the rail: its list scrolls inside it, New session stays at its foot — ${at}`, async ({ page }) => {
      await open(page, theme, size, { __AIJ_FIXTURE__: allStatuses });
      const list = page.getByTestId("rail-list");
      const foot = page.getByTestId("rail-new-session");
      if ((await list.count()) === 0 || !(await list.isVisible())) test.skip(true, "the rail is folded at this size");
      await expect(foot).toBeVisible();
      await settle(page);
      const before = await boxOf(foot);
      const start = await scrollState(list);
      const box = await boxOf(list);
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.wheel(0, 600);
      await settle(page);
      if (start.height > start.client + 1) expect((await scrollState(list)).top).toBeGreaterThan(start.top);
      expect(same(before, await boxOf(foot)), "New session didn't move").toBe(true);
      expect(await traps(page)).toEqual([]);
    });

    test(`a dialog: its body scrolls under the wheel and Page Down, its footer stays — ${at}`, async ({ page }) => {
      await open(page, theme, size, { __AIJ_FIXTURE__: allStatuses });
      await openRailSession(page, "sess-2");
      await openSwitch(page);
      await switchRows(page).first().click();
      const dialog = page.getByTestId("switch-flow");
      await expect(dialog).toBeVisible();
      await settle(page);
      const body = dialog.locator("[data-dialog-part='body']").first();
      const footer = dialog.locator("[data-dialog-part='footer']").first();
      const start = await scrollState(body);
      if (start.height <= start.client + 1) test.skip(true, "nothing to scroll at this size");
      const foot = await boxOf(footer);
      const box = await boxOf(body);
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.wheel(0, 300);
      await settle(page);
      const wheeled = await scrollState(body);
      expect(wheeled.top, "the wheel scrolls the body").toBeGreaterThan(start.top);
      expect(same(foot, await boxOf(footer)), "the footer stayed").toBe(true);
      await body.evaluate((el) => el.scrollTo({ top: 0 }));
      // Keyboard focus inside the body, as a Tab into it puts it (the body
      // itself when it holds no control: a scroll box takes the focus then).
      const control = body.locator("input, button, a[href], [tabindex]:not([tabindex='-1'])").first();
      if ((await control.count()) > 0) await control.focus();
      else await body.focus();
      await page.keyboard.press("PageDown");
      await settle(page);
      expect((await scrollState(body)).top, "Page Down scrolls the body").toBeGreaterThan(0);
      expect(same(foot, await boxOf(footer)), "the footer stayed").toBe(true);
    });

    test(`Home: no scroll box traps the wheel — ${at}`, async ({ page }) => {
      await open(page, theme, size, { __AIJ_FIXTURE__: allStatuses });
      await settle(page);
      expect(await traps(page)).toEqual([]);
    });
  }
}
