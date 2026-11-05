/**
 * 0.4's new gallery families share this: the app opened in a theme and a
 * size with the preview's data (and any seam a state needs), a settled
 * capture into its family's folder, and axe on each (0 critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, GATE_NOW, settle } from "./axe-gallery";

export type Theme = "dark" | "light";
export type Size = { width: number; height: number };
export const SIZES: Size[] = [
  { width: 1280, height: 800 },
  { width: 1024, height: 700 },
];
export const THEMES: Theme[] = ["dark", "light"];
export const at = (size: Size) => `${size.width}x${size.height}`;

/** The family's folder under the gallery, made once. */
export function family(name: string): string {
  const out = resolve(process.cwd(), `../docs/qa/evidence/_gallery/${name}`);
  mkdirSync(out, { recursive: true });
  return out;
}

/** The app in `theme` at `size`, with `seams` set on window before it loads. */
export async function open(page: Page, theme: Theme, size: Size, seams: Record<string, unknown> = {}, storage: Record<string, string> = {}): Promise<void> {
  await page.addInitScript(
    ([theme, seams, storage, now]) => {
      localStorage.setItem("aij.theme", theme as string);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      for (const [key, value] of Object.entries(storage as Record<string, string>)) localStorage.setItem(key, value);
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => now as number;
      for (const [key, value] of Object.entries(seams as Record<string, unknown>)) (window as unknown as Record<string, unknown>)[key] = value;
    },
    [theme, seams, storage, GATE_NOW] as const,
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** Axe on the page as it is: no critical or serious finding. */
export async function axe(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(
    bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`),
    `axe on ${what}`,
  ).toEqual([]);
}

/** A settled capture of the page into `dir`, then axe on it. */
export async function shot(page: Page, dir: string, name: string): Promise<void> {
  await page.mouse.move(0, 0);
  await settle(page);
  await page.screenshot({ path: resolve(dir, `${name}.png`) });
  await axe(page, name);
}
