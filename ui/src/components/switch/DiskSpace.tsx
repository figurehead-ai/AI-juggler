/**
 * 0.3.9: the disk space a switch uses, on the screen before Switch: the
 * conversation's size and what the copy takes (almost nothing when it's a
 * clone), what's free, and when that's low or short what to do. Short
 * means the switch can't start: the Switch button waits (the copy itself
 * also stops before it would fill the disk).
 */
import { useEffect, useState, type JSX } from "react";

import { switchSpace, type SwitchSpaceInfo } from "../../lib/ipc/switchSpace";

/** The disk space switching `sessionId` there uses; `null` until known
 * (or when it can't be read: the copy still checks as it goes). */
export function useSwitchSpace(sessionId: string, targetProfileId: string): { space: SwitchSpaceInfo | null; recheck: () => void } {
  const [space, setSpace] = useState<SwitchSpaceInfo | null>(null);
  const [round, setRound] = useState(0);
  useEffect(() => {
    let alive = true;
    void switchSpace(sessionId, targetProfileId).then(
      (next) => alive && setSpace(next),
      () => alive && setSpace(null),
    );
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId, round]);
  return { space, recheck: () => setRound((was) => was + 1) };
}

/** Whether the switch can't start for want of disk space. */
export function tooLittleSpace(space: SwitchSpaceInfo | null): boolean {
  return space?.kind === "short";
}

export function DiskSpace({ space, onCheckAgain }: { space: SwitchSpaceInfo | null; onCheckAgain?: () => void }): JSX.Element | null {
  if (!space || space.kind === "none" || !space.words) return null;
  const short = space.kind === "short";
  return (
    <div className="flex flex-col gap-0.5" data-testid="switch-disk-space" data-kind={space.kind}>
      <p className={"text-body " + (short ? "font-semibold text-warning" : "text-ink-2")}>{space.words}</p>
      {space.advice && (
        <p className={"text-body " + (short || space.kind === "low" ? "text-warning" : "text-ink-2")} data-testid="switch-disk-space-advice">
          {space.advice}
        </p>
      )}
      {short && onCheckAgain && (
        <button type="button" onClick={onCheckAgain} className="self-start rounded-lg border border-hairline px-2.5 py-1 text-meta text-ink outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="switch-disk-space-again">
          Check again
        </button>
      )}
    </div>
  );
}
