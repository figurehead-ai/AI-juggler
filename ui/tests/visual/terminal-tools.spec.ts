/**
 * AJ-E1506 — the terminal's tools, in the design preview: the Live word,
 * find (with its budget: 10,000 lines under 100 ms), file-path and web
 * links (⌘-click), copy, text size, jump to the bottom, maximize (Esc
 * outside the terminal restores) and its own window (the main window then
 * says where it went). Gallery in both themes into `_gallery/82-terminal-tools/`;
 * axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/82-terminal-tools");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));
const MOD = process.platform === "darwin" ? "Meta" : "Control+Shift";

async function open(page: Page, theme: "dark" | "light" = "dark") {
  await page.addInitScript((theme) => {
    localStorage.setItem("aij.theme", theme);
    // AJ-E1522: a pinned clock, so the capture is the same every run.
    Date.now = () => 1_728_800_000_000;
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, theme);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  // AJ-E2104: an AI Juggler session's row, under AI Juggler's place: a
  // click opens its page, on its Terminal tab (AJ-E1533).
  await page.getByTestId("rail").locator('[data-instance-id="sess-live-1"]').click();
  await expect(page.getByRole("tab", { name: "Terminal" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
  return page.locator("[data-terminal-host]").first().getAttribute("data-terminal-host") as Promise<string>;
}

/** Writes lines into terminal `id` and waits until xterm has them. */
async function write(page: Page, id: string, lines: string[]) {
  await page.evaluate(
    ([id, text]) =>
      new Promise<void>((done) => {
        const seams = window as unknown as { __AIJ_TERMINALS__: Record<string, { term: { write: (s: string, cb: () => void) => void } }> };
        seams.__AIJ_TERMINALS__[id]!.term.write(text, done);
      }),
    [id, lines.join("\r\n") + "\r\n"] as const,
  );
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

test("live, find with its keys and count, and the 10,000-line budget", async ({ page }) => {
  const id = await open(page);
  await expect(page.getByTestId("terminal-connection")).toHaveAttribute("data-connection", "live");
  await page.locator(".xterm-helper-textarea").first().focus();
  await page.keyboard.press(`${MOD}+f`);
  const input = page.getByTestId("terminal-search-input");
  await expect(input).toBeFocused();
  await input.fill("Ask");
  await expect(page.getByTestId("terminal-search-count")).toHaveText("1 of 1");
  await input.fill("nothing like this");
  await expect(page.getByTestId("terminal-search-count")).toHaveText("No matches");
  await input.press("Escape");
  await expect(page.getByTestId("terminal-search")).toHaveCount(0);
  await expect(page.locator(".xterm-helper-textarea").first()).toBeFocused();

  await write(page, id, Array.from({ length: 10_000 }, (_, i) => `line ${i} of the build log`));
  const timings = await page.evaluate((id) => {
    const seams = window as unknown as { __AIJ_TERMINALS__: Record<string, { search: { findNext: (q: string, o?: object) => boolean; findPrevious: (q: string, o?: object) => boolean } }> };
    const { search } = seams.__AIJ_TERMINALS__[id]!;
    const time = (run: () => void) => {
      const start = performance.now();
      run();
      return performance.now() - start;
    };
    return {
      oneMatch: time(() => search.findPrevious("line 17 of")),
      noMatch: time(() => search.findNext("not in the log at all")),
      everyLineMarked: time(() => search.findNext("build log", { decorations: { matchOverviewRuler: "#7c7cf4", activeMatchColorOverviewRuler: "#f4b73e" } })),
    };
  }, id);
  writeFileSync(resolve(OUT, "search-timings.json"), JSON.stringify({ lines: 10_000, ms: timings }, null, 2) + "\n");
  for (const [what, ms] of Object.entries(timings)) expect(ms, `${what} over 10,000 lines`).toBeLessThan(100);
});

test("paths and web links open on ⌘-click, saying so on hover", async ({ page }) => {
  const id = await open(page);
  await write(page, id, ["● Edit src/auth/session.test.ts:42 — see https://example.com/docs"]);
  const row = page.locator(".xterm-rows > div", { hasText: "src/auth/session.test.ts" }).first();
  const box = (await row.boundingBox())!;
  const cols = await page.evaluate((id) => (window as unknown as { __AIJ_TERMINALS__: Record<string, { term: { cols: number } }> }).__AIJ_TERMINALS__[id]!.term.cols, id);
  const cell = box.width / cols;
  const text = await row.innerText();
  const at = (needle: string) => ({ x: box.x + (text.indexOf(needle) + 3) * cell, y: box.y + box.height / 2 });
  await page.mouse.move(at("src/auth").x, at("src/auth").y);
  await expect(page.getByTestId("terminal-link-hint")).toContainText("opens src/auth/session.test.ts:42");
  await page.screenshot({ path: resolve(OUT, "link-hint-1280x800-dark.png") });
  // A plain click selects as ever; ⌘-click (Ctrl-click) opens.
  await page.mouse.click(at("src/auth").x, at("src/auth").y);
  expect(await page.evaluate(() => (window as unknown as { __AIJ_OPENED_LINK__?: string }).__AIJ_OPENED_LINK__)).toBeUndefined();
  await page.keyboard.down(process.platform === "darwin" ? "Meta" : "Control");
  await page.mouse.click(at("src/auth").x, at("src/auth").y);
  expect(await page.evaluate(() => (window as unknown as { __AIJ_OPENED_LINK__?: string }).__AIJ_OPENED_LINK__)).toBe("src/auth/session.test.ts:42");
  await page.mouse.move(at("example.com").x, at("example.com").y);
  await page.mouse.click(at("example.com").x, at("example.com").y);
  await page.keyboard.up(process.platform === "darwin" ? "Meta" : "Control");
  expect(await page.evaluate(() => (window as unknown as { __AIJ_OPENED_LINK__?: string }).__AIJ_OPENED_LINK__)).toBe("https://example.com/docs");
});

test("copy, text size and jump to the bottom", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const id = await open(page);
  await page.getByTestId("terminal-copy").click();
  await expect(page.getByTestId("terminal-note")).toHaveText("Copied what's on screen");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain("Ask anything");

  const size = () => page.evaluate((id) => (window as unknown as { __AIJ_TERMINALS__: Record<string, { term: { options: { fontSize: number } } }> }).__AIJ_TERMINALS__[id]!.term.options.fontSize, id);
  await page.getByTestId("terminal-larger").click();
  await expect.poll(size).toBe(13);
  await page.locator(".xterm-helper-textarea").first().focus();
  await page.keyboard.press(`${MOD}+Minus`);
  await expect.poll(size).toBe(12);
  await page.keyboard.press(`${MOD}+Digit0`);
  await expect.poll(size).toBe(12.5);

  await write(page, id, Array.from({ length: 300 }, (_, i) => `output ${i}`));
  await expect(page.getByTestId("terminal-jump-bottom")).toHaveCount(0);
  await page.evaluate((id) => (window as unknown as { __AIJ_TERMINALS__: Record<string, { term: { scrollLines: (n: number) => void } }> }).__AIJ_TERMINALS__[id]!.term.scrollLines(-100), id);
  await page.getByTestId("terminal-jump-bottom").click();
  await expect(page.getByTestId("terminal-jump-bottom")).toHaveCount(0);
});

for (const theme of ["dark", "light"] as const) {
  test(`maximize, find and its own window — ${theme}`, gate("82-terminal-tools"), async ({ page }) => {
    await open(page, theme);
    await page.getByTestId("terminal-find").click();
    await page.getByTestId("terminal-search-input").fill("Ask");
    await expect(page.getByTestId("terminal-search-count")).toHaveText("1 of 1");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `toolbar-find-1280x800-${theme}.png`) });
    await axe(page, `terminal toolbar and find ${theme}`);
    await page.getByTestId("terminal-search-close").click();

    await page.getByTestId("terminal-maximize").click();
    const terminal = page.getByTestId("terminal");
    await expect(terminal).toHaveAttribute("data-maximized", "true");
    expect(await terminal.boundingBox()).toEqual({ x: 0, y: 0, width: 1280, height: 800 });
    await page.screenshot({ path: resolve(OUT, `maximized-1280x800-${theme}.png`) });
    await axe(page, `maximized terminal ${theme}`);
    // Esc in the terminal is the agent's; outside it, Esc restores.
    await page.locator(".xterm-helper-textarea").first().focus();
    await page.keyboard.press("Escape");
    await expect(terminal).toHaveAttribute("data-maximized", "true");
    await page.getByTestId("terminal-maximize").focus();
    await page.keyboard.press("Escape");
    await expect(terminal).toHaveAttribute("data-maximized", "false");

    await page.getByTestId("terminal-window").click();
    await expect(page.getByTestId("terminal-away")).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `own-window-placeholder-1280x800-${theme}.png`) });
    await axe(page, `terminal in its own window ${theme}`);
    await page.getByRole("button", { name: "Bring it back here" }).click();
    await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
  });
}
