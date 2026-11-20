/**
 * 0.5 (D-117, AJ-E3601 R6) — the attention budget, as a gate. Each need is
 * loud once per area; colour, fills and motion are for what needs you.
 * On three fixtures (allStatuses; a typical day: 8 open on 4 Apps, 2 ask
 * and 1 to review; a flood: 7 that wait and 1 that broke), both themes,
 * per area (the header, the rail, Home), from computed styles:
 *
 *   (i)   text painted warning or danger ≤ the area's distinct L2/L3
 *         items, plus its Needs you count;
 *   (ii)  no amber fill (a 6 px level mark is a mark, not a fill);
 *   (iii) at most one danger-soft block per area;
 *   (iv)  at most one filled accent button per view (the top bar's New
 *         session is tinted by design, `variant="accent"`, never filled,
 *         so it is outside this count; on Home the one fill is the first
 *         pinned need's primary, or the empty feed's New session only
 *         once the needs are read and none waits, D-118);
 *   (v)   no animation that runs for ever (0.4.5's rule, idle.spec.ts);
 *   (vi)  every row's data-level is attentionLevel() of its need and state;
 *   (vii) every collapsible in the rail keeps its chevron, visible at rest
 *         and when focused (D-117 point 3);
 *   (viii) selection is its own category, never loud: whatever is
 *         selected, chosen or current (aria-checked, -current, -selected,
 *         -pressed) wears the one selected look — a neutral edge 3:1 off
 *         what is around it on a raised fill, its words semibold
 *         (`--color-selected*`, controls/selected.ts) — never an amber,
 *         red or accent fill, so it never counts in (i)–(iv) and never
 *         spends the budget a need has. The top bar's current page is
 *         the one exception, as D-118's mock draws it: semibold words
 *         and no box (`NAV_CURRENT`), still never a fill.
 *
 * Each screen's calm pass (AJ-E3604…) adds its fixtures and areas here.
 *
 * AJ-E3603 (R3–R5, the rail and the header, calm): a fourth fixture,
 * L3-heavy (four sessions that stopped with an error, one limit, two that
 * ask), and (x) on every fixture, both themes and all three rail
 * densities (Comfortable, Compact, Minimal): the rail's and the header's
 * words are calm. Only Needs you's count (the folded rail's too) is
 * painted amber or red; every need's level is its small mark beside
 * words in calm ink, never a whole loud line, on every page (Settings
 * too); no amber fill, no tinted row, no filled badge; and axe finds no
 * critical or serious problem (colour contrast among them) in the rail
 * or the header.
 *
 * D-118 (AJ-E3701, Home's calm pass): Home's loud copy of a need is its
 * pinned zone ("Needs you now"), whose count carries the level as the
 * rail's does; the feed under it is quiet (ix): an item about a need is a
 * history line, never amber or red, whatever its level was; and Home's
 * folds keep their chevrons too (vii).
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { allStatuses, expectRail, goTo } from "./fixtures";
import { placesConversations } from "../../src/preview/places";
import { attentionLevel } from "../../src/lib/attention";

const T0 = new Date(2026, 9, 24, 15, 0, 0).getTime();
const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }];

function own(id: string, over: Record<string, unknown> = {}) {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: "opus", launch_model: "opus",
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "feat/read-model", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: T0 - 600_000,
    last_activity_ms: T0 - 60_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

const waits = (n: number) => Array.from({ length: n }, (_, i) => own(`wait-${i}`, { status: "waiting_user", needs_attention: "waiting_user", last_activity_ms: T0 - (i + 1) * 120_000 }));

const broke = (n: number) => Array.from({ length: n }, (_, i) => own(`broke-${i}`, { status: "failed", needs_attention: "failed", title: ["Port the settings parser", "Migrate the billing webhooks", "Rename the photo exports", "Fix the flaky login test"][i % 4], last_activity_ms: T0 - (i + 1) * 300_000 }));

const FIXTURES = {
  "all-statuses": allStatuses,
  typical: { accounts: ACCOUNTS, projects: PROJECTS, sessions: [own("sess-1")], conversations: placesConversations(T0) },
  flood: { accounts: ACCOUNTS, projects: PROJECTS, sessions: [...waits(6), own("crash", { status: "failed", needs_attention: "failed" }), own("busy")], conversations: placesConversations(T0, 12) },
  // AJ-E3603 R5: a bad morning: four stopped with an error, one at its
  // limit, two that ask, one still working.
  "l3-heavy": { accounts: ACCOUNTS, projects: PROJECTS, sessions: [...broke(4), own("limit", { status: "quota_blocked", needs_attention: "quota_blocked", title: "Tidy the blog's images" }), ...waits(2), own("busy", { title: "Refactor the read model" })], conversations: placesConversations(T0) },
} as const;

async function open(page: Page, fixture: keyof typeof FIXTURES, theme: "dark" | "light", density: "comfortable" | "compact" | "minimal" = "comfortable") {
  await page.addInitScript(
    ({ set, theme: t, t0, density: d }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = set;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.density", d);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { set: FIXTURES[fixture], theme, t0: T0, density },
  );
  // Motion allowed: the reduced-motion rule would hide what loops.
  await page.emulateMedia({ reducedMotion: "no-preference", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expectRail(page);
  await expect(page.getByTestId("home-heading")).not.toHaveText(/Checking/);
  await expect(page.locator("[data-testid='rail'] [data-level]").first()).toBeVisible();
}

/** (x) The words in an area painted amber or red, but for Needs you's
 * count (the rail's, the folded rail's): each "tag "words"". */
function loudWords(page: Page, selector: string): Promise<string[]> {
  return page.evaluate((sel) => {
    const area = document.querySelector(sel);
    const probe = document.createElement("span");
    document.body.appendChild(probe);
    const colour = (value: string) => {
      probe.style.color = value;
      return getComputedStyle(probe).color;
    };
    const loud = [colour("var(--color-warning)"), colour("var(--color-danger)")];
    probe.remove();
    if (!area) return [];
    const COUNT = "[data-testid='rail-needs-you'] [data-testid='section-count'], [data-testid='rail-folded-needs-you']";
    return [...area.querySelectorAll<HTMLElement>("*")]
      .filter((element) => [...element.childNodes].some((node) => node.nodeType === Node.TEXT_NODE && (node.textContent ?? "").trim() !== ""))
      .filter((element) => element.getBoundingClientRect().width > 0 && loud.includes(getComputedStyle(element).color))
      .filter((element) => !element.closest(COUNT))
      .map((element) => `${element.tagName.toLowerCase()} "${(element.textContent ?? "").trim().slice(0, 40)}"`);
  }, selector);
}

const AREAS = { header: "header", rail: "[data-testid='rail']", home: "[data-testid='home']" } as const;

interface Measured {
  loudText: string[];
  amberFills: string[];
  dangerBlocks: number;
  distinctLoud: number;
  needsCount: number;
}

/** One area's loud elements, read from computed styles against the
 * theme's own tokens. */
function measure(page: Page, selector: string): Promise<Measured> {
  return page.evaluate((sel) => {
    const area = document.querySelector(sel);
    const probe = document.createElement("span");
    document.body.appendChild(probe);
    const colour = (value: string) => {
      probe.style.color = value;
      return getComputedStyle(probe).color;
    };
    const warning = colour("var(--color-warning)");
    const danger = colour("var(--color-danger)");
    const warningSoft = colour("var(--color-warning-soft)");
    const dangerSoft = colour("var(--color-danger-soft)");
    probe.remove();
    const out = { loudText: [] as string[], amberFills: [] as string[], dangerBlocks: 0, distinctLoud: 0, needsCount: 0 };
    if (!area) return out;
    // The facts at L2/L3: rows (a session, a notice, a tile), and a fact of
    // its own inside a calm row (a live session's Account out of room).
    const ROW = "[data-rail-row], [data-testid='rail-notice'], [data-testid='home-app-tile'], [data-testid^='home-need']";
    const MARK = "[data-testid='section-count'], [data-testid='place-needs'], [data-testid='home-app-needs'], [data-testid='rail-folded-needs'], [data-testid='need-word'], .level-mark";
    const facts = new Set<Element>();
    for (const el of area.querySelectorAll<HTMLElement>("[data-level]")) {
      if (Number(el.dataset.level) < 2) continue;
      if (el.matches(ROW)) facts.add(el);
      else if (!el.matches(MARK) && !el.parentElement?.closest(ROW + "[data-level='2'], " + ROW + "[data-level='3']")) facts.add(el);
    }
    out.distinctLoud = facts.size;
    const count = area.querySelector("[data-testid='rail-needs-you'] [data-testid='section-count'], [data-testid='home-needs-now-section'] [data-testid='section-count']");
    out.needsCount = count ? 1 : 0;
    const seen = new Set<Element>();
    const tinted: DOMRect[] = [];
    for (const element of area.querySelectorAll<HTMLElement>("*")) {
      const style = getComputedStyle(element);
      if (style.visibility === "hidden" || style.display === "none") continue;
      const box = element.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) continue;
      const ownText = [...element.childNodes].some((node) => node.nodeType === Node.TEXT_NODE && (node.textContent ?? "").trim() !== "");
      // One loud mark per fact: words painted amber or red, counted once
      // per the level-carrying element they sit in.
      if (ownText && (style.color === warning || style.color === danger)) {
        const holder = element.closest("[data-level]") ?? element;
        if (!seen.has(holder)) {
          seen.add(holder);
          out.loudText.push(`${element.tagName.toLowerCase()} "${(element.textContent ?? "").trim().slice(0, 40)}"`);
        }
      }
      const fill = style.backgroundColor;
      const isMark = box.width <= 8 && box.height <= 8;
      if (!isMark && (fill === warning || fill === warningSoft)) out.amberFills.push(`${element.tagName.toLowerCase()}.${element.className.toString().slice(0, 60)}`);
      if (fill === dangerSoft) tinted.push(box);
    }
    // L3 rows that sit together (≤ 6 px apart) read as one block.
    tinted.sort((a, b) => a.top - b.top);
    tinted.forEach((box, i) => {
      if (i === 0 || box.top - tinted[i - 1]!.bottom > 6) out.dangerBlocks += 1;
    });
    return out;
  }, selector);
}

for (const theme of ["dark", "light"] as const) {
  for (const fixture of Object.keys(FIXTURES) as (keyof typeof FIXTURES)[]) {
    test(`the attention budget holds: ${fixture} — ${theme}`, async ({ page }) => {
      await open(page, fixture, theme);
      for (const [name, selector] of Object.entries(AREAS)) {
        const m = await measure(page, selector);
        // (i) loud once per fact per area (+ the Needs you count).
        expect(m.loudText.length, `${name}: amber/red words ${JSON.stringify(m.loudText)} for ${m.distinctLoud} needs`).toBeLessThanOrEqual(m.distinctLoud + m.needsCount);
        // (ii) no amber fills.
        expect(m.amberFills, `${name}: amber fills`).toEqual([]);
        // (iii) at most one L3 tint block (the L3 rows sit together on top).
        expect(m.dangerBlocks, `${name}: danger-soft blocks`).toBeLessThanOrEqual(fixture === "typical" ? 0 : 1);
      }
      // (iv) one filled accent button per view, at most.
      const filled = await page.evaluate(() => {
        const probe = document.createElement("span");
        document.body.appendChild(probe);
        probe.style.color = "var(--color-accent-fill)";
        const fill = getComputedStyle(probe).color;
        probe.style.color = "var(--color-accent)";
        const accent = getComputedStyle(probe).color;
        probe.remove();
        return [...document.querySelectorAll<HTMLElement>("button")]
          .filter((button) => button.getBoundingClientRect().width > 0)
          .filter((button) => [fill, accent].includes(getComputedStyle(button).backgroundColor))
          .map((button) => (button.textContent ?? "").trim().slice(0, 30));
      });
      expect(filled.length, `filled accent buttons: ${JSON.stringify(filled)}`).toBeLessThanOrEqual(1);
      // (viii) selection, its own category: quiet, and never a loud fill.
      const selection = await page.evaluate((areas) => {
        const probe = document.createElement("span");
        document.body.appendChild(probe);
        const colour = (value: string) => {
          probe.style.color = value;
          return getComputedStyle(probe).color;
        };
        const loud = ["warning", "warning-soft", "danger", "danger-soft", "accent", "accent-fill", "accent-soft"].map((name) => colour(`var(--color-${name})`));
        const selected = colour("var(--color-selected)");
        probe.remove();
        const SEL = "[role='radio'][aria-checked='true'], [aria-current]:not([aria-current='false']), [aria-selected='true'], [aria-pressed='true']";
        const found = areas.flatMap((sel) => [...(document.querySelector(sel)?.querySelectorAll<HTMLElement>(SEL) ?? [])]);
        const page = document.querySelector<HTMLElement>("header [data-testid='main-nav'] [aria-current='page']");
        return {
          drawn: found.filter((element) => element.className.toString().includes("inset-ring-selected-ring") && getComputedStyle(element).backgroundColor === selected).length,
          // The current page: semibold, no fill (transparent).
          page: page ? { weight: Number(getComputedStyle(page).fontWeight), fill: getComputedStyle(page).backgroundColor } : null,
          loud: found
            .filter((element) => element.getBoundingClientRect().width > 0 && loud.includes(getComputedStyle(element).backgroundColor))
            .map((element) => `${element.tagName.toLowerCase()} "${(element.textContent ?? "").trim().slice(0, 30)}"`),
        };
      }, Object.values(AREAS));
      expect(selection.loud, "a selected item drawn with a loud fill").toEqual([]);
      // The header's current page is always marked, by weight, never a box.
      expect(selection.page, "the header's current page").not.toBeNull();
      expect(selection.page!.weight, "the current page's words semibold").toBeGreaterThanOrEqual(600);
      expect(selection.page!.fill, "the current page without a fill").toBe("rgba(0, 0, 0, 0)");
      // Every other chosen thing in sight (the rail's lens, a filter) wears the selected look.
      expect(selection.drawn, "selected items in the selected look").toBeGreaterThan(0);
      // (v) nothing loops for ever in the rail or on Home.
      await expect
        .poll(() =>
          page.evaluate(() =>
            document
              .getAnimations()
              .filter((animation) => animation.playState === "running" && animation.effect?.getComputedTiming().iterations === Infinity)
              .map((animation) => ((animation.effect as KeyframeEffect | null)?.target as Element | null)?.closest("[data-testid='rail'], [data-testid='home']") !== null).filter(Boolean).length,
          ),
        { timeout: 10_000 },
        )
        .toBe(0);
      // (ix) D-118: the feed's stream is quiet: no amber or red words in it.
      const loudInFeed = await page.locator("[data-testid='feed']").evaluateAll((feeds) => {
        const probe = document.createElement("span");
        document.body.appendChild(probe);
        const colour = (value: string) => {
          probe.style.color = value;
          return getComputedStyle(probe).color;
        };
        const loud = [colour("var(--color-warning)"), colour("var(--color-danger)")];
        probe.remove();
        return feeds.flatMap((feed) => [...feed.querySelectorAll<HTMLElement>("*")].filter((element) => loud.includes(getComputedStyle(element).color) && (element.textContent ?? "").trim() !== "").map((element) => (element.textContent ?? "").trim().slice(0, 40)));
      });
      expect(loudInFeed, "loud words in the feed").toEqual([]);
      // (vi) every row's level is attentionLevel()'s.
      const rows = await page.locator("[data-testid='rail'] [data-level][data-state]").evaluateAll((all) => all.map((row) => ({ level: Number((row as HTMLElement).dataset.level), need: (row as HTMLElement).dataset.need ?? null, state: (row as HTMLElement).dataset.state ?? null })));
      expect(rows.length).toBeGreaterThan(0);
      for (const row of rows) expect(row.level, JSON.stringify(row)).toBe(attentionLevel({ need: row.need, state: row.state }));
    });
  }

  // (x) AJ-E3603 R3–R5: the rail and the header in every density, on
  // every fixture: calm words, the level on its mark, axe clean; on Home
  // and on Settings (R5: "on every page, Settings included").
  for (const density of ["comfortable", "compact", "minimal"] as const) {
    for (const fixture of Object.keys(FIXTURES) as (keyof typeof FIXTURES)[]) {
      test(`the rail and the header are calm: ${fixture}, ${density} — ${theme}`, async ({ page }) => {
        await open(page, fixture, theme, density);
        await expect(page.getByTestId("rail")).toHaveAttribute("data-density", density);
        for (const where of ["Home", "Settings"] as const) {
          if (where === "Settings") {
            await goTo(page, "Settings");
            await expect(page.getByRole("navigation", { name: "Settings" })).toBeVisible();
          }
          for (const [name, selector] of [["rail", AREAS.rail], ["header", AREAS.header]] as const) {
            expect(await loudWords(page, selector), `${where}, ${name}: amber or red words other than Needs you's count`).toEqual([]);
            const m = await measure(page, selector);
            expect(m.amberFills, `${where}, ${name}: amber fills`).toEqual([]);
            expect(m.dangerBlocks, `${where}, ${name}: tinted rows`).toBe(0);
          }
          // Every need in the rail carries its level's mark (the words are calm).
          const unmarked = await page.locator("[data-testid='rail'] [data-level='2'][data-rail-row], [data-testid='rail'] [data-level='3'][data-rail-row]").evaluateAll((rows) => rows.filter((row) => !row.querySelector(".level-mark")).map((row) => (row.textContent ?? "").trim().slice(0, 40)));
          expect(unmarked, `${where}: a need's row without its level's mark`).toEqual([]);
          const axe = await new AxeBuilder({ page }).include(AREAS.rail).include(AREAS.header).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"]).analyze();
          const bad = axe.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
          expect(bad.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`), `${where}: axe on the rail and the header`).toEqual([]);
        }
      });
    }
  }

  // (vii) D-117 point 3: every collapsible keeps its chevron (D-118:
  // Home's Needs you now, Your apps and Runway by account too).
  test(`every rail and Home collapsible keeps its chevron, at rest and focused — ${theme}`, async ({ page }) => {
    await open(page, "typical", theme);
    // Disclosures (a menu's button is a popup, not a fold; "+3 recent" is
    // words that show on hover, as before D-117).
    const toggles = page.locator("[data-testid='place-board'] button[aria-expanded]:not([aria-haspopup]):not([data-testid='place-recent']), [data-testid='home'] button[aria-expanded][data-testid$='-fold']");
    const n = await toggles.count();
    expect(n).toBeGreaterThan(2);
    for (let i = 0; i < n; i++) {
      const toggle = toggles.nth(i);
      if (!(await toggle.isVisible())) continue;
      const chevron = toggle.locator("svg").last();
      await expect(chevron, `chevron of ${await toggle.getAttribute("aria-label") ?? await toggle.textContent()}`).toBeVisible();
      const opacity = await chevron.evaluate((svg) => Number(getComputedStyle(svg).opacity));
      expect(opacity).toBeGreaterThan(0.3);
      await toggle.focus();
      await expect(chevron).toBeVisible();
    }
  });
}
