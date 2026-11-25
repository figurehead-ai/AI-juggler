// @vitest-environment node
/**
 * AJ-E1549: no white flash. The page's base colour is painted in four
 * places before and under the page: the native window (tauri.conf.json,
 * the dark default), the window again at run time (theme.ts), and
 * index.html's inline CSS, before any script. Each is a literal copy of
 * `--color-canvas`, read here from styles.css, so a token change that
 * left one behind fails.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { BASE_COLOUR, WINDOW_THEME_KEY } from "../../src/theme";

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), "utf8");
const css = read("../../src/styles.css");
const html = read("../../index.html");
const windowFrame = read("../../../crates/juggler-app/src/window_frame.rs");
const config = JSON.parse(read("../../../crates/juggler-app/tauri.conf.json")) as { app: { windows: { label: string; backgroundColor?: string }[] } };

/** `--color-canvas` in the dark theme (the @theme block) and the light one. */
function canvas(): { dark: string; light: string } {
  const light = css.indexOf('[data-theme="light"] {');
  const found = [...css.matchAll(/--color-canvas:\s*(#[0-9a-fA-F]{6})/g)];
  const dark = found.find((match) => match.index! < light)?.[1];
  const lit = found.find((match) => match.index! > light)?.[1];
  if (!dark || !lit) throw new Error("styles.css has no --color-canvas for both themes");
  return { dark: dark.toLowerCase(), light: lit.toLowerCase() };
}

/** index.html's inline CSS before its first script. */
function headCss(): string {
  const firstScript = html.indexOf("<script");
  return [...html.slice(0, firstScript).matchAll(/<style>([\s\S]*?)<\/style>/g)].map((match) => match[1]!).join("\n");
}

/** The background a selector gets in `block` (the last rule naming it). */
function backgroundOf(block: string, selector: string): string | undefined {
  const rules = [...block.matchAll(/([^{}]+)\{\s*background:\s*(#[0-9a-fA-F]{6});?\s*\}/g)];
  const named = rules.filter((rule) => rule[1]!.split(",").map((part) => part.trim()).includes(selector));
  return named.at(-1)?.[2]?.toLowerCase();
}

describe("the base colour, from the first frame", () => {
  it("theme.ts paints the window with each theme's canvas", () => {
    expect(BASE_COLOUR).toEqual(canvas());
  });

  it("the main window is made on the dark canvas (the default theme)", () => {
    const main = config.app.windows.find((window) => window.label === "main");
    expect(main?.backgroundColor?.toLowerCase()).toBe(canvas().dark);
  });

  it("the backend makes the window in the chosen theme's canvas (0.5.1 F21)", () => {
    // The light colour it paints when Light is chosen, whatever macOS shows.
    const light = windowFrame.match(/LIGHT_COLOUR: Color = Color\(0x([0-9a-f]{2}), 0x([0-9a-f]{2}), 0x([0-9a-f]{2}), 0xff\)/);
    expect(light && `#${light[1]}${light[2]}${light[3]}`).toBe(canvas().light);
    // The setting the page writes is the one it reads.
    expect(windowFrame).toContain(`pub const THEME_KEY: &str = "${WINDOW_THEME_KEY}";`);
  });

  it("index.html paints html and body in the canvas before any script", () => {
    const head = headCss();
    expect(head, "inline CSS before the first <script>").not.toBe("");
    const { dark, light } = canvas();
    // No theme yet: dark, and macOS's own in either case.
    for (const selector of ["html", "body"]) expect(backgroundOf(head, selector), selector).toBe(dark);
    const media = (scheme: string) => head.match(new RegExp(`@media \\(prefers-color-scheme: ${scheme}\\)\\s*\\{([^@]*?\\})\\s*\\}`))?.[1] ?? "";
    expect(backgroundOf(media("light"), "html:not([data-theme])")).toBe(light);
    expect(backgroundOf(media("light"), "html:not([data-theme]) body")).toBe(light);
    expect(backgroundOf(media("dark"), "html:not([data-theme])")).toBe(dark);
    expect(backgroundOf(media("dark"), "html:not([data-theme]) body")).toBe(dark);
    // The stored theme, once theme-boot.js has set it.
    expect(backgroundOf(head, 'html[data-theme="dark"]')).toBe(dark);
    expect(backgroundOf(head, 'html[data-theme="dark"] body')).toBe(dark);
    expect(backgroundOf(head, 'html[data-theme="light"]')).toBe(light);
    expect(backgroundOf(head, 'html[data-theme="light"] body')).toBe(light);
  });

  it("the inline colours sit in the lowest layer, so styles.css's body still wins", () => {
    expect(headCss()).toMatch(/@layer aij-base\s*\{[\s\S]*html, body \{ background:/);
    // Declared before styles.css's layers: the head's <style> comes before
    // the entry script that brings the stylesheet in.
    expect(html.indexOf("@layer aij-base")).toBeLessThan(html.indexOf('<script type="module"'));
  });
});
