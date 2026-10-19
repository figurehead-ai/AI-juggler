/**
 * AJ-E804 Req.1 — axe-core on every gallery state. The gallery spec
 * imports `test` from here instead of `@playwright/test`; with
 * `AJ_AXE_GALLERY=1` every `page.screenshot({ path })` first scans the
 * page exactly as captured (WCAG 2.0/2.1/2.2 A and AA rules) and fails
 * the test on any critical or serious violation. One JSON per test
 * lands in `docs/qa/evidence/AJ-E804/axe/`; `scripts/axe_summary.py`
 * turns them into the summary table. Without the variable this is
 * plain Playwright — the gallery runs exactly as before.
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

export const test = base.extend({
  page: async ({ page }, use, testInfo) => {
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
