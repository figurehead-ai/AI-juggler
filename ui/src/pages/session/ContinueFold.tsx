/**
 * D-143 (amends D-103.3): the "Continue in another app" card's fold on a
 * session's page, beside the side column's Switch account… (PAT-SW).
 */
import { useState, type JSX } from "react";
import { AccountRoomLine } from "../../components/ContinueSummary";
import { useOwnRoom } from "../../lib/own-room-notices";
import type { SwitchTargetData } from "../../lib/ipc/switch-targets";
import type { SessionModel } from "./model";
import type { Target } from "./ContinueOn";

/** D-103.3 as amended by D-143: whether the card's list shows, and whether
 * it looks for room. Out of room, it opens by itself (its user can still
 * fold it); out or low, it searches at once; an ended one with room only
 * once it's opened. */
export function useFold(model: SessionModel, dialog: boolean): { open: boolean; setChosen: (open: boolean) => void; searching: boolean } {
  const ownRoom = useOwnRoom(model.profileId);
  const [chosen, setChosen] = useState<boolean | null>(null);
  const open = chosen ?? ownRoom === "out";
  return { open, setChosen, searching: model.live || ownRoom !== null || open || dialog };
}

/** D-143: folded, what's possible first ("2 apps can take it over ·
 * Personal suggested"), then its account's fact; never the fact alone.
 * `rows` undefined: it hasn't looked (an ended one with room). */
export function FoldedLines({ rows, targets, accountId }: { rows: SwitchTargetData[] | null | undefined; targets: readonly Target[]; accountId: string }): JSX.Element {
  return (
    <>
      {rows !== undefined && (
        <p className="text-meta leading-[1.5] text-ink-2" data-testid="continue-teaser">
          {continueTeaser(rows, targets)}
        </p>
      )}
      <AccountRoomLine accountId={accountId} />
    </>
  );
}

/** D-143: the folded card's line, what's possible: "2 apps can take it
 * over · Personal suggested". */
export function continueTeaser(rows: SwitchTargetData[] | null, targets: readonly Target[]): string {
  if (rows === null) return "Looking for room…";
  const open = targets.filter((target) => target.blocked === null);
  if (open.length === 0) return "No other app can take it now.";
  const count = open.length === 1 ? "1 app can take it over" : `${open.length} apps can take it over`;
  const best = open.find((target) => target.suggested);
  return best ? `${count} · ${best.name ?? best.label} suggested` : count;
}
