/**
 * AJ-E2327 — under an App whose sessions can't be shown (no tool folder
 * linked: nothing told which one it uses), why in plain words and
 * "Choose its folder…", where the rows would be. D-117: a quiet note
 * (L1), muted words and a rail link, never a coloured block.
 */
import type { JSX, ReactNode } from "react";
import { unlinked } from "../lib/app-folder";
import { chooseAppFolder } from "../lib/choose-app-folder";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { PlaceFolderInfo } from "../types/generated/PlaceFolderInfo";

export function AppFolderNote({ why, onChoose }: { why: string; onChoose: () => void }): JSX.Element {
  return (
    <div className="ml-3 flex flex-col gap-0.5 px-2.5 pb-1" data-testid="app-folder-note">
      <p className="text-label leading-snug text-muted">{why}</p>
      <button
        type="button"
        onClick={onChoose}
        data-rail-nav
        data-testid="app-folder-choose"
        className="self-start rounded-md text-label text-ink-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        Choose its folder…
      </button>
    </div>
  );
}

/** A rail header's folder parts, by App (`lens` "place"): its folder (or
 * why none), Choose its folder… (no folder, or one guessed by its name),
 * and the note under an unfolded one with nothing to show. */
export function headerFolder(
  lens: string,
  known: KnownPlaceInfo | null,
  label: string,
  shown: { folded: boolean; rows: number },
): { folder: PlaceFolderInfo | null; choose: (() => void) | undefined; note: ReactNode } {
  const folder = lens === "place" ? (known?.folder ?? null) : null;
  if (!known || !folder || (!unlinked(folder) && folder.by !== "named")) return { folder, choose: undefined, note: null };
  const choose = () => void chooseAppFolder(known.id, label);
  const note = unlinked(folder) && !shown.folded && shown.rows === 0 ? <AppFolderNote why={folder.why} onChoose={choose} /> : null;
  return { folder, choose, note };
}
