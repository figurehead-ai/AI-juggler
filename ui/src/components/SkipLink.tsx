/**
 * deep/11 §9: the shell's first stop is a skip link. The 0.2 walk (F11)
 * found none, and 84 Tabs to get past the rail. Hidden until it has
 * focus; Enter puts the keyboard on the page (`<main>`), past the rail
 * and the header.
 */
import type { JSX } from "react";
import { focusPage } from "../lib/nav";

export function SkipLink(): JSX.Element {
  return (
    <button
      type="button"
      data-testid="skip-to-page"
      onClick={focusPage}
      className="sr-only rounded-lg border border-accent/50 bg-surface px-3 py-2 text-meta font-medium text-ink shadow-card outline-none focus:not-sr-only focus:absolute focus:left-3 focus:top-3 focus:z-[60] focus-visible:ring-2 focus-visible:ring-accent/60"
    >
      Skip to page
    </button>
  );
}
