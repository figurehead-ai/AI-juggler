/**
 * AJ-E3501: the Floor button in the header, beside the user menu. It opens
 * the Floor's own window (or brings it forward); ⌘⇧M does the same, and
 * from the Floor brings this window back. Where the Floor can't run yet,
 * the toast says so, and why. Nothing of the Floor runs until it's first
 * opened.
 */
import { useEffect, type JSX } from "react";

import { IconFloor } from "./icons-places";
import { showToast } from "./Toast";
import { errorText } from "../lib/bridge";
import { floorOpen, onFloorUnavailable } from "../lib/ipc/floor";

function unavailable(words: string): void {
  showToast({ id: "floor-unavailable", tone: "warning", message: words });
}

export function FloorButton(): JSX.Element {
  // ⌘⇧M and the tray open it from the backend; their failures arrive here.
  useEffect(() => onFloorUnavailable(unavailable), []);
  return (
    <button
      type="button"
      aria-label="The Floor"
      aria-keyshortcuts="Meta+Shift+M"
      title="The Floor (⌘⇧M)"
      onClick={() => void floorOpen().catch((cause: unknown) => unavailable(errorText(cause)))}
      className="flex size-9 items-center justify-center rounded-lg border border-transparent text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="floor-button"
    >
      <IconFloor width={18} height={18} aria-hidden="true" />
    </button>
  );
}
