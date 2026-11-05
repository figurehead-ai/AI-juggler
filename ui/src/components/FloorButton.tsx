/**
 * AJ-E3501, AJ-E3511 (D-78): the Floor button in the header, beside the
 * user menu — a split button. One click opens the Floor's own window (or
 * brings it forward); ⌘⇧M does the same, and from the Floor brings this
 * window back. The small arrow opens its menu: Open the Floor, the Floor
 * list, Floor settings…. Where the Floor can't run yet, the toast says so,
 * and why. Nothing of the Floor runs until it's first opened.
 */
import { useEffect, useState, type JSX } from "react";

import { IconFloor } from "./icons-places";
import { showToast } from "./Toast";
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { IconPage } from "./icons-actions";
import { IconGear } from "./icons-menu";
import { errorText } from "../lib/bridge";
import { floorOpen, onFloorUnavailable } from "../lib/ipc/floor";
import { useKeys } from "../lib/use-keys";

function unavailable(words: string): void {
  showToast({ id: "floor-unavailable", tone: "warning", message: words });
}

const open = () => void floorOpen().catch((cause: unknown) => unavailable(errorText(cause)));
const PART = "flex h-9 items-center justify-center border border-transparent text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60";

export function FloorButton(): JSX.Element {
  // ⌘⇧M and the tray open it from the backend; their failures arrive here.
  useEffect(() => onFloorUnavailable(unavailable), []);
  const [at, setAt] = useState<MenuAt | null>(null);
  const keys = useKeys("floor");
  const close = () => setAt(null);
  return (
    <span className="flex items-center" data-testid="floor-split">
      <button type="button" aria-label="The Floor" aria-keyshortcuts={keys.aria} title={`The Floor (${keys.words})`} onClick={open} className={`${PART} w-9 rounded-l-lg`} data-testid="floor-button">
        <IconFloor width={18} height={18} aria-hidden="true" />
      </button>
      <button
        type="button"
        aria-label="More of the Floor"
        aria-haspopup="menu"
        aria-expanded={at !== null}
        onClick={(event) => setAt(at ? null : menuPoint(event))}
        className={`${PART} w-6 rounded-r-lg text-[10px]`}
        data-testid="floor-menu-button"
      >
        <span aria-hidden="true">▾</span>
      </button>
      {at && (
        <RowMenu heading="" ariaLabel="The Floor" at={at} minWidth={220} testId="floor-menu" onClose={close}>
          <MenuItem onClick={() => (close(), open())} testId="floor-menu-open">
            <MenuLine icon={IconFloor} label="Open the Floor" hint={keys.words} />
          </MenuItem>
          <MenuItem onClick={() => (close(), window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "floor" } })))} testId="floor-menu-list">
            <MenuLine icon={IconPage} label="The Floor list" />
          </MenuItem>
          <MenuItem onClick={() => (close(), window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "the-floor" } })))} testId="floor-menu-settings">
            <MenuLine icon={IconGear} label="Floor settings…" />
          </MenuItem>
        </RowMenu>
      )}
    </span>
  );
}
