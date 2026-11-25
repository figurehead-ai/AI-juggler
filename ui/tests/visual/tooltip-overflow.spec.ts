/**
 * AJ-E1556 — no tooltip or tip is clipped, and nothing overlaps one. The
 * 0.5.1 Nightly: Bot's New topic tip read "Nothing is do…" with a "23"
 * drawn over it. In the gallery's states (Home and the rail in each lens
 * and the folded rail, Bot's panel at 1280 and 1024, a session's page,
 * Accounts and Usage's charts), every control that can show a tip is
 * hovered in turn, and every tip drawn (`[role=tooltip]`, `[data-tip]`)
 * is measured in the browser:
 *
 * - its text is whole: every line of its words inside its box, no element
 *   in it wider than its box where its overflow is hidden or ellipsised,
 *   none line-clamped short;
 * - it is inside the window, and inside every box that clips it;
 * - nothing is drawn over it (the topmost element at points across it is
 *   the tip's own).
 *
 * Measurements, no captures: run by the accessibility gate
 * (scripts/a11y_gate.sh). Writes docs/qa/evidence/AJ-E1556/tooltip-overflow.json.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { BOT_NOW, STATES } from "../../src/preview/bot";
import type { FixtureSet } from "../../src/preview/fixtures";
import { allStatuses, goTo, projectLens, railViewAs } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E1556");
/** Per state: the controls hovered, the tips opened (by test id), and what was wrong. */
const results: Record<string, { hovered: number; tips: Record<string, number>; findings: string[] }> = {};

test.afterAll(() => {
  if (Object.keys(results).length === 0) return;
  mkdirSync(OUT, { recursive: true });
  writeFileSync(resolve(OUT, "tooltip-overflow.json"), JSON.stringify(results, null, 2) + "\n");
});

type Setup = { theme: "dark" | "light"; size: { width: number; height: number }; density?: "minimal"; bot?: boolean };

async function open(page: Page, setup: Setup, fixture: FixtureSet = allStatuses): Promise<void> {
  const bot = STATES.newtopic!;
  await page.addInitScript(
    ({ fixture: f, theme, density, now, bot: b }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = f;
      if (b) {
        w.__AIJ_BOT__ = JSON.parse(JSON.stringify(b.bot));
        w.__AIJ_AI_JOBS__ = b.jobs;
      }
      localStorage.setItem("aij.theme", theme);
      if (density) localStorage.setItem("aij.density", density);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => now;
    },
    { fixture, theme: setup.theme, density: setup.density ?? null, now: BOT_NOW, bot: setup.bot ? bot : null },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: setup.theme });
  await page.setViewportSize(setup.size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** What is wrong with every tip drawn now, in words (in the page). */
function measureTips(page: Page): Promise<Array<{ id: string; problems: string[] }>> {
  return page.evaluate(() => {
    const name = (el: Element) => {
      const id = el.getAttribute("data-testid");
      const text = (el.textContent ?? "").trim().replace(/\s+/g, " ").slice(0, 32);
      return `${el.tagName.toLowerCase()}${id ? `[${id}]` : ""}${text ? ` "${text}"` : ""}`;
    };
    const tips = [...document.querySelectorAll<HTMLElement>("[role='tooltip'], [data-tip]")].filter((tip) => {
      const box = tip.getBoundingClientRect();
      return box.width > 0 && box.height > 0 && getComputedStyle(tip).visibility !== "hidden" && !tip.closest("[hidden]");
    });
    return tips.map((tip) => {
      const problems: string[] = [];
      const box = tip.getBoundingClientRect();
      // Inside the window.
      if (box.left < -0.5 || box.top < -0.5 || box.right > window.innerWidth + 0.5 || box.bottom > window.innerHeight + 0.5) {
        problems.push(`outside the window (${Math.round(box.left)},${Math.round(box.top)} to ${Math.round(box.right)},${Math.round(box.bottom)} in ${window.innerWidth}×${window.innerHeight})`);
      }
      // Inside every box that clips it.
      for (let at = tip.parentElement; at && at !== document.body && at !== document.documentElement; at = at.parentElement) {
        const style = getComputedStyle(at);
        if (style.position === "fixed" && at !== tip) break;
        if (style.overflowX === "visible" && style.overflowY === "visible") continue;
        const clip = at.getBoundingClientRect();
        if (box.left < clip.left - 1 || box.right > clip.right + 1 || box.top < clip.top - 1 || box.bottom > clip.bottom + 1) problems.push(`cut by ${name(at)}`);
      }
      // Its words whole: nothing cut or ellipsised, nothing clamped short.
      for (const el of [tip, ...tip.querySelectorAll<HTMLElement>("*")]) {
        if (!(el.textContent ?? "").trim()) continue;
        const style = getComputedStyle(el);
        const hides = style.overflowX !== "visible" || style.textOverflow === "ellipsis";
        if (hides && el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1) problems.push(`text cut: ${name(el)} (${el.scrollWidth} px in ${el.clientWidth} px${style.textOverflow === "ellipsis" ? ", ellipsised" : ""})`);
        const clamp = style.getPropertyValue("-webkit-line-clamp");
        if (clamp && clamp !== "none" && el.scrollHeight > el.clientHeight + 1) problems.push(`text clamped: ${name(el)}`);
        if (style.overflowY !== "visible" && el.clientHeight > 0 && el.scrollHeight > el.clientHeight + 1) problems.push(`text cut below: ${name(el)}`);
      }
      // Its words inside its box: a line that runs past its edge (kept on
      // one line by a nowrap around it) is cut by whatever clips the page.
      const walker = document.createTreeWalker(tip, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        if (!(node.textContent ?? "").trim()) continue;
        const range = document.createRange();
        range.selectNodeContents(node);
        const words = range.getBoundingClientRect();
        if (words.width === 0) continue;
        if (words.right > box.right + 1 || words.left < box.left - 1 || words.bottom > box.bottom + 1) {
          problems.push(`words past its edge: "${(node.textContent ?? "").trim().slice(0, 32)}" (${Math.round(words.right - box.right)} px over)`);
        }
      }
      // Nothing over it: the topmost element across it is its own. A tip
      // lets the pointer through, so it takes it for the measurement.
      const parts = [tip, ...tip.querySelectorAll<HTMLElement>("*")];
      const saved = parts.map((el) => el.style.pointerEvents);
      for (const el of parts) el.style.pointerEvents = "auto";
      const over = new Set<string>();
      for (const fx of [0.08, 0.3, 0.5, 0.7, 0.92]) {
        for (const fy of [0.15, 0.5, 0.85]) {
          const top = document.elementFromPoint(box.left + box.width * fx, box.top + box.height * fy);
          if (top && !tip.contains(top)) over.add(name(top));
        }
      }
      parts.forEach((el, n) => (el.style.pointerEvents = saved[n]!));
      for (const what of over) problems.push(`covered by ${what}`);
      return { id: tip.getAttribute("data-testid") ?? tip.getAttribute("role") ?? "tip", problems };
    });
  });
}

/**
 * Hovers every control in `scopes` that could show a tip (buttons, the
 * greyed ones, anything that names a tip), one by one, and measures every
 * tip each one draws. `extra` runs more hovers (a chart's surface).
 */
async function sweep(page: Page, state: string, scopes: string[], extra?: (check: () => Promise<void>) => Promise<void>): Promise<void> {
  const result = (results[state] = { hovered: 0, tips: {} as Record<string, number>, findings: [] as string[] });
  const check = async (what: string) => {
    for (const tip of await measureTips(page)) {
      result.tips[tip.id] = (result.tips[tip.id] ?? 0) + 1;
      for (const problem of tip.problems) {
        const line = `${tip.id} (over ${what}): ${problem}`;
        if (!result.findings.includes(line)) result.findings.push(line);
      }
    }
  };
  // Every control's middle, read once (a hover that shows a row's tools
  // moves nothing: AJ-E1554's slots hold still).
  const controls = await page.evaluate((selectors) => {
    const seen = new Set<Element>();
    const out: Array<{ x: number; y: number; label: string }> = [];
    for (const scope of selectors) {
      for (const root of document.querySelectorAll(scope)) {
        for (const el of root.querySelectorAll<HTMLElement>("button, [role='button'], [aria-disabled='true'], [aria-describedby], [data-tip-trigger]")) {
          if (seen.has(el)) continue;
          seen.add(el);
          const box = el.getBoundingClientRect();
          if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > window.innerHeight || box.right < 0 || box.left > window.innerWidth) continue;
          out.push({ x: box.left + box.width / 2, y: box.top + box.height / 2, label: el.getAttribute("data-testid") ?? el.getAttribute("aria-label") ?? (el.textContent ?? "").trim().slice(0, 24) });
        }
      }
    }
    return out;
  }, scopes);
  for (const control of controls) {
    await page.mouse.move(control.x, control.y);
    result.hovered += 1;
    await check(control.label);
  }
  if (extra) await extra(() => check("a chart"));
  await page.mouse.move(0, 0);
}

const findingsOf = (state: string) => results[state]?.findings ?? [];

// A sweep hovers a few hundred controls, measuring after each.
test.describe.configure({ timeout: 240_000 });

test("Home and the rail (App lens), 1280×800 dark: every tip whole, nothing over it", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 } });
  await sweep(page, "home-app-lens-1280-dark", ["[data-testid='rail']", "main", "header"]);
  expect(findingsOf("home-app-lens-1280-dark")).toEqual([]);
});

test("Home and the rail (App lens), 1024×680 light", async ({ page }) => {
  await open(page, { theme: "light", size: { width: 1024, height: 680 } });
  await sweep(page, "home-app-lens-1024-light", ["[data-testid='rail']", "main", "header"]);
  expect(findingsOf("home-app-lens-1024-light")).toEqual([]);
});

test("the rail's Project lens", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 } });
  await projectLens(page);
  await sweep(page, "project-lens-1280-dark", ["[data-testid='rail']"]);
  expect(findingsOf("project-lens-1280-dark")).toEqual([]);
});

test("the folded rail: each icon's card and the head's names", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 }, density: "minimal" });
  await sweep(page, "folded-rail-1280-dark", ["[data-testid='rail']"]);
  const tips = results["folded-rail-1280-dark"]!.tips;
  expect(tips["rail-folded-tip"] ?? 0, "the icons' cards were opened").toBeGreaterThan(0);
  expect(findingsOf("folded-rail-1280-dark")).toEqual([]);
});

for (const width of [1280, 1024]) {
  test(`Bot's panel at ${width}: New topic's tip and the head's`, async ({ page }) => {
    await open(page, { theme: width === 1280 ? "dark" : "light", size: { width, height: 860 }, bot: true });
    await page.getByTestId("header-bot").click();
    await expect(page.getByTestId("bot-panel")).toBeVisible();
    const state = `bot-panel-${width}`;
    await sweep(page, state, ["[data-testid='bot-panel']"]);
    expect(results[state]!.tips["bot-topic-tip"] ?? 0, "New topic's tip was opened").toBeGreaterThan(0);
    expect(findingsOf(state)).toEqual([]);
  });
}

test("a session's page", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 } });
  await page.getByTestId("rail").locator("button[data-instance-id]").first().click();
  await expect(page.getByTestId("page-type")).toHaveText("Session");
  await sweep(page, "session-page-1280-dark", ["main"]);
  expect(findingsOf("session-page-1280-dark")).toEqual([]);
});

test("an ended session's page (its greyed controls say why)", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 } });
  await railViewAs(page, { group: "place", show: { ended: true, quiet: true } });
  const ended = page.getByTestId("rail").locator("[data-rail-row][data-state='ended'] button[data-instance-id]").first();
  await ended.click();
  await expect(page.getByTestId("page-type")).toHaveText("Session");
  await sweep(page, "ended-session-page-1280-dark", ["main"]);
  expect(findingsOf("ended-session-page-1280-dark")).toEqual([]);
});

test("Accounts", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 } });
  await goTo(page, "Accounts");
  await sweep(page, "accounts-1280-dark", ["main"]);
  expect(findingsOf("accounts-1280-dark")).toEqual([]);
});

test("Usage: every chart's tip across its width", async ({ page }) => {
  await open(page, { theme: "dark", size: { width: 1280, height: 800 } });
  await goTo(page, "Usage");
  await expect(page.locator("main svg").first()).toBeVisible();
  await sweep(page, "usage-1280-dark", ["main"], async (check) => {
    const charts = await page.locator("main svg").all();
    for (const chart of charts) {
      await chart.scrollIntoViewIfNeeded().catch(() => undefined);
      const box = await chart.boundingBox();
      if (!box || box.width < 120 || box.height < 40) continue;
      for (const fx of [0.03, 0.25, 0.5, 0.75, 0.97]) {
        for (const fy of [0.2, 0.6]) {
          await page.mouse.move(box.x + box.width * fx, box.y + box.height * fy);
          await check();
        }
      }
    }
  });
  expect(results["usage-1280-dark"]!.tips["chart-tooltip"] ?? 0, "the charts' tips were opened").toBeGreaterThan(0);
  expect(findingsOf("usage-1280-dark")).toEqual([]);
});
