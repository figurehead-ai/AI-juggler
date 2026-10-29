/**
 * AJ-E2106 — the action icons: one per action, the same wherever the
 * action shows (menus, ⌘K). Drawn like every chrome icon (`icons.tsx`);
 * `ActionIcon.tsx` maps the registry's names to them.
 */
import type { SVGProps } from "react";
import { base } from "./icons";

/** Go to it: out to where the session runs. */
export function IconGoTo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M14 4h6v6M20 4l-9 9" />
      <path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4" />
    </svg>
  );
}

/** Its page here, in AI Juggler. */
export function IconPage(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M7 3h7l5 5v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Z" />
      <path d="M14 3v5h5M9 13h6M9 17h4" />
    </svg>
  );
}

/** Carried into AI Juggler. */
export function IconCarryIn(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m10 17 5-5-5-5M15 12H3" />
      <path d="M15 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4" />
    </svg>
  );
}

export function IconRename(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 20h4L19 9l-4-4L4 16Z" />
      <path d="m14 6 4 4" />
    </svg>
  );
}

/** Snoozed: a moon. */
export function IconSnooze(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M19.5 14.5A8 8 0 1 1 9.5 4.5a6.5 6.5 0 0 0 10 10Z" />
    </svg>
  );
}

export function IconMute(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M8.5 4.7A6 6 0 0 1 18 9.5c0 2.6.5 4.4 1.2 5.5M15 17H4.5S6 15.5 6 9.5c0-.9.2-1.8.6-2.6" />
      <path d="M10 20a2 2 0 0 0 4 0M3.5 3.5l17 17" />
    </svg>
  );
}

/** Back on the list: a bell. */
export function IconBell(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M6 9.5a6 6 0 0 1 12 0c0 4.5 1.5 7.5 1.5 7.5h-15S6 14 6 9.5Z" />
      <path d="M10 20a2 2 0 0 0 4 0" />
    </svg>
  );
}

export function IconCopy(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="8" y="8" width="12" height="12" rx="2" />
      <path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
    </svg>
  );
}

/** Stop: a square in a ring (the stopped state is the square alone). */
export function IconStop(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8.5" />
      <rect x="9" y="9" width="6" height="6" rx="1" />
    </svg>
  );
}

export function IconPlus(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

export function IconEditor(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m9 7-5 5 5 5M15 7l5 5-5 5" />
    </svg>
  );
}

/** Finder: an open folder. */
export function IconFolderOpen(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 19 7.5 11H21l-3.5 8Z" />
      <path d="M4 19V7a2 2 0 0 1 2-2h4l2 2h5a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

/** A project's mark: a tag. */
export function IconTag(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M3.5 12.5v-8a1 1 0 0 1 1-1h8l8 8a1.4 1.4 0 0 1 0 2l-7 7a1.4 1.4 0 0 1-2 0Z" />
      <path d="M8 8h.01" />
    </svg>
  );
}

/** Compare two sides. */
export function IconCompare(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="6" cy="6" r="2.2" />
      <circle cx="18" cy="18" r="2.2" />
      <path d="M6 8.2V15a2 2 0 0 0 2 2h6m-2.5-2.5L14 17l-2.5 2.5" />
      <path d="M18 15.8V9a2 2 0 0 0-2-2h-6m2.5 2.5L10 7l2.5-2.5" />
    </svg>
  );
}

export function IconKey(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="8" cy="15" r="4" />
      <path d="m11 12 8.5-8.5M16.5 6.5l2.5 2.5" />
    </svg>
  );
}

/** Settings: sliders. */
export function IconSliders(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
      <circle cx="15" cy="7" r="2" />
      <circle cx="9" cy="17" r="2" />
    </svg>
  );
}

export function IconStar(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m12 3.8 2.5 5.1 5.6.8-4 4 .9 5.6-5-2.6-5 2.6.9-5.6-4-4 5.6-.8Z" />
    </svg>
  );
}

export function IconReplay(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 12a8 8 0 1 0 2.4-5.7L4 8.5" />
      <path d="M4 4v4.5h4.5" />
    </svg>
  );
}

export function IconArchive(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="4" width="18" height="4" rx="1" />
      <path d="M5 8v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8M10 12h4" />
    </svg>
  );
}

/** Relocate: a map pin. */
export function IconLocate(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M12 21s6.5-5.8 6.5-11a6.5 6.5 0 0 0-13 0c0 5.2 6.5 11 6.5 11Z" />
      <circle cx="12" cy="10" r="2.3" />
    </svg>
  );
}

/** Forget or remove (nothing on disk goes): a minus in a ring. */
export function IconRemove(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M8 12h8" />
    </svg>
  );
}

export function IconTrash(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 7h16M9.5 7V4.5h5V7M6.5 7l.8 11.2a2 2 0 0 0 2 1.8h5.4a2 2 0 0 0 2-1.8L17.5 7M10 11v5M14 11v5" />
    </svg>
  );
}

export function IconInfo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 11v5M12 8h.01" />
    </svg>
  );
}

/** This Mac: a screen. */
export function IconComputer(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="4" width="18" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </svg>
  );
}

/** Cleanup: a broom. */
export function IconBroom(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m20 4-7.5 7.5" />
      <path d="M11 10.5 13.5 13c0 3-2 5.5-5.5 7L4 16c1.5-3.5 4-5.5 7-5.5Z" />
      <path d="m7.5 17.5 2-2" />
    </svg>
  );
}

/** Its pool: sessions that work together, in one ring (AJ-E1702). */
export function IconPool(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="9" />
      <circle cx="9" cy="10" r="1.75" />
      <circle cx="15" cy="10" r="1.75" />
      <circle cx="12" cy="15.25" r="1.75" />
    </svg>
  );
}
