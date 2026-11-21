/**
 * 0.4.1: the one "Out" mark (`lib/room-mark`), drawn as the session
 * header's state chip is: a rounded chip, its tone's border and wash, an
 * icon and the words. Out is the danger tone with an empty gauge; low the
 * calmer amber with a low one. `size="sm"` for rows and cards, `md` for a
 * page's header; `compact` where a line is tight (the rail's App header):
 * "Out", the reset in its title. Its title says the whole of it.
 */
import type { JSX } from "react";
import { IconGaugeEmpty, IconGaugeLow } from "./icons";
import type { RoomMark } from "../lib/room-mark";
import { READINGS_EVERY_MS, useQuotaDisplays } from "../lib/kept-reads";
import { useSubscriptions } from "../lib/subscriptions";
import { accountMark, roomCountWords, roomCounts, roomMarkOfApp } from "../lib/room-mark";
import type { SubscriptionInfo } from "../lib/subscriptions";
import { clocksWhole } from "../lib/time-words";
import { openAccounts } from "../lib/accounts-filter";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const TONE: Record<RoomMark["kind"], string> = {
  out: "border-danger/30 bg-danger-soft text-danger",
  low: "border-warning/35 bg-warning-soft text-warning",
};

export function RoomChip({ mark, size = "sm", compact = false, iconOnly = false, testId = "room-chip" }: { mark: RoomMark | null; size?: "sm" | "md"; compact?: boolean; iconOnly?: boolean; testId?: string }): JSX.Element | null {
  if (!mark) return null;
  const Icon = mark.kind === "out" ? IconGaugeEmpty : IconGaugeLow;
  // D-100 (the 0.4.2 review): in the rail's App header the mark is its
  // icon alone (its words on hover), so the App's name keeps the room.
  // D-118 (one quiet mark per need, the rail calm): a glyph, no amber or
  // red disc: low room is a note (muted), Out its danger glyph; the loud
  // copy of a need is its Needs you row.
  if (iconOnly) {
    return (
      <span title={mark.title} data-testid={testId} data-kind={mark.kind} className={`inline-flex size-[18px] shrink-0 items-center justify-center ${mark.kind === "out" ? "text-danger" : "text-muted"}`}>
        <Icon aria-hidden="true" width={11} height={11} />
        <span className="sr-only">{mark.title}</span>
      </span>
    );
  }
  // 0.4.5 (the maintainer's 0.4.4 finding: "Out · resets 6:00 AM
  // tomorrow" ran past Home's narrowest app card): never wider than its
  // container; words that don't fit go on to a second line inside it,
  // the icon beside the first, a clock never parted from its AM. One
  // line is drawn exactly as before: the icon sits in a line-tall box,
  // centred as it was, and each size's corner is half its one-line
  // height (line, padding and border: md 26, sm 20, compact 18 px), what
  // `rounded-full` came to; two lines are a rounded box, not a capsule
  // whose ends crowd the icon.
  const box =
    size === "md"
      ? "gap-1.5 px-2.5 py-0.5 text-meta leading-5 rounded-[13px]"
      : compact
        ? "gap-0.5 px-1 text-label leading-4 rounded-[9px]"
        : "gap-1 px-1.5 py-px text-label leading-4 rounded-[10px]";
  return (
    // Screen readers hear its title as text (an aria-label on a plain
    // span is prohibited: axe's aria-prohibited-attr); sighted users see
    // the short word and get the title on hover.
    <span
      title={mark.title}
      data-testid={testId}
      data-kind={mark.kind}
      className={`inline-flex max-w-full shrink-0 items-start border font-semibold ${box} ${TONE[mark.kind]}`}
    >
      {/* Compact (the rail's App header) is the red word alone: its icon
          took the room the App's name needs ("Work M…", the 0.4.1 gate). */}
      {!compact && (
        <span className={`flex shrink-0 items-center ${size === "md" ? "h-5" : "h-4"}`}>
          <Icon aria-hidden="true" width={size === "md" ? 14 : 12} height={size === "md" ? 14 : 12} />
        </span>
      )}
      <span aria-hidden="true" className="min-w-0 break-words">
        {compact ? mark.short : clocksWhole(mark.label)}
      </span>
      <span className="sr-only">{mark.title}</span>
    </span>
  );
}

/** The mark for folder `profileId`, from the shared reads every screen
 * uses (the readings kept for half a minute, the Accounts). F-rc3c-3:
 * the Accounts are read as often as the readings (a mark stays drawn in
 * the rail; read once, a folder that moved Accounts was still counted in
 * the one it left, its new reading marking that one Out). */
export function useRoomMark(profileId: string | null | undefined, sessions: readonly InstanceInfo[] = [], appId: string | null = null): RoomMark | null {
  const { displays } = useQuotaDisplays();
  const accounts = useSubscriptions(0, !!profileId || !!appId, READINGS_EVERY_MS);
  // AJ-E3210: an App's mark is its own Account's (a Claude app profile
  // and its CLI on one folder can be on two).
  return roomMarkOfApp(appId, profileId, accounts, displays, sessions);
}

/** The chip for folder `profileId`, read where it's drawn; `appId`: the
 * App it marks (AJ-E3210: by that App's own Account). */
export function ProfileRoomChip({ profileId, appId = null, size, compact, testId }: { profileId: string | null | undefined; appId?: string | null; size?: "sm" | "md"; compact?: boolean; testId?: string }): JSX.Element | null {
  return <RoomChip mark={useRoomMark(profileId, [], appId)} size={size} compact={compact} testId={testId} />;
}

/** An Account's own mark (its page's header): by its readings, wherever
 * they were read (AJ-E3210: an Account only a Claude app profile is on
 * has no folder of its own). */
export function useAccountRoomMark(account: SubscriptionInfo): RoomMark | null {
  const { displays } = useQuotaDisplays();
  return accountMark(account, displays);
}

/** D-87, 0.4.1: the top bar's calm count of Accounts out of room or low
 * ("1 account out"), with the gauge, beside This Mac's line; a click
 * opens Accounts on the out ones (0.4.2), or the low ones when none is
 * out. Nothing when every account has room. D-117 (c2) and D-118's mock:
 * chrome spends no colour on a note: a ghost button, no fill or border
 * (no amber fill anywhere, budget rule 2); low room is a note (L1) in
 * muted words, Out keeps the danger word. Runway by account and Needs
 * you carry the loud copy when live work is on it. */
export function RoomCount(): JSX.Element | null {
  const { displays } = useQuotaDisplays();
  const accounts = useSubscriptions(0, true, READINGS_EVERY_MS);
  const counts = roomCounts(accounts, displays);
  const words = roomCountWords(counts);
  if (!words) return null;
  const out = counts.out > 0;
  const Icon = out ? IconGaugeEmpty : IconGaugeLow;
  return (
    <button
      type="button"
      // 0.4.2: it lands on what it says: the out Accounts (by the filter's
      // id, lib/accounts-filter); only low ones: Accounts, low ones marked.
      onClick={() => openAccounts(out ? "out" : "low")}
      title={`${words}: show ${counts[out ? "out" : "low"] === 1 ? "it" : "them"} in Accounts`}
      data-testid="header-room-count"
      data-kind={out ? "out" : "low"}
      className={`btn-ghost inline-flex h-7 shrink-0 items-center gap-1 whitespace-nowrap rounded-full px-1.5 text-meta font-medium outline-none focus-visible:ring-2 focus-visible:ring-accent/60 ${out ? "text-danger" : "text-muted"}`}
    >
      <Icon aria-hidden="true" width={13} height={13} />
      {/* Its number alone until the bar has room for its words (the
          header fits one row at 1024 and 1280 with the list open). */}
      <span aria-hidden="true" className="tabular-nums @min-[1180px]/header:hidden">{counts.out > 0 ? counts.out : counts.low}</span>
      <span className="sr-only @min-[1180px]/header:not-sr-only">{words}</span>
    </button>
  );
}
