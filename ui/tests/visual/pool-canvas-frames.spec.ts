/**
 * AJ-E2808's performance budget: 50 members and 200 edges drag at 60 fps
 * (≤ 16 ms a frame), traced to `docs/qa/evidence/AJ-E2808/frames.json`.
 *
 * Measured from a Chrome trace: the renderer main thread's work in each
 * frame while the node is dragged (headless Chromium paces its frames at
 * about 17–18 ms whatever they cost, so their interval can't say).
 *
 * A timing run, not a gate on every machine: it runs when `AIJ_FRAMES=1`
 * (on the Mac it ships on), and fails there when a p95 passes 16 ms.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";
import { bigCanvasSeed } from "./fixtures/pool-canvas";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E2808");
const NOW = Date.UTC(2026, 10, 6, 10, 0, 0);
const MOVES = 120;

test.skip(!process.env.AIJ_FRAMES, "a timing run: AIJ_FRAMES=1 to trace it");

async function openBigPool(page: Page) {
  await page.addInitScript((seed) => {
    Object.assign(window, seed);
    localStorage.setItem("aij.theme", "dark");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, bigCanvasSeed(NOW));
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
  await goTo(page, "Pools");
  await page.getByTestId("pool-card-tile").filter({ hasText: "Fifty agents" }).click();
  await expect(page.getByTestId("pool-canvas")).toBeVisible();
  await expect(page.getByTestId("canvas-node")).toHaveCount(50);
}

const percentile = (values: number[], p: number) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1)] ?? 0;
};
const round = (value: number) => Math.round(value * 100) / 100;
const summary = (values: number[]) => ({ count: values.length, p50: round(percentile(values, 50)), p95: round(percentile(values, 95)), max: round(Math.max(...values)) });

test("a 50-member, 200-edge pool drags at ≤ 16 ms a frame", async ({ page, browser, browserName }) => {
  await openBigPool(page);
  const edges = await page.locator("[data-edge]").count();
  const node = page.locator('[data-node="s-big-01"]');
  const box = (await node.boundingBox())!;
  await page.mouse.move(box.x + 30, box.y + 10);
  // Nothing covers the node where the drag starts.
  expect(await page.evaluate(([x, y]) => document.elementFromPoint(x!, y!)?.closest("[data-node]")?.getAttribute("data-node"), [box.x + 30, box.y + 10])).toBe("s-big-01");
  await browser.startTracing(page, { categories: ["toplevel", "devtools.timeline", "disabled-by-default-devtools.timeline.frame"] });
  await page.mouse.down();
  for (let step = 1; step <= MOVES; step += 1) {
    const angle = (step / MOVES) * Math.PI * 2;
    await page.mouse.move(box.x + 30 + Math.sin(angle) * 160 + step * 1.5, box.y + 10 + Math.sin(angle * 2) * 110);
  }
  await page.mouse.up();
  const frames = mainThreadFrames(JSON.parse((await browser.stopTracing()).toString("utf8")));
  const result = {
    task: "AJ-E2808",
    what: "the lead's node of a 50-member, 200-edge pool dragged in loops 180 px across (120 pointer moves) at 1440×900, dark",
    how: "a Chrome trace (toplevel, devtools.timeline): the renderer main thread's work in each frame, from one BeginMainThreadFrame to the next (script, style, layout, paint); headless Chromium paces frames itself, so their interval says nothing of the cost",
    browser: `${browserName} ${browser.version()}`,
    nodes: await page.getByTestId("canvas-node").count(),
    edges_drawn: edges,
    budget_ms: 16,
    frame_work_ms: summary(frames),
    raw: { frame_work_ms: frames.map(round) },
  };
  mkdirSync(OUT, { recursive: true });
  writeFileSync(resolve(OUT, "frames.json"), `${JSON.stringify(result, null, 2)}\n`);
  expect(result.nodes).toBe(50);
  // The drag moved the node: one place saved at its end.
  const saves = () => page.evaluate(() => {
    const calls: unknown = Reflect.get(window, "__AIJ_POOL_CANVAS_CALLS__");
    return Array.isArray(calls) ? calls.filter((called) => String(called).includes("s-big-01@")).length : 0;
  });
  await expect.poll(saves).toBe(1);
  expect(result.frame_work_ms.count).toBeGreaterThan(60);
  expect(result.frame_work_ms.max).toBeGreaterThan(0); // the trace saw the work
  expect(result.frame_work_ms.p95).toBeLessThanOrEqual(16);
});

type TraceEvent = { name: string; ph: string; ts: number; dur?: number; tid: number; pid: number; args?: { name?: string } };

/** The renderer main thread's busy time in each frame, in ms: its
 * top-level tasks, cut at each BeginMainThreadFrame. */
function mainThreadFrames(trace: { traceEvents: TraceEvent[] }): number[] {
  const events = trace.traceEvents;
  const main = events.find((event) => event.ph === "M" && event.name === "thread_name" && event.args?.name === "CrRendererMain");
  if (!main) throw new Error("no renderer main thread in the trace");
  const mine = events.filter((event) => event.pid === main.pid && event.tid === main.tid);
  const starts = mine.filter((event) => event.name === "BeginMainThreadFrame").map((event) => event.ts).sort((a, b) => a - b);
  const tasks = mine.filter((event) => event.ph === "X" && event.name === "ThreadControllerImpl::RunTask" && event.dur !== undefined);
  const out: number[] = [];
  for (let index = 0; index + 1 < starts.length; index += 1) {
    const [from, to] = [starts[index]!, starts[index + 1]!];
    const busy = tasks.reduce((sum, task) => sum + Math.max(0, Math.min(task.ts + task.dur!, to) - Math.max(task.ts, from)), 0);
    out.push(busy / 1000);
  }
  return out;
}
