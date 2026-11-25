/**
 * D-16 dual-theme controller: System / Light / Dark, persisted,
 * applied to <html data-theme> before first paint (no flash).
 */
export type ThemeChoice = "system" | "light" | "dark";
const KEY = "aij.theme";

/**
 * AJ-E1549: each theme's base colour, `--color-canvas` (styles.css). The
 * native window paints it under the page (dark from tauri.conf.json, then
 * the theme's own from here), and so does index.html before any script:
 * a frame the page hasn't drawn shows the page's colour, never WebKit's
 * white. tests/bundle/theme-base.test.ts keeps all four in step with the tokens.
 */
export const BASE_COLOUR = { dark: "#0b0e14", light: "#eceef3" } as const;

/** The colour the native window was last given. */
let windowColour: string | null = null;

/** The window, and its web view, on `colour` (only inside the app). */
function paintWindow(colour: string): void {
  if (windowColour === colour || typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
  windowColour = colour;
  void import("@tauri-apps/api/webviewWindow")
    .then(({ getCurrentWebviewWindow }) => getCurrentWebviewWindow().setBackgroundColor(colour))
    .catch(() => {
      // The page still paints its colour; the window keeps its last.
      windowColour = null;
    });
}

/**
 * 0.5.1 F21: the app setting the backend reads to make the next launch's
 * window in this choice's colour from its first frame (before any page
 * loads: `crates/juggler-app/src/window_frame.rs`, `THEME_KEY`).
 */
export const WINDOW_THEME_KEY = "ui.window.theme";

/** The choice the backend was last told. */
let toldChoice: ThemeChoice | null = null;

/** Tells the backend the choice (only inside the app, once per change). */
function tellWindow(choice: ThemeChoice): void {
  if (toldChoice === choice || typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
  toldChoice = choice;
  void import("./lib/ipc/settings")
    .then(({ appSettingSet }) => appSettingSet(WINDOW_THEME_KEY, choice))
    .catch(() => {
      // The next change or launch tells it again.
      toldChoice = null;
    });
}

export function storedTheme(): ThemeChoice {
  const v = localStorage.getItem(KEY);
  return v === "light" || v === "dark" || v === "system" ? v : "dark";
}

export function applyTheme(choice: ThemeChoice): void {
  const system = window.matchMedia("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
  const effective = choice === "system" ? system : choice;
  document.documentElement.dataset.theme = effective;
  paintWindow(BASE_COLOUR[effective]);
}

/** Fired on every change (⌘⇧L or Settings) so every control agrees. */
export const THEME_EVENT = "aij:theme-changed";

export function setTheme(choice: ThemeChoice): void {
  localStorage.setItem(KEY, choice);
  applyTheme(choice);
  tellWindow(choice);
  window.dispatchEvent(new CustomEvent(THEME_EVENT, { detail: choice }));
}

export function cycleTheme(): ThemeChoice {
  const order: ThemeChoice[] = ["dark", "light", "system"];
  const next = order[(order.indexOf(storedTheme()) + 1) % order.length]!;
  setTheme(next);
  return next;
}

export function initTheme(): void {
  applyTheme(storedTheme());
  // After the first paint: a choice made before 0.5.1 (or in another
  // data folder) reaches the backend too.
  setTimeout(() => tellWindow(storedTheme()), 1500);
  window
    .matchMedia("(prefers-color-scheme: light)")
    .addEventListener("change", () => {
      if (storedTheme() === "system") applyTheme("system");
    });
}
