/**
 * D-16 dual-theme controller: System / Light / Dark, persisted,
 * applied to <html data-theme> before first paint (no flash).
 */
export type ThemeChoice = "system" | "light" | "dark";
const KEY = "aij.theme";

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
}

export function setTheme(choice: ThemeChoice): void {
  localStorage.setItem(KEY, choice);
  applyTheme(choice);
}

export function cycleTheme(): ThemeChoice {
  const order: ThemeChoice[] = ["dark", "light", "system"];
  const next = order[(order.indexOf(storedTheme()) + 1) % order.length];
  setTheme(next);
  return next;
}

export function initTheme(): void {
  applyTheme(storedTheme());
  window
    .matchMedia("(prefers-color-scheme: light)")
    .addEventListener("change", () => {
      if (storedTheme() === "system") applyTheme("system");
    });
}
