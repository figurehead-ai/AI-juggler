/**
 * AJ-E3816 — the Watches view's icons, on the app's 24-grid with its 1.75
 * stroke (`base`), drawn as in the approved mock-up: the eye, each
 * condition's mark, each lifetime's, and the row's actions.
 */
import type { JSX, SVGProps } from "react";
import { base } from "../icons";

type P = SVGProps<SVGSVGElement>;
export type WatchIcon = (props: P) => JSX.Element;

function draw(paths: JSX.Element, strokeWidth?: number): WatchIcon {
  return function Icon(props: P): JSX.Element {
    return <svg {...base(strokeWidth ? { strokeWidth, ...props } : props)}>{paths}</svg>;
  };
}

export const IconEye = draw(
  <>
    <path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" />
    <circle cx="12" cy="12" r="2.75" />
  </>,
);
export const IconCheckCircle = draw(
  <>
    <circle cx="12" cy="12" r="9" />
    <path d="m8.5 12.3 2.4 2.4 4.6-4.9" />
  </>,
);
export const IconMessage = draw(<path d="M4 5.5h16v10H9l-5 4v-14Z" />);
export const IconHourglass = draw(<path d="M7 3.5h10M7 20.5h10M8 3.5v3l4 5.5-4 5.5v3M16 3.5v3L12 12l4 5.5v3" />);
export const IconFail = draw(
  <>
    <circle cx="12" cy="12" r="9" />
    <path d="m9 9 6 6M15 9l-6 6" />
  </>,
);
export const IconStopSquare = draw(<rect x="5.5" y="5.5" width="13" height="13" rx="2.5" />);
export const IconMoon = draw(<path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10Z" />);
export const IconSun = draw(
  <>
    <circle cx="12" cy="12" r="3.5" />
    <path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4" />
  </>,
);
export const IconClockFace = draw(
  <>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7.5V12l3 2" />
  </>,
);
export const IconGauge = draw(
  <>
    <path d="M4.5 17a8 8 0 1 1 15 0" />
    <path d="m12 13 4-4" />
    <circle cx="12" cy="13.5" r="1" />
  </>,
);
export const IconTerminal = draw(
  <>
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <path d="m7 9 3 3-3 3M12.5 15H17" />
  </>,
);
export const IconRestart = draw(<path d="M20 11a8 8 0 0 0-14.6-4.5L4 8M4 4v4h4M4 13a8 8 0 0 0 14.6 4.5L20 16M20 20v-4h-4" />);
export const IconPlay = draw(<path d="M8 5.5v13l10.5-6.5L8 5.5Z" />);
export const IconPause = draw(<path d="M9 6v12M15 6v12" />, 2);
export const IconDots = draw(
  <>
    <circle cx="6" cy="12" r="1.2" />
    <circle cx="12" cy="12" r="1.2" />
    <circle cx="18" cy="12" r="1.2" />
  </>,
  2,
);
export const IconPencil = draw(
  <>
    <path d="M4.5 19.5h4l10-10-4-4-10 10v4Z" />
    <path d="m13 7 4 4" />
  </>,
);
export const IconBin = draw(<path d="M4.5 7h15M10 7V5h4v2M6.5 7l1 12.5h9l1-12.5" />);
export const IconAdd = draw(<path d="M12 5v14M5 12h14" />, 2);
export const IconCross = draw(<path d="M6 6l12 12M18 6 6 18" />);
export const IconFind = draw(
  <>
    <circle cx="11" cy="11" r="6.5" />
    <path d="m20 20-4-4" />
  </>,
);
export const IconPersonOutline = draw(
  <>
    <circle cx="12" cy="8" r="3.5" />
    <path d="M5 20c.8-3.6 3.6-5.5 7-5.5s6.2 1.9 7 5.5" />
  </>,
);
export const IconFolderShut = draw(<path d="M3.5 7a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2V7Z" />);
export const IconTiles = draw(
  <>
    <rect x="4" y="4" width="6.5" height="6.5" rx="1.5" />
    <rect x="13.5" y="4" width="6.5" height="6.5" rx="1.5" />
    <rect x="4" y="13.5" width="6.5" height="6.5" rx="1.5" />
    <rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.5" />
  </>,
);
export const IconPoolRing = draw(
  <>
    <circle cx="12" cy="12" r="9" />
    <circle cx="9" cy="10" r="1.75" />
    <circle cx="15" cy="10" r="1.75" />
    <circle cx="12" cy="15.25" r="1.75" />
  </>,
);
export const IconInfoCircle = draw(
  <>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5.5" />
    <path d="M12 7.6v.1" />
  </>,
);
export const IconWarnTriangle = draw(
  <>
    <path d="M12 4 2.8 19.5h18.4L12 4Z" />
    <path d="M12 10v4.5" />
    <path d="M12 17.2v.1" />
  </>,
);
export const IconChevronR = draw(<path d="m9 6 6 6-6 6" />);
export const IconChevronD = draw(<path d="m6 9 6 6 6-6" />);
export const IconChevronL = draw(<path d="m15 6-6 6 6 6" />);
export const IconArrowR = draw(<path d="M5 12h14M13 6l6 6-6 6" />);
export const IconUndoArrow = draw(
  <>
    <path d="M9 14 4 9l5-5" />
    <path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" />
  </>,
);
export const IconInfinity = draw(<path d="M7.5 15.5C4 15.5 3 13 3 12s1-3.5 4.5-3.5c3.5 0 5.5 7 9 7 3.5 0 4.5-2.5 4.5-3.5s-1-3.5-4.5-3.5c-3.5 0-5.5 7-9 7Z" />);
export const IconCopyTwo = draw(
  <>
    <rect x="8" y="8" width="12" height="12" rx="2.5" />
    <path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
  </>,
);
export const IconLines = draw(<path d="M5 7h14M5 12h14M5 17h9" />);
export const IconShieldLine = draw(<path d="M12 3.5 5 6v5.5c0 4.2 3 7.6 7 9 4-1.4 7-4.8 7-9V6l-7-2.5Z" />);
export const IconBell = draw(
  <>
    <path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15L6 16Z" />
    <path d="M10 20.5a2 2 0 0 0 4 0" />
  </>,
);
export const IconCommit = draw(
  <>
    <circle cx="12" cy="12" r="3.5" />
    <path d="M3 12h5.5M15.5 12H21" />
  </>,
);

/** Bot, as its header button draws it (a generic robot head). */
export const IconBotHead = draw(
  <>
    <rect x="4.5" y="8" width="15" height="11.5" rx="3.5" />
    <path d="M12 8V4.8" />
    <circle cx="12" cy="4" r="1" />
    <path d="M9.3 13.2v.6M14.7 13.2v.6" />
    <path d="M2.5 12.5v2.5M21.5 12.5v2.5" />
  </>,
);

/** Each condition's mark. */
export const CONDITION_ICONS: Record<string, WatchIcon> = {
  finishes: IconCheckCircle,
  asks_you: IconMessage,
  stuck: IconHourglass,
  fails: IconFail,
  ends: IconStopSquare,
  idle_for: IconClockFace,
  hits_limit: IconGauge,
  runway_below: IconGauge,
  runway_resets: IconRestart,
  leaves_processes: IconTerminal,
  restarted: IconRestart,
  pool_message: IconMessage,
  task_changes: IconCheckCircle,
  commits: IconCommit,
  starts: IconPlay,
};

/** Each lifetime's mark, on its chip. */
export const LIFETIME_ICONS: Record<string, WatchIcon> = {
  once: IconInfoCircle,
  until_ends: IconHourglass,
  until: IconSun,
  always: IconInfinity,
};
