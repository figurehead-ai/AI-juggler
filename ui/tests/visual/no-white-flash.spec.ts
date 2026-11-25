/**
 * AJ-E1549: no white flash, ever. In the dark theme, every frame the page
 * paints while it switches screens, steps through the add-an-account
 * wizard to its sign-in terminal and opens a session's terminal panel is
 * checked, two ways:
 *
 * - a probe in the page, every animation frame: what is painted at a grid
 *   of points (the topmost element with a background, down to the page's
 *   own canvas: a page whose root drew nothing shows the browser's white
 *   there), counted near white or not;
 * - screenshots in quick succession, the composited pixels.
 *
 * Any frame mostly near white fails. The native window behind the page
 * (WKWebView's own background, tauri.conf.json's `backgroundColor`) is out
 * of a browser's reach: `index.html`'s and the window's colours are kept
 * one with the tokens by `tests/bundle/theme-base.test.ts` and the app's own test
 * (`webview_guard.rs`). The seeded test proves the probe sees a flash.
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses, goTo, honestEmpty, openRailSession } from "./fixtures";
import type { FixtureSet } from "../../src/preview/fixtures";

/** A channel this high or more on all three is "near white". */
const NEAR_WHITE = 225;
/** A frame with more than this share of near-white points fails. */
const MOSTLY = 0.5;

type Flash = { frame: number; share: number; at: string };
type Probe = { frames: number; flashes: Flash[]; stop: boolean };

/** Installed before the page's own scripts: samples every frame. */
function installProbe(limits: { nearWhite: number; mostly: number }): void {
  const w = window as unknown as { __AIJ_FLASH__: Probe };
  const probe: Probe = { frames: 0, flashes: [], stop: false };
  w.__AIJ_FLASH__ = probe;
  const swatch = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
  const cache = new Map<string, [number, number, number, number]>();
  // Any CSS colour (rgb, oklch, color(), hex) as sRGB bytes, by drawing it.
  const rgba = (css: string): [number, number, number, number] => {
    const known = cache.get(css);
    if (known) return known;
    let out: [number, number, number, number] = [0, 0, 0, 0];
    if (swatch) {
      swatch.clearRect(0, 0, 1, 1);
      swatch.fillStyle = "rgba(0,0,0,0)";
      swatch.fillStyle = css;
      swatch.fillRect(0, 0, 1, 1);
      const [r, g, b, a] = swatch.getImageData(0, 0, 1, 1).data;
      out = [r!, g!, b!, a! / 255];
    }
    cache.set(css, out);
    return out;
  };
  const firstColour = (image: string): string | null => image.match(/(rgba?\([^)]*\)|oklch\([^)]*\)|oklab\([^)]*\)|color\([^)]*\)|#[0-9a-f]{3,8})/i)?.[1] ?? null;
  /** The background an element paints, if it paints one. */
  const paint = (el: Element): [number, number, number, number] | null => {
    const style = getComputedStyle(el);
    const colour = rgba(style.backgroundColor);
    if (colour[3] >= 0.9) return colour;
    const image = style.backgroundImage !== "none" ? firstColour(style.backgroundImage) : null;
    if (image) {
      const stop = rgba(image);
      if (stop[3] >= 0.9) return stop;
    }
    return null;
  };
  /** What is painted at (x, y): "unknown" for a canvas or an image. */
  const at = (x: number, y: number): [number, number, number, number] | "unknown" => {
    for (const el of document.elementsFromPoint(x, y)) {
      if (el instanceof HTMLCanvasElement || el instanceof HTMLImageElement || el instanceof HTMLVideoElement) return "unknown";
      if (el === document.documentElement || el === document.body) continue;
      if (getComputedStyle(el).visibility === "hidden") continue;
      const own = paint(el);
      if (own) return own;
    }
    // The root and the body: the root's background is the window's; with
    // none, the body's spreads to it; with neither, the browser's canvas
    // (white, unless the page asked for a dark colour scheme).
    const root = paint(document.documentElement) ?? (document.body ? paint(document.body) : null);
    if (root) return root;
    return getComputedStyle(document.documentElement).colorScheme.includes("dark") ? [18, 18, 18, 1] : [255, 255, 255, 1];
  };
  const tick = () => {
    if (probe.stop) return;
    probe.frames += 1;
    const width = window.innerWidth;
    const height = window.innerHeight;
    let white = 0;
    let known = 0;
    for (let col = 0; col < 8; col += 1) {
      for (let row = 0; row < 5; row += 1) {
        const point = at(((col + 0.5) / 8) * width, ((row + 0.5) / 5) * height);
        if (point === "unknown") continue;
        known += 1;
        if (point[0] >= limits.nearWhite && point[1] >= limits.nearWhite && point[2] >= limits.nearWhite) white += 1;
      }
    }
    const share = known === 0 ? 0 : white / 40;
    if (share > limits.mostly && probe.flashes.length < 20) {
      probe.flashes.push({ frame: probe.frames, share, at: `${location.pathname}${location.search} ${document.title}` });
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

/** Screenshots taken back to back until `done` resolves. */
async function screenshotsUntil(page: Page, done: Promise<unknown>): Promise<Buffer[]> {
  const shots: Buffer[] = [];
  let finished = false;
  void done.finally(() => (finished = true));
  while (!finished && shots.length < 400) {
    try {
      shots.push(await page.screenshot({ scale: "css", animations: "allow", caret: "initial", timeout: 2_000 }));
    } catch {
      // A screenshot that met a navigation or a closing page: the next one.
    }
  }
  return shots;
}

/** Each screenshot's share of near-white pixels (read in a page of their own). */
async function whiteShares(page: Page, shots: Buffer[]): Promise<number[]> {
  const reader = await page.context().newPage();
  try {
    return await reader.evaluate(
      async ({ images, nearWhite }) => {
        const shares: number[] = [];
        for (const base64 of images) {
          const bitmap = await createImageBitmap(await (await fetch(`data:image/png;base64,${base64}`)).blob());
          const canvas = new OffscreenCanvas(64, 40);
          const ctx = canvas.getContext("2d", { willReadFrequently: true })!;
          ctx.drawImage(bitmap, 0, 0, 64, 40);
          const data = ctx.getImageData(0, 0, 64, 40).data;
          let white = 0;
          for (let i = 0; i < data.length; i += 4) {
            if (data[i]! >= nearWhite && data[i + 1]! >= nearWhite && data[i + 2]! >= nearWhite) white += 1;
          }
          shares.push(white / (64 * 40));
        }
        return shares;
      },
      { images: shots.map((shot) => shot.toString("base64")), nearWhite: NEAR_WHITE },
    );
  } finally {
    await reader.close();
  }
}

async function open(page: Page, fixture: Partial<FixtureSet>, extra: Record<string, unknown> = {}): Promise<void> {
  await page.addInitScript(installProbe, { nearWhite: NEAR_WHITE, mostly: MOSTLY });
  await page.addInitScript(
    ({ fixture, extra }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(extra)) w[key] = value;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture, extra },
  );
  await page.emulateMedia({ reducedMotion: "no-preference", colorScheme: "dark" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

/** A terminal on screen, drawn by either renderer (the gate's browser
 * has no WebGL and draws rows; WebKit draws on its canvas). */
async function terminalShown(page: Page): Promise<void> {
  await expect(page.locator(".xterm-screen").first()).toBeVisible();
}

/** Runs `walk` with both samplers on; fails on any mostly-white frame. */
async function noWhiteFrame(page: Page, walk: () => Promise<void>): Promise<{ frames: number; shots: number }> {
  const walked = walk();
  const shots = await screenshotsUntil(page, walked);
  await walked;
  const probe = await page.evaluate(() => {
    const w = window as unknown as { __AIJ_FLASH__: Probe };
    w.__AIJ_FLASH__.stop = true;
    return w.__AIJ_FLASH__;
  });
  const shares = await whiteShares(page, shots);
  const whiteShots = shares.map((share, n) => ({ n, share })).filter((shot) => shot.share > MOSTLY);
  expect(probe.frames, "the probe ran every frame").toBeGreaterThan(30);
  expect(shots.length, "screenshots were taken").toBeGreaterThan(5);
  expect(probe.flashes, "frames painted mostly near white (the probe)").toEqual([]);
  expect(whiteShots, "screenshots mostly near white").toEqual([]);
  return { frames: probe.frames, shots: shots.length };
}

const ONE_TOOL = [
  { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
];
const NO_SCAN = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };

test.describe("AJ-E1549: no white frame, dark theme", () => {
  test("switching screens and opening a session's terminal panel", async ({ page }) => {
    await open(page, allStatuses);
    const seen = await noWhiteFrame(page, async () => {
      await goTo(page, "Home");
      for (const name of ["Projects", "Usage", "History", "Tasks", "Pools", "Settings", "Processes", "This Mac", "Home"]) {
        await goTo(page, name);
        await page.waitForTimeout(120);
      }
      // A live session: its page, its terminal drawn.
      await openRailSession(page, "sess-2");
      await terminalShown(page);
      await page.waitForTimeout(200);
      // Another live one, and back: the terminal panel opened again.
      await openRailSession(page, "sess-4");
      await page.waitForTimeout(200);
      await openRailSession(page, "sess-2");
      await terminalShown(page);
      await page.waitForTimeout(200);
    });
    test.info().annotations.push({ type: "sampled", description: `${seen.frames} frames, ${seen.shots} screenshots` });
  });

  test("the add-an-account wizard, to its sign-in terminal", async ({ page }) => {
    await open(page, honestEmpty, { __AIJ_PROVIDER_INSTALLS__: ONE_TOOL, __AIJ_SCAN_REPORT__: NO_SCAN, __AIJ_SERVICES__: [] });
    const seen = await noWhiteFrame(page, async () => {
      await goTo(page, "Accounts");
      await page.getByTestId("add-account").click();
      await expect(page.getByTestId("first-run-fresh")).toBeVisible();
      await page.getByTestId("first-run-fresh-create").click();
      const row = page.getByTestId("first-run-signin-row");
      await expect(row).toContainText("Claude Code");
      await row.getByRole("button", { name: /^Sign in/ }).click();
      // The sign-in's terminal, live in the wizard.
      await terminalShown(page);
      await page.waitForTimeout(300);
    });
    test.info().annotations.push({ type: "sampled", description: `${seen.frames} frames, ${seen.shots} screenshots` });
  });

  // The probe itself: a page whose background goes for a moment (the
  // root's and the body's, as an empty root over no window colour would
  // show) must fail it. Seeded, so the check can't pass by seeing nothing.
  test("seeded: a moment with no page background is caught", async ({ page }) => {
    await open(page, allStatuses);
    await goTo(page, "Home");
    await page.evaluate(async () => {
      const style = document.createElement("style");
      style.textContent = "html, body, body * { background: transparent !important; }";
      document.head.append(style);
      await new Promise((done) => setTimeout(done, 250));
      style.remove();
    });
    const flashes = await page.evaluate(() => (window as unknown as { __AIJ_FLASH__: Probe }).__AIJ_FLASH__.flashes);
    expect(flashes.length, "the seeded flash was seen").toBeGreaterThan(0);
  });
});
