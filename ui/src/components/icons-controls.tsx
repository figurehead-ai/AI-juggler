/**
 * 0.3 — the shared controls' own icons (a picker's chevron, a choice's
 * check, a disclosure's arrow) and the View menu's Show lines, drawn on
 * the same 24-grid stroke as the rest of the set.
 */
import type { SVGProps } from "react";
import { base } from "./icons";

/** A picker's chevron: it opens a menu below. */
export function IconChevronDown(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m7 10 5 5 5-5" />
    </svg>
  );
}

/** A line that opens more (a submenu, a disclosure that is closed). */
export function IconChevronRight(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m10 7 5 5-5 5" />
    </svg>
  );
}

/** The chosen one, in a menu of choices. */
export function IconCheck(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m5 12.5 4.5 4.5L19 7.5" />
    </svg>
  );
}

/** Inactive: in an app's list, not running (a paused circle). */
export function IconInactive(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8" />
      <path d="M10 9.5v5M14 9.5v5" />
    </svg>
  );
}

/** Quiet ones: idle, folded away (a crescent moon). */
export function IconQuiet(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M19 14.5A7.5 7.5 0 0 1 9.5 5a7.5 7.5 0 1 0 9.5 9.5Z" />
    </svg>
  );
}

/** Ended today: a circle with a check. */
export function IconEnded(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8" />
      <path d="m8.5 12.2 2.4 2.4 4.6-4.8" />
    </svg>
  );
}

/** Background runs: a window behind another (headless, from scripts). */
export function IconBackground(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="8" y="8" width="12" height="11" rx="2" />
      <path d="M16 5.5V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v9a1 1 0 0 0 1 1h.5" />
    </svg>
  );
}
