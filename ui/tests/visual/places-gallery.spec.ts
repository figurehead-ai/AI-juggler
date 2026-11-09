/**
 * AJ-E2104, AJ-E2105, AJ-E2107 gallery: the rail by place at 1, 8 and 40
 * open sessions (Needs you, Open now's places, the Also line), the View
 * open, and Home empty, typical and busy (heading, Your apps, Needs you,
 * Runway by account). Both themes, axe 0 critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, gate, scan, test } from "./axe-gallery";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/94-places");
const T0 = new Date(2026, 9, 24, 15, 0, 0).getTime();

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

/** One of AI Juggler's own sessions. */
function own(id: string, over: Record<string, unknown> = {}) {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: "opus", launch_model: "opus",
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "feat/read-model", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: T0 - 600_000,
    last_activity_ms: T0 - 60_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

const SETS = {
  one: { sessions: [], conversations: placesConversations(T0).slice(0, 1) },
  eight: { sessions: [own("sess-1")], conversations: placesConversations(T0) },
  forty: {
    sessions: [own("sess-1"), own("sess-2", { status: "waiting_user", needs_attention: "waiting_user", git_branch: "fix/limits" }), own("sess-3", { status: "quota_blocked", needs_attention: "quota_blocked", git_branch: "chore/deps" })],
    conversations: placesConversations(T0, 30),
  },
  empty: { sessions: [], conversations: [] },
  thousand: { sessions: [], conversations: placesConversations(T0, 1000) },
} as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** 0.4.1: the cards' line-up with room measured (one Account low) and the
 * remembered apps, the closed ones among them. */
const LINE_UP = {
  known: knownPlacesPreview(T0),
  room: [
    { account: "claude-work", five_hour: { percentage: 30, state: "measured", resets_at_ms: null }, weekly: null },
    { account: "claude-personal", five_hour: { percentage: 82, state: "measured", resets_at_ms: null }, weekly: null },
  ],
};

async function seed(page: Page, set: keyof typeof SETS, theme: "dark" | "light", extra: typeof LINE_UP | null = null) {
  await page.addInitScript(
    ({ fixture, theme, t0, extra }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      if (extra) {
        w.__AIJ_PLACES__ = extra.known;
        w.__AIJ_QUOTA_DISPLAY__ = extra.room;
      }
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, projects: PROJECTS, ...SETS[set] }, theme, t0: T0, extra },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string, target: "page" | "rail" = "page") {
  await page.mouse.move(0, 0);
  const path = resolve(OUT, `${name}-${theme}.png`);
  if (target === "rail") await page.getByTestId("rail").screenshot({ path });
  else await page.screenshot({ path });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the rail with 1 open session — ${theme}`, gate("94-places"), async ({ page }) => {
    await seed(page, "one", theme);
    await expect(page.getByTestId("rail-open-now")).toBeVisible();
    await shoot(page, "rail-1", theme, "rail");
  });

  test(`the rail with 8 open sessions: Needs you, places, Also — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    await expect(page.getByTestId("rail-needs-you")).toContainText("3");
    // D-62: grouped by App: Claude Code · Work Max (its terminal sessions in
    // one group), ChatGPT, Claude app · Personal, Claude app · Work Max.
    await expect(page.getByTestId("place-header")).toHaveCount(4);
    await expect(page.getByTestId("rail-also")).toContainText("History");
    // The maintainer's review of 0.1.4: each Needs you row shows its ↗,
    // named for its app and profile (another app runs all three here).
    const needRows = page.getByTestId("place-board").locator("[data-rail-row]").filter({ has: page.getByTestId("need-word") });
    await expect(needRows.getByRole("button", { name: /^Go to it in / })).toHaveCount(3);
    await shoot(page, "rail-8", theme, "rail");
  });

  test(`a row opens its page here; its ↗ goes to its app, saying so — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    // The session that asks: its row under Needs you (it shows again in
    // its app's group further down).
    const asks = page.locator("[data-rail-row]", { has: page.locator('[data-conversation-id="outside-3"]') }).first();
    await asks.locator("[data-rail-item]").click({ position: { x: 60, y: 10 } });
    await expect(page.getByTestId("conversation-page")).toBeVisible();
    await expect(page.getByRole("status").filter({ hasText: /forward|Going to it/ })).toHaveCount(0);
    await asks.getByRole("button", { name: "Go to it in Claude app · Work Max" }).click();
    await expect(page.getByText(/Going to it|forward|Couldn't go to it/).first()).toBeVisible();
    // ⌘↩ on the row does the same; ↩ opens its page.
    await asks.locator("[data-rail-item]").focus();
    await page.keyboard.press("Meta+Enter");
    await expect(page.getByText(/Going to it|forward|Couldn't go to it/).first()).toBeVisible();
  });

  test(`the rail with 40 open sessions, drawn as it scrolls — ${theme}`, async ({ page }) => {
    await seed(page, "forty", theme);
    await expect(page.getByTestId("rail-needs-you")).toBeVisible();
    await shoot(page, "rail-40", theme, "rail");
  });

  test(`the rail's View — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    await page.keyboard.press("Shift+V");
    // 0.3: the View is a menu (Show, Only), its first line holding the keyboard.
    await expect(page.getByRole("menu", { name: "View" })).toBeVisible();
    await shoot(page, "view", theme);
  });

  // 0.4.1: the View's flake (4–5 runs in 30): ⇧V pressed as the rail was
  // first drawn reached no listener, App's keys and the View's being
  // passive effects that ran after paint. Now they listen before paint;
  // here the key comes in the very task that draws the View's button.
  test(`⇧V works from the first frame the rail is drawn — ${theme}`, async ({ page }) => {
    await page.addInitScript(() => {
      const watch = new MutationObserver(() => {
        if (!document.querySelector('[data-testid="rail-view-button"]')) return;
        watch.disconnect();
        document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "V", code: "KeyV", shiftKey: true, bubbles: true, cancelable: true }));
      });
      watch.observe(document, { childList: true, subtree: true });
    });
    await seed(page, "eight", theme);
    await expect(page.getByRole("menu", { name: "View" })).toBeVisible();
  });

  test(`Home, nothing open — ${theme}`, async ({ page }) => {
    await seed(page, "empty", theme);
    await expect(page.getByTestId("home-heading")).toHaveText("Ready when you are");
    await shoot(page, "home-empty", theme);
  });

  test(`Home, typical: what needs you, your apps, runway by account — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    // Counted as the rail counts them (the review of 0.1.4).
    await expect(page.getByTestId("home-heading")).toHaveText("3 sessions need you");
    await expect(page.getByTestId("home-app-tile")).toHaveCount(4);
    await shoot(page, "home-typical", theme);
  });

  test(`Home, the app cards line up: room measured, one low, closed apps after — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme, LINE_UP);
    await expect(page.getByTestId("home-app-closed").first()).toBeVisible();
    await expect(page.getByTestId("home-app-account").first()).toBeVisible();
    await page.getByTestId("home-apps").scrollIntoViewIfNeeded();
    await shoot(page, "home-cards", theme);
  });

  test(`Home, busy — ${theme}`, async ({ page }) => {
    await seed(page, "forty", theme);
    await expect(page.getByTestId("home-apps-more").or(page.getByTestId("home-app-tile").first())).toBeVisible();
    await shoot(page, "home-busy", theme);
  });
}

/**
 * 0.4.1 — what's under the pointer holds still (3efc37b61: a Needs you
 * notice drawn above the place headers after the rail's first paint moved
 * a header ~94 px down between a press on its Open door and the release,
 * so no click reached the door). A line of that height goes in above.
 */
test.describe("a line drawn above the pointer", () => {
  /** A 94 px line, as the notice and its section head, put first in the list. */
  const insert = (when: "now" | "pointerdown") =>
    `(() => {
      const put = () => {
        const line = document.createElement("div");
        line.setAttribute("data-line-key", "notice:arrived");
        line.style.height = "94px";
        line.style.flexShrink = "0";
        const board = document.querySelector('[data-testid="place-board"]');
        board.insertBefore(line, board.firstChild);
      };
      if (${JSON.stringify(when)} === "now") put();
      else window.addEventListener("pointerdown", put, { once: true, capture: true });
    })()`;

  test("leaves the header under the pointer where it was, and says it came", async ({ page }) => {
    await seed(page, "eight", "dark");
    const header = page.getByTestId("place-header").nth(1);
    const search = page.getByRole("searchbox", { name: /^Search sessions/ });
    await header.hover();
    const before = await header.boundingBox();
    const searchAt = await search.boundingBox();
    await page.evaluate(insert("now"));
    // The same frame: no layout between the insert and this read.
    await expect.poll(async () => (await header.boundingBox())?.y).toBe(before?.y);
    // The rail's head stays where it is: only the list scrolled.
    expect(await search.boundingBox()).toEqual(searchAt);
    await expect(page.getByTestId("rail-new-above")).toHaveText("1 new above");
    // Its pill scrolls up to it.
    await page.getByTestId("rail-new-above").click();
    await expect(page.getByTestId("rail-new-above")).toHaveCount(0);
    await expect(page.locator('[data-line-key="notice:arrived"]')).toBeInViewport();
  });

  test("a line drawn as the door is pressed: the click still lands on the door", async ({ page }) => {
    await seed(page, "eight", "dark");
    const header = page.getByTestId("place-header").nth(1);
    await header.hover();
    const door = header.getByRole("button", { name: /^Open / });
    await expect(door).toBeVisible();
    const before = await header.boundingBox();
    await door.evaluate((element) => {
      (window as unknown as { doorClicks: number }).doorClicks = 0;
      element.addEventListener("click", (event) => {
        (window as unknown as { doorClicks: number }).doorClicks += 1;
        event.stopPropagation();
        event.preventDefault();
      }, { capture: true });
    });
    await page.evaluate(insert("pointerdown"));
    await door.click();
    expect(await page.evaluate(() => (window as unknown as { doorClicks: number }).doorClicks)).toBe(1);
    expect((await header.boundingBox())?.y).toBe(before?.y);
  });
});

/**
 * 0.4.1 — the rail's list is its one scroller. It was not: the list grew
 * with its content inside a box that scrolled the rail's head too, so
 * the list's windowing measured a box that never moved and drew every
 * line, and ↑/↓ past the drawn lines scrolled nothing.
 */
test("a rail of 1 000 sessions draws only what's in view, and ↑/↓ scroll the list itself", async ({ page }) => {
  await seed(page, "thousand", "dark");
  const list = page.getByTestId("rail-list");
  const search = page.getByRole("searchbox", { name: /^Search sessions/ });
  // One App's 250 sessions, all of them shown.
  await page.getByTestId("rail-more-quiet").first().click();
  const lines = page.getByTestId("place-board").locator("[data-rail-line]");
  const box = () => list.evaluate((el) => ({ top: el.scrollTop, height: el.clientHeight, content: el.scrollHeight }));
  await expect.poll(async () => (await box()).content).toBeGreaterThan(5_000);
  const first = await box();
  expect(first.height).toBeLessThan(800);
  // Only the lines in view and a little either side are drawn.
  expect(await lines.count()).toBeLessThan(60);
  // Scrolled to the middle: what's drawn follows, still a window.
  await list.evaluate((el) => (el.scrollTop = el.scrollHeight / 2));
  await expect.poll(() => lines.count()).toBeLessThan(60);
  // …and the list's view is full of drawn rows, not a spacer's blank.
  const inView = () =>
    list.evaluate((el) => {
      const view = el.getBoundingClientRect();
      return [...el.querySelectorAll("[data-rail-item]")].filter((row) => {
        const at = row.getBoundingClientRect();
        return at.bottom > view.top && at.top < view.bottom;
      }).length;
    });
  await expect.poll(inView).toBeGreaterThan(8);
  // ↓ from the last row in view moves on, scrolling the list (not the rail).
  const searchAt = await search.boundingBox();
  const was = (await box()).top;
  const rows = page.getByTestId("place-board").locator("[data-rail-item]");
  await rows.filter({ visible: true }).last().focus();
  for (let n = 0; n < 15; n += 1) await page.keyboard.press("ArrowDown");
  await expect.poll(async () => (await box()).top).toBeGreaterThan(was);
  await expect(page.locator(":focus")).toBeInViewport();
  expect(await search.boundingBox()).toEqual(searchAt);
});
