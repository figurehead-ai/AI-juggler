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
import { useQuotaDisplays } from "../lib/kept-reads";
import { useSubscriptions } from "../lib/subscriptions";
import { roomCountWords, roomCounts, roomMarkOfProfile } from "../lib/room-mark";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const TONE: Record<RoomMark["kind"], string> = {
  out: "border-danger/30 bg-danger-soft text-danger",
  low: "border-warning/35 bg-warning-soft text-warning",
};

export function RoomChip({ mark, size = "sm", compact = false, testId = "room-chip" }: { mark: RoomMark | null; size?: "sm" | "md"; compact?: boolean; testId?: string }): JSX.Element | null {
  if (!mark) return null;
  const Icon = mark.kind === "out" ? IconGaugeEmpty : IconGaugeLow;
  const box = size === "md" ? "gap-1.5 px-2.5 py-0.5 text-meta leading-5" : compact ? "gap-0.5 px-1 text-label leading-4" : "gap-1 px-1.5 py-px text-label leading-4";
  return (
    // Screen readers hear its title as text (an aria-label on a plain
    // span is prohibited: axe's aria-prohibited-attr); sighted users see
    // the short word and get the title on hover.
    <span
      title={mark.title}
      data-testid={testId}
      data-kind={mark.kind}
      className={`inline-flex shrink-0 items-center whitespace-nowrap rounded-full border font-semibold ${box} ${TONE[mark.kind]}`}
    >
      <Icon aria-hidden="true" width={size === "md" ? 14 : compact ? 11 : 12} height={size === "md" ? 14 : compact ? 11 : 12} />
      <span aria-hidden="true">{compact ? mark.short : mark.label}</span>
      <span className="sr-only">{mark.title}</span>
    </span>
  );
}

/** The mark for folder `profileId`, from the shared reads every screen
 * uses (the readings kept for half a minute, the Accounts). */
export function useRoomMark(profileId: string | null | undefined, sessions: readonly InstanceInfo[] = []): RoomMark | null {
  const { displays } = useQuotaDisplays();
  const accounts = useSubscriptions(0, !!profileId);
  return roomMarkOfProfile(profileId, accounts, displays, sessions);
}

/** The chip for folder `profileId`, read where it's drawn. */
export function ProfileRoomChip({ profileId, size, compact, testId }: { profileId: string | null | undefined; size?: "sm" | "md"; compact?: boolean; testId?: string }): JSX.Element | null {
  return <RoomChip mark={useRoomMark(profileId)} size={size} compact={compact} testId={testId} />;
}

/** D-87, 0.4.1: the top bar's calm count of Accounts out of room or low
 * ("1 account out"), with the gauge, beside This Mac's line; a click
 * opens Accounts. Nothing when every account has room. */
export function RoomCount(): JSX.Element | null {
  const { displays } = useQuotaDisplays();
  const accounts = useSubscriptions();
  const counts = roomCounts(accounts, displays);
  const words = roomCountWords(counts);
  if (!words) return null;
  const out = counts.out > 0;
  const Icon = out ? IconGaugeEmpty : IconGaugeLow;
  return (
    <button
      type="button"
      onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "accounts" } }))}
      title={`${words}: open Accounts`}
      data-testid="header-room-count"
      data-kind={out ? "out" : "low"}
      className={`inline-flex h-7 shrink-0 items-center gap-1 whitespace-nowrap rounded-full border px-2 text-label font-semibold outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 ${TONE[out ? "out" : "low"]}`}
    >
      <Icon aria-hidden="true" width={13} height={13} />
      {/* Its number alone until the bar has room for its words (the
          header fits one row at 1024 and 1280 with the list open). */}
      <span aria-hidden="true" className="tabular-nums @min-[1180px]/header:hidden">{counts.out > 0 ? counts.out : counts.low}</span>
      <span className="sr-only @min-[1180px]/header:not-sr-only">{words}</span>
    </button>
  );
}
