/**
 * AJ-E3812 — the Models screen's own icons, drawn like the rest (24 grid,
 * 1.75 stroke, `base`): a chip for Apple's model and the AI pane, a
 * download, a lock ("Stays on this Mac"), a disk, offline, and one per
 * AI job. No vendor marks (models have no installed app icon).
 */
import type { JSX, SVGProps } from "react";
import { base } from "../../../components/icons";
import type { ModelJob } from "../../../types/generated/ModelJob";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

const draw = (paths: JSX.Element): Icon =>
  function Drawn(props) {
    return <svg {...base(props)}>{paths}</svg>;
  };

export const IconChip = draw(
  <>
    <rect x="6.5" y="6.5" width="11" height="11" rx="2.2" />
    <rect x="9.5" y="9.5" width="5" height="5" rx="1" />
    <path d="M9.5 3.5v3M14.5 3.5v3M9.5 17.5v3M14.5 17.5v3M3.5 9.5h3M3.5 14.5h3M17.5 9.5h3M17.5 14.5h3" />
  </>,
);

export const IconDownload = draw(
  <>
    <path d="M12 4v11M7 10.5l5 5 5-5" />
    <path d="M5 19.5h14" />
  </>,
);

export const IconLock = draw(
  <>
    <rect x="4.5" y="10.5" width="15" height="10" rx="2" />
    <path d="M8 10.5V7.5a4 4 0 0 1 8 0v3" />
  </>,
);

export const IconDisk = draw(
  <>
    <rect x="3" y="6" width="18" height="12" rx="2.5" />
    <path d="M7 14.5h.01M10.5 14.5h.01" />
    <path d="M3 11h18" />
  </>,
);

export const IconOffline = draw(
  <>
    <path d="M3 3l18 18" />
    <path d="M8.5 16.5a5 5 0 0 1 7 0M5 13a10 10 0 0 1 5-2.6M14.5 10.6A10 10 0 0 1 19 13M2 9.5a15 15 0 0 1 4.3-2.7M10.7 6a15 15 0 0 1 11.3 3.5" />
    <path d="M12 20h.01" />
  </>,
);

export const IconRefresh = draw(<path d="M20 11a8 8 0 0 0-14.6-4.5L4 8M4 4v4h4M4 13a8 8 0 0 0 14.6 4.5L20 16M20 20v-4h-4" />);

/** AJ-E3826 (M1): a downloaded model, a box that stays on this Mac. */
const IconModelBox = draw(
  <>
    <path d="M12 3.5 19.5 7.5v9L12 20.5 4.5 16.5v-9Z" />
    <path d="M4.5 7.5 12 11.5l7.5-4M12 11.5v9" />
  </>,
);

/** Automatic: the recommendation picks for you. */
export const IconAuto = draw(
  <>
    <path d="M10 4l1.9 4.6L16.5 10.5l-4.6 1.9L10 17l-1.9-4.6L3.5 10.5l4.6-1.9Z" />
    <path d="M18 14.5l.8 1.7 1.7.8-1.7.8-.8 1.7-.8-1.7-1.7-.8 1.7-.8Z" />
  </>,
);

/** Today's use: a pulse line. */
export const IconPulse = draw(<path d="M3 12h4l2.5-6 5 12 2.5-6H21" />);

/** A model's icon by its kind (M1): Apple's chip, a downloaded model's
 * box. The picker's options and its field carry it. */
export function modelIcon(model: { built_in: boolean }): Icon {
  return model.built_in ? IconChip : IconModelBox;
}

const IconReview = draw(
  <>
    <rect x="4.5" y="3.5" width="15" height="17" rx="2" />
    <path d="m8 9 1.6 1.6L12.5 7.7M8 15h8" />
  </>,
);
const IconAnswer = draw(<path d="M4 5.5h16v10H9l-5 4v-14Z" />);
const IconFlag = draw(<path d="M5 21V4.5h11l-1.8 4 1.8 4H5" />);
const IconLines = draw(<path d="M4 6.5h16M4 12h11M4 17.5h13" />);
const IconDigest = draw(
  <>
    <rect x="4" y="4" width="16" height="16" rx="3" />
    <path d="M8 9h8M8 12.5h8M8 16h5" />
  </>,
);
const IconEye = draw(
  <>
    <path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" />
    <circle cx="12" cy="12" r="2.75" />
  </>,
);

const JOB_ICONS: Record<ModelJob, Icon> = {
  review: IconReview,
  chat: IconAnswer,
  attention: IconFlag,
  feed: IconLines,
  digest: IconDigest,
  watches: IconEye,
};

/** A job's icon as a component (the kit's RowChip draws it). */
export function jobIconOf(job: ModelJob): Icon {
  return JOB_ICONS[job];
}

/** A job's icon, decoration only (its name is beside it). */
export function JobIcon({ job, size = 15 }: { job: ModelJob; size?: number }): JSX.Element {
  const Icon = JOB_ICONS[job];
  return <Icon width={size} height={size} aria-hidden="true" />;
}
