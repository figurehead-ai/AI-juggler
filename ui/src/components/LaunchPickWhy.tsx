/**
 * New session's pick, explained (the 0.2 walk, F17: the pick wasn't the
 * one marked Suggested, and nothing said why). Beside the pick: why it's
 * the one ("your default", "used last here", "Suggested: the most room").
 * Under it, when it isn't the Suggested one: that one with its room and
 * "Use it" (the spec's suggest-only line, deep/11 §4.8); and a pick of
 * yours passed over for being out of room, with until when.
 */
import type { JSX } from "react";
import type { QuotaDisplayData } from "../lib/bridge";
import { launchRoom, pickWhyWords, type LaunchPick } from "../lib/launch-pick";
import { outOfRoomWords } from "../lib/room";

export function LaunchPickWhy({
  pick,
  quota,
  onUse,
}: {
  pick: LaunchPick;
  quota: Record<string, QuotaDisplayData>;
  /** Makes this account the pick. */
  onUse: (accountId: string) => void;
}): JSX.Element {
  const why = pickWhyWords(pick.why);
  const other = pick.suggested && pick.suggested.account_id !== pick.recipe.account_id ? pick.suggested : null;
  const otherLeft = other ? launchRoom(quota[other.account_id]).left : null;
  const passed = pick.passedOver;
  const passedWhy = passed ? pickWhyWords(passed.why) : null;
  return (
    <>
      {why && (
        <span className="ml-auto text-label text-muted" data-testid="launch-default" data-why={pick.why}>
          {why}
        </span>
      )}
      {passed && (
        <span className="basis-full text-label text-warning" data-testid="launch-passed-over">
          {passed.recipe.account_label}
          {passedWhy ? ` (${passedWhy})` : ""}: {outOfRoomWords(launchRoom(quota[passed.recipe.account_id]).resetsAtMs)}. A session there would stop at its
          limit at once.{" "}
          <button
            type="button"
            onClick={() => onUse(passed.recipe.account_id)}
            className="rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            Use {passed.recipe.account_label} anyway
          </button>
        </span>
      )}
      {other && (
        <span className="basis-full text-label text-muted" data-testid="launch-suggested">
          Suggested: {other.account_label} has the most room{otherLeft !== null ? ` (${otherLeft}% left)` : ""}.{" "}
          <button
            type="button"
            onClick={() => onUse(other.account_id)}
            className="rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
            data-testid="launch-use-suggested"
          >
            Use {other.account_label}
          </button>
        </span>
      )}
    </>
  );
}
