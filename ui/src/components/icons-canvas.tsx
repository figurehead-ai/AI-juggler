import type { SVGProps } from "react";
import { base } from "./icons";

/**
 * 0.3: the pool canvas's icons, drawn like the rest of the set (24-grid,
 * 1.75 stroke, round caps).
 */

/** A thing's menu: three dots in a row. */
export function IconMore(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="5.5" cy="12" r="1.1" fill="currentColor" />
      <circle cx="12" cy="12" r="1.1" fill="currentColor" />
      <circle cx="18.5" cy="12" r="1.1" fill="currentColor" />
    </svg>
  );
}

/** Fit to screen: four corners closing in. */
export function IconFit(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 9V5a1 1 0 0 1 1-1h4" />
      <path d="M15 4h4a1 1 0 0 1 1 1v4" />
      <path d="M20 15v4a1 1 0 0 1-1 1h-4" />
      <path d="M9 20H5a1 1 0 0 1-1-1v-4" />
      <rect x="9" y="9" width="6" height="6" rx="1" />
    </svg>
  );
}

export function IconZoomIn(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="11" cy="11" r="6.5" />
      <path d="m20 20-4.2-4.2" />
      <path d="M11 8.5v5M8.5 11h5" />
    </svg>
  );
}

export function IconZoomOut(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="11" cy="11" r="6.5" />
      <path d="m20 20-4.2-4.2" />
      <path d="M8.5 11h5" />
    </svg>
  );
}

/** Tidy up: boxes falling into rows. */
export function IconTidy(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="9" y="3.5" width="6" height="4.5" rx="1" />
      <rect x="3.5" y="15.5" width="6" height="4.5" rx="1" />
      <rect x="14.5" y="15.5" width="6" height="4.5" rx="1" />
      <path d="M12 8v3.5M6.5 15.5v-2a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

/** View as list. */
export function IconList(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M9 6h11M9 12h11M9 18h11" />
      <circle cx="4.5" cy="6" r="0.9" fill="currentColor" />
      <circle cx="4.5" cy="12" r="0.9" fill="currentColor" />
      <circle cx="4.5" cy="18" r="0.9" fill="currentColor" />
    </svg>
  );
}

/** The canvas: nodes joined by lines. */
export function IconCanvas(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="9" y="3.5" width="6" height="5" rx="1.2" />
      <rect x="3" y="15.5" width="6" height="5" rx="1.2" />
      <rect x="15" y="15.5" width="6" height="5" rx="1.2" />
      <path d="M11 8.5 6.5 15.5M13 8.5l4.5 7" />
    </svg>
  );
}

/** Tasks: a checklist. */
export function IconTasks(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m3.5 6.5 1.5 1.5 3-3" />
      <path d="m3.5 13.5 1.5 1.5 3-3" />
      <path d="M11 7h9.5M11 14h9.5M11 20h9.5" />
    </svg>
  );
}

/** Messages: a speech bubble. */
export function IconMessage(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M20 12.5a7.5 7.5 0 0 1-11 6.6L4 20l1-4.2A7.5 7.5 0 1 1 20 12.5Z" />
    </svg>
  );
}

/** The lead: a small crown. */
export function IconLead(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 17.5 3 7.5l5 4 4-6 4 6 5-4-1 10Z" />
      <path d="M5 20.5h14" />
    </svg>
  );
}
