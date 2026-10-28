/**
 * The update channel's one word (AJ-E1010 Req.3; D-04/D-11), the same in
 * About, every update prompt and your menu. Moved out of UpdatesCard.tsx
 * (its size budget).
 */
import type { JSX } from "react";
import type { UpdateStatusInfo } from "../lib/bridge";

/** The channel's name as the update prompt shows it (D-04/D-11: the
 * beta channel is always labelled "Beta"). */
export function channelLabel(channel: string): string {
  return channel === "stable" ? "Stable" : channel === "nightly" ? "Nightly" : channel === "dev" ? "Dev" : "Beta";
}

/** What this build is on, one word everywhere (About and your menu; the
 * 0.2 walk, F18, found "Beta" in one and "Nightly" in the other): the
 * Nightly and Dev editions are their own, else the channel followed. */
export function shownChannel(status: Pick<UpdateStatusInfo, "edition" | "channel">): string {
  return status.edition === "nightly" || status.edition === "dev" ? status.edition : status.channel;
}

/** The channel chip: the same look in About and in every update prompt. */
export function ChannelChip({ channel, testId }: { channel: string; testId?: string }): JSX.Element {
  return (
    <span
      className="rounded-full border border-warning/40 bg-warning-soft px-2 py-0.5 text-label font-medium text-ink"
      data-testid={testId}
    >
      {channelLabel(channel)}
    </span>
  );
}
