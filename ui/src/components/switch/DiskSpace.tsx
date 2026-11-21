/**
 * 0.3.9: the disk space a switch uses, on the screen before Switch: the
 * conversation's size and what the copy takes (almost nothing when it's a
 * clone), what's free, and when that's low or short what to do. Short
 * means the switch can't start: the Switch button waits (the copy itself
 * also stops before it would fill the disk).
 */
import type { JSX } from "react";
import { useResource } from "../../lib/use-resource";
import { Alert } from "../Alert";
import { IconFolderOpen } from "../icons-actions";
import { Button } from "../primitives";
import { Fact } from "./Facts";

import { switchSpace, type SwitchSpaceInfo } from "../../lib/ipc/switchSpace";

/** The disk space switching `sessionId` there uses; `null` until known
 * (or when it can't be read: the copy still checks as it goes). */
export function useSwitchSpace(sessionId: string, targetProfileId: string): { space: SwitchSpaceInfo | null; recheck: () => void } {
  const space = useResource(`switch-space:${sessionId}:${targetProfileId}`, () => switchSpace(sessionId, targetProfileId));
  return { space: space.error === undefined ? (space.data ?? null) : null, recheck: space.reload };
}

/** Whether the switch can't start for want of disk space. */
export function tooLittleSpace(space: SwitchSpaceInfo | null): boolean {
  return space?.kind === "short";
}

/** The space as a fact row; short, the one thing that stops the switch,
 * as a needs-you Alert with its clearing action (ALR-4). */
export function DiskSpace({ space, onCheckAgain }: { space: SwitchSpaceInfo | null; onCheckAgain?: () => void }): JSX.Element | null {
  if (!space || space.kind === "none" || !space.words) return null;
  if (space.kind === "short") {
    return (
      <Alert
        level="needs-you"
        live={null}
        testId="switch-disk-space"
        data-kind={space.kind}
        title={space.words}
        actions={
          onCheckAgain && (
            <Button size="sm" variant="secondary" type="button" onClick={onCheckAgain} data-testid="switch-disk-space-again">
              Check again
            </Button>
          )
        }
      >
        {space.advice && <span data-testid="switch-disk-space-advice">{space.advice}</span>}
      </Alert>
    );
  }
  return (
    <Fact icon={IconFolderOpen} label="Disk space" tone={space.kind === "low" ? "warn" : "plain"} testId="switch-disk-space">
      <span data-kind={space.kind}>
        {space.words}
        {space.advice && (
          <span className="block text-meta text-ink-2" data-testid="switch-disk-space-advice">
            {space.advice}
          </span>
        )}
      </span>
    </Fact>
  );
}
