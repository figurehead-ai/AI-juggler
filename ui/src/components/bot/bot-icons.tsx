/**
 * D-136 — Bot's own icons, on the app's 24-grid with its 1.75 stroke
 * (`base`). The robot head is generic, never a vendor's mark.
 */
import type { JSX, SVGProps } from "react";
import { base } from "../icons";

type P = SVGProps<SVGSVGElement>;

export function IconBot(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <rect x="4.5" y="8" width="15" height="11.5" rx="3.5" />
      <path d="M12 8V4.8" />
      <circle cx="12" cy="4" r="1" />
      <path d="M9.3 13.2v.6M14.7 13.2v.6" />
      <path d="M2.5 12.5v2.5M21.5 12.5v2.5" />
    </svg>
  );
}

export function IconExpand(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M14 4.5h5.5V10M10 19.5H4.5V14M19.5 4.5 13.5 10.5M4.5 19.5l6-6" />
    </svg>
  );
}

export function IconCollapse(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M19.5 10H14V4.5M4.5 14H10v5.5M14 10l6-6M10 14l-6 6" />
    </svg>
  );
}

export function IconSparkle(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M12 4l1.8 5.2L19 11l-5.2 1.8L12 18l-1.8-5.2L5 11l5.2-1.8L12 4Z" />
    </svg>
  );
}

export function IconBookmark(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M7 4.5h10a1 1 0 0 1 1 1v14l-6-4-6 4v-14a1 1 0 0 1 1-1Z" />
    </svg>
  );
}

export function IconLock(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <rect x="5" y="10.5" width="14" height="9.5" rx="2" />
      <path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5" />
    </svg>
  );
}

export function IconUp(props: P): JSX.Element {
  return (
    <svg {...base({ strokeWidth: 2, ...props })}>
      <path d="M12 19V5M6 11l6-6 6 6" />
    </svg>
  );
}

export function IconSearch(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <circle cx="11" cy="11" r="6.5" />
      <path d="m20 20-4-4" />
    </svg>
  );
}

export function IconEye(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" />
      <circle cx="12" cy="12" r="2.75" />
    </svg>
  );
}

export function IconTaskBox(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <rect x="4" y="4" width="16" height="16" rx="3" />
      <path d="m8.5 12.3 2.4 2.4 4.6-4.9" />
    </svg>
  );
}

export function IconPerson(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="8" r="3.5" />
      <path d="M5 20c.8-3.6 3.6-5.5 7-5.5s6.2 1.9 7 5.5" />
    </svg>
  );
}

export function IconCheckCircle(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="9" />
      <path d="m8.5 12.3 2.4 2.4 4.6-4.9" />
    </svg>
  );
}

export function IconMessage(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M4 5.5h16v10H9l-5 4v-14Z" />
    </svg>
  );
}

export function IconHourglass(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M7 3.5h10M7 20.5h10M8 3.5c0 4 4 5 4 8.5s-4 4.5-4 8.5M16 3.5c0 4-4 5-4 8.5s4 4.5 4 8.5" />
    </svg>
  );
}

export function IconFail(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="9" />
      <path d="m9 9 6 6M15 9l-6 6" />
    </svg>
  );
}

export function IconMoon(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10Z" />
    </svg>
  );
}

export function IconFlag(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M5.5 20.5V4.5M5.5 4.5h11l-2 4 2 4h-11" />
    </svg>
  );
}

export function IconChip(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <rect x="6.5" y="6.5" width="11" height="11" rx="2" />
      <path d="M9.5 3.5v3M14.5 3.5v3M9.5 17.5v3M14.5 17.5v3M3.5 9.5h3M3.5 14.5h3M17.5 9.5h3M17.5 14.5h3" />
    </svg>
  );
}

export function IconEdit(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M4.5 19.5h4l10-10-4-4-10 10v4Z" />
      <path d="m13 7 4 4" />
    </svg>
  );
}

export function IconUndo(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M9 14 4 9l5-5" />
      <path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" />
    </svg>
  );
}

export function IconGauge(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M4.5 17a8 8 0 1 1 15 0" />
      <path d="m12 13 4-4" />
      <circle cx="12" cy="13.5" r="1" />
    </svg>
  );
}

export function IconDownload(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M12 4v11M7 10.5l5 5 5-5M5 20h14" />
    </svg>
  );
}

export function IconQuestion(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="9" />
      <path d="M9.6 9.3a2.5 2.5 0 0 1 4.8.9c0 1.7-2.4 2.1-2.4 3.6" />
      <path d="M12 16.8v.1" />
    </svg>
  );
}

export function IconLines(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M4 6.5h16M4 12h11M4 17.5h13" />
    </svg>
  );
}

export function IconFolder(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M3.5 7a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2V7Z" />
    </svg>
  );
}

export function IconArrowRight(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <path d="M5 12h14M13 6l6 6-6 6" />
    </svg>
  );
}

export function IconGrid(props: P): JSX.Element {
  return (
    <svg {...base(props)}>
      <rect x="4" y="4" width="6.5" height="6.5" rx="1.5" />
      <rect x="13.5" y="4" width="6.5" height="6.5" rx="1.5" />
      <rect x="4" y="13.5" width="6.5" height="6.5" rx="1.5" />
      <rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.5" />
    </svg>
  );
}
