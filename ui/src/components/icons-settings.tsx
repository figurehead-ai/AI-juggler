/**
 * AJ-E1558 (D-126 rich mode, ICN-1): the icons Settings' and the edit
 * sheets' rows lead with, each one meaning only. Drawn like the rest of
 * the set (icons.tsx: a 24 grid, 1.75 stroke, round caps and joins).
 */
import type { SVGProps } from "react";
import { base } from "./icons";

/** The light theme: a sun. */
export function IconThemeLight(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" />
    </svg>
  );
}

/** The dark theme: a moon with a star. */
export function IconThemeDark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z" />
      <path d="M17 3.5v3M15.5 5h3" />
    </svg>
  );
}

/** The theme follows the system's: a screen, half light and half dark. */
export function IconThemeSystem(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="4" width="18" height="13" rx="2" />
      <path d="M12 4v13" />
      <path d="M12 6.5h6.5M12 9.5h6.5M12 12.5h6.5" strokeWidth={1.25} />
      <path d="M9 21h6M12 17v4" />
    </svg>
  );
}

/** Two lines per row (Comfortable). */
export function IconRowsTwo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="4" width="18" height="7" rx="1.5" />
      <rect x="3" y="13" width="18" height="7" rx="1.5" />
      <path d="M6 7h8M6 16h8" />
    </svg>
  );
}

/** One line per row (Compact). */
export function IconRowsOne(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 5h16M4 10h16M4 15h16M4 20h16" />
    </svg>
  );
}

/** Icons only (Minimal). */
export function IconRowsIcons(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="4" y="3.5" width="6" height="5" rx="1.5" />
      <rect x="4" y="10" width="6" height="5" rx="1.5" />
      <rect x="4" y="16.5" width="6" height="4" rx="1.5" />
      <path d="M14 4v16" />
    </svg>
  );
}

/** Open at login: a power symbol. */
export function IconPower(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M12 3v8" />
      <path d="M7 6.3a7.5 7.5 0 1 0 10 0" />
    </svg>
  );
}

/** Start again after a crash: a circle arrow. */
export function IconRestart(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M20 12a8 8 0 1 1-2.4-5.7" />
      <path d="M20 4v4.5h-4.5" />
    </svg>
  );
}

/** Keep the Mac awake: a cup. */
export function IconAwake(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 9h12v5a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5Z" />
      <path d="M16 10.5h1.5a2.5 2.5 0 0 1 0 5H16" />
      <path d="M8 3.5c-.6.8-.6 1.7 0 2.5M12 3.5c-.6.8-.6 1.7 0 2.5" />
    </svg>
  );
}

/** On battery. */
export function IconBattery(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="2.5" y="7" width="17" height="10" rx="2" />
      <path d="M21.5 10.5v3" />
      <path d="M6 10v4M9 10v4" />
    </svg>
  );
}

/** A text size. */
export function IconTextSize(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M3 18 8 6l5 12M4.6 14h6.8" />
      <path d="M14 18l3.5-8 3.5 8M15 15.5h5" />
    </svg>
  );
}

/** Lines kept above the screen (scrollback). */
export function IconScrollback(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="4" y="3" width="16" height="18" rx="2" />
      <path d="M8 7h8M8 10.5h8M8 14h5" />
      <path d="M17 17.5v-4M15.5 15 17 13.5l1.5 1.5" />
    </svg>
  );
}

/** The tools an agent may use: a wrench. */
export function IconTools(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M14.5 5.5a4 4 0 0 0 4.9 4.9L20 11l-9 9a2.1 2.1 0 0 1-3-3l9-9 .6.6a4 4 0 0 0-4.9-4.9l2.3 2.3-1.5 1.5Z" />
    </svg>
  );
}

/** Money paid: a coin. */
export function IconCoin(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M14.5 9.2c-.5-.8-1.4-1.2-2.5-1.2-1.5 0-2.5.8-2.5 1.9 0 2.6 5 1.4 5 4.1 0 1.1-1 2-2.5 2-1.2 0-2.1-.5-2.6-1.3M12 6.5V8M12 16v1.5" />
    </svg>
  );
}

/** Still in beta: a flask. */
export function IconFlask(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M9.5 3h5M10 3v6l-5 9a2 2 0 0 0 1.8 3h10.4a2 2 0 0 0 1.8-3l-5-9V3" />
      <path d="M7.2 15h9.6" />
    </svg>
  );
}

/** The debug log: a bug. */
export function IconBug(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="7" y="7" width="10" height="13" rx="5" />
      <path d="M9.5 7.5a2.5 2.5 0 0 1 5 0M12 11v9M7 12H3.5M20.5 12H17M7.5 16.5 4.5 18.5M16.5 16.5l3 2M7.5 8.5 5 6M16.5 8.5 19 6" />
    </svg>
  );
}

/** What is kept on this computer (a cache, stored copies): a stack of disks. */
export function IconStorage(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <ellipse cx="12" cy="5.5" rx="7.5" ry="2.5" />
      <path d="M4.5 5.5v6.5c0 1.4 3.4 2.5 7.5 2.5s7.5-1.1 7.5-2.5V5.5" />
      <path d="M4.5 12v6.5c0 1.4 3.4 2.5 7.5 2.5s7.5-1.1 7.5-2.5V12" />
    </svg>
  );
}
