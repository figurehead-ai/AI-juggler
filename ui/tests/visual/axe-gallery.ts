/**
 * AJ-E804 Req.1 — axe-core on every gallery state. The gallery spec
 * imports `test` from here instead of `@playwright/test`; with
 * `AJ_AXE_GALLERY=1` every `page.screenshot({ path })` first scans the
 * page exactly as captured (WCAG 2.0/2.1/2.2 A and AA rules) and fails
 * the test on any critical or serious violation. One JSON per test
 * lands in `docs/qa/evidence/AJ-E804/axe/`; `scripts/axe_summary.py`
 * turns them into the summary table. Without the variable this is
 * plain Playwright — the gallery runs exactly as before.
 *
 * AJ-E1522 — the gallery in the pixel gate. Each family tags one state
 * with `gate("<family>")`; the visual gate's `gallery` project runs only
 * those, and there a `screenshot({ path })` of the page or of one element
 * is compared with the approved baseline `gallery/<family>/<file>` instead
 * of being written. A gate state that compares nothing fails.
 */
import { test as base, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { relative, resolve } from "node:path";

const ON = process.env.AJ_AXE_GALLERY === "1";
const GALLERY = resolve(process.cwd(), "../docs/qa/evidence/_gallery");
const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E804/axe");
const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

type Finding = { id: string; impact: string; help: string; nodes: number; targets: string[]; measured?: string[]; why?: string };
type Scan = { state: string; counts: Record<string, number>; findings: Finding[] };

/**
 * The canvas is a layered gradient, and axe cannot compute contrast over
 * a gradient — it files every such text node as "incomplete", not as a
 * violation, so a scan of the real page silently skips most contrast
 * checks. For the scan only, the gradient is replaced by its
 * conservative end, `--color-canvas-hi` (lighter in dark, darker in
 * light: the lower contrast either way); the capture is taken after the
 * override is removed.
 */
const SOLID_CANVAS =
  "body, [data-theme] body { background-image: none !important; background-color: var(--color-canvas-hi) !important; }";

export async function scan(page: import("@playwright/test").Page): Promise<Finding[]> {
  const solid = await page.addStyleTag({ content: SOLID_CANVAS });
  try {
    return await scanAsIs(page);
  } finally {
    await solid.evaluate((el) => el.remove());
  }
}

async function scanAsIs(page: import("@playwright/test").Page): Promise<Finding[]> {
  const run = async () =>
    (await new AxeBuilder({ page }).withTags(TAGS).analyze()).violations.map((v) => ({
      id: v.id,
      impact: v.impact ?? "minor",
      help: v.help,
      nodes: v.nodes.length,
      targets: v.nodes.slice(0, 5).map((n) => n.target.join(" ")),
      why: v.nodes[0]?.failureSummary?.replace(/\s+/g, " ").slice(0, 300),
      // Contrast nodes carry axe's own measurement: fg, bg, ratio.
      measured: v.nodes.slice(0, 5).flatMap((n) => {
        const d = n.any[0]?.data as { fgColor?: string; bgColor?: string; contrastRatio?: number } | undefined;
        return d?.fgColor ? [`${d.fgColor} on ${d.bgColor} = ${d.contrastRatio}:1`] : [];
      }),
    }));
  let findings = await run();
  // AJ-E806: one confirmatory re-scan after a settle beat — a transition
  // still settling under the parallel suite does not survive it; a real
  // violation reproduces and still fails.
  if (findings.some((f) => f.impact === "critical" || f.impact === "serious")) {
    await page.waitForTimeout(400);
    findings = await run();
  }
  return findings;
}

/** The families already given their gate state (per worker; every
 * worker registers the tests in the same order). */
const gated = new Set<string>();

/** The gate's clock (2024-10-13): one instant for every gate screen,
 * the regression screens' and the gallery's gate states alike. */
export const GATE_NOW = 1_728_800_000_000;

/** Tags the first test `family` registers as its gate state: in a loop,
 * its first cell (themes list dark first). */
export function gate(family: string): { tag?: string } {
  if (gated.has(family)) return {};
  gated.add(family);
  return { tag: "@gate" };
}

type Page = import("@playwright/test").Page;
type Locator = import("@playwright/test").Locator;

/** A page settled for a capture: what loads after the first paint (lazy
 * screens, the prefetch, data) has landed. A capture taken before it
 * raced it: the capture spec's determinism test differed 1 run in 3.
 * Cheap once landed (it waits only on the page's first load). */
export async function settle(page: Page): Promise<void> {
  await page.waitForLoadState("networkidle");
}

/** In the gate's `gallery` project, a capture (the page's or one
 * element's) is compared with its approved baseline as the regression
 * screens are (≤ 0.1 % of pixels). Returns how many it has compared so
 * far, and how to put element captures back once the test is done.
 *
 * Each comparison is soft: a differing capture fails the test but the
 * flow goes on to its next capture, so one run writes the actual of
 * every capture that changed. A hard stop at the first one hid the rest
 * behind it, and baselines approved from that run's actuals left those
 * stale for the next run to find (0.1.5: 43 of 45 "diffs"). */
function compareInstead(page: Page): { compared: () => number; restore: () => void } {
  let compared = 0;
  const baseline = (path: string | undefined): string[] => {
    if (!path) throw new Error("a gate capture needs its gallery path");
    return ["gallery", ...relative(GALLERY, path).split("/")];
  };
  const same = { maxDiffPixelRatio: 0.001, animations: "disabled", caret: "hide" } as const;
  page.screenshot = (async (options?: Parameters<Page["screenshot"]>[0]) => {
    const name = baseline(options?.path);
    // What loads after the first paint has landed (see settle()).
    await settle(page);
    await expect.soft(page).toHaveScreenshot(name, { ...same, fullPage: options?.fullPage, clip: options?.clip });
    compared += 1;
    return Buffer.alloc(0);
  }) as Page["screenshot"];
  // Every locator shares one prototype; a worker runs one test at a time.
  const locators = Object.getPrototypeOf(page.locator("body")) as Locator;
  const own = locators.screenshot;
  locators.screenshot = async function (this: Locator, options?: Parameters<Locator["screenshot"]>[0]) {
    const name = baseline(options?.path);
    await settle(this.page());
    await expect.soft(this).toHaveScreenshot(name, same);
    compared += 1;
    return Buffer.alloc(0);
  };
  return {
    compared: () => compared,
    restore: () => {
      locators.screenshot = own;
    },
  };
}

export const test = base.extend({
  page: async ({ page }, use, testInfo) => {
    if (testInfo.project.name === "regression") {
      // The gate's one clock seam: `Date.now()` and `new Date()` both read
      // GATE_NOW, so a screen that reads the clock ("5 h ago", a chart's
      // days, the month a budget counts from) renders the same any day.
      await page.clock.setFixedTime(GATE_NOW);
      await use(page);
      return;
    }
    if (testInfo.project.name === "gallery") {
      const gate = compareInstead(page);
      // The same clock for every gate state: a state that reads the clock
      // ("740 d ago") would differ tomorrow. A spec that pins its own
      // `Date.now` still wins (its init script runs later).
      await page.clock.setFixedTime(GATE_NOW);
      // The gate's self-test, as the regression screens have it: a 2 px
      // shift of every button must fail the comparison (AJ-E803).
      if (process.env.AIJ_VISUAL_SELFTEST === "1") {
        await page.addInitScript(() => {
          document.addEventListener("DOMContentLoaded", () => {
            const style = document.createElement("style");
            style.textContent = "button { transform: translateX(2px); }";
            document.head.append(style);
          });
        });
      }
      await use(page);
      gate.restore();
      // A gate state that captures nothing would pass while checking nothing.
      if (testInfo.status === testInfo.expectedStatus && gate.compared() === 0) {
        throw new Error(`the gate state "${testInfo.title}" compared no capture`);
      }
      return;
    }
    if (!ON) {
      await use(page);
      return;
    }
    const scans: Scan[] = [];
    const shoot = page.screenshot.bind(page);
    page.screenshot = (async (options?: Parameters<typeof page.screenshot>[0]) => {
      if (options?.path) {
        const findings = await scan(page);
        const counts: Record<string, number> = { critical: 0, serious: 0, moderate: 0, minor: 0 };
        for (const f of findings) counts[f.impact] = (counts[f.impact] ?? 0) + 1;
        const state = relative(GALLERY, options.path);
        scans.push({ state, counts, findings });
        const blocking = findings.filter((f) => f.impact === "critical" || f.impact === "serious");
        expect.soft(blocking, `axe critical/serious on ${state}`).toEqual([]);
      }
      return shoot(options);
    }) as typeof page.screenshot;
    await use(page);
    if (scans.length > 0) {
      mkdirSync(OUT, { recursive: true });
      const name = testInfo.titlePath.slice(1).join(" — ").replace(/[^\w.-]+/g, "_").slice(0, 150);
      writeFileSync(resolve(OUT, `${name}.json`), JSON.stringify({ test: testInfo.title, scans }, null, 2) + "\n");
    }
  },
});

export { expect };
