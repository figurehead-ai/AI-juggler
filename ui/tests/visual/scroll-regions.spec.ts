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
import { GATE_NOW } from "./axe-gallery";
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

/** An account almost out of room, as the model holds it: the session
 * page's own notice ("This session's account, Work Max, is almost out of
 * room") for `account`'s sessions. */
function roomNotice(account: string): Record<string, unknown> {
  const facts = { window: "5-hour", resets_at_ms: GATE_NOW + 95 * 60_000, sessions: 2, version: null, channel: null, attempt: null, tries: null, account: "Work Max" };
  const target = { session_id: null, conversation_id: null, account_id: account, project_id: null, page: null, task_id: null, pool_id: null };
  return { app: [{ id: 41, kind: "quota", needs: true, unread: true, title: "Work Max is almost out of room", detail: "7% of its 5-hour window left", project_id: null, project: null, target, facts, action: "hand-over", meter: 7, created_ms: GATE_NOW - 180_000, snoozed_until_ms: null, muted: false, dismissed: false }], earlier: [] };
}

/** Every box on the page that scrolls now, but a terminal's own. */
async function scrollers(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    Array.from(document.querySelectorAll<HTMLElement>("[data-page-kind], [data-page-kind] *"))
      .filter((el) => /(auto|scroll)/.test(getComputedStyle(el).overflowY) && el.scrollHeight > el.clientHeight + 1 && !el.closest(".xterm"))
      .map((el) => el.dataset.testid ?? el.tagName.toLowerCase()),
  );
}

// R2 (the 0.5.0 re-walk): at 1024×680 with the account's notice up, a
// session page scrolled three boxes inside each other: the page (9 px),
// its tab's section (24 px, a 22rem floor the conversation kept beside the
// column) and the column, 131 px of 704. One region per area: the page
// and its section never scroll; the column (and the conversation's own
// list) do; Switch account… and the column's first card are in view.
test.describe("R2 a session page at 1024×680 with its account's notice", () => {
  for (const [session, account, notice] of [
    ["sess-2", "claude-work", true],
    ["sess-3", "claude-personal", true],
    // An ended one: its conversation is its main pane (the 22rem floor).
    ["sess-8", "claude-work", false],
  ] as const) {
    test(`${session}: one scroll region per area`, async ({ page }) => {
      const size = { width: 1024, height: 680 };
      await open(page, "dark", size, { __AIJ_FIXTURE__: allStatuses, __AIJ_NOTICES__: roomNotice(account) });
      await openRailSession(page, session);
      if (notice) await expect(page.getByTestId("session-notice-quota")).toBeVisible();
      const button = page.getByTestId("switch-account");
      await expect(button).toBeVisible();
      await settle(page);
      const found = await scrollers(page);
      expect(found.filter((name) => name !== "session-side" && name !== "last-messages-scroll"), "only the column and the conversation's own list scroll").toEqual([]);
      const root = page.locator("[data-page-kind]").first();
      const state = await scrollState(root);
      expect(state.height, "the page itself doesn't scroll").toBeLessThanOrEqual(state.client + 1);
      const lead = await boxOf(button);
      expect(lead.y + lead.height, "Switch account… whole").toBeLessThanOrEqual(size.height);
      const side = await boxOf(page.getByTestId("session-side"));
      const first = await boxOf(page.getByTestId("session-side").locator(":scope > *").first());
      expect(first.y, "the first card starts in the column").toBeGreaterThanOrEqual(side.y - 1);
      // With the notice up, its first card whole; an ended one's (its
      // header two rows, its card a line longer) mostly, More below under it.
      if (notice) expect(first.y + first.height, "the first card whole").toBeLessThanOrEqual(side.y + side.height + 1);
      else expect(side.y + side.height - first.y, "most of the first card").toBeGreaterThanOrEqual(200);
      expect(await traps(page)).toEqual([]);
    });
  }
});

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
