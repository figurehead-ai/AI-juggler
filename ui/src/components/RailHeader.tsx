/**
 * The rail's head: the logo (it folds and unfolds the rail, ⌘B), the
 * name, and which edition this window is when it isn't AI Juggler itself
 * (D-23: Nightly and Dev run side by side). Moved out of App.tsx.
 *
 * Folded, the 0.2 walk (F2) found the logo and the Dev mark cut off at
 * the left: the two sat side by side in a 60 px column. Folded, the mark
 * now sits under the logo, both whole; 0.4.1: both below the window's
 * traffic lights.
 */
import type { JSX } from "react";
import { IconJuggler } from "./icons";
import type { AppInfo } from "../types/generated/AppInfo";
import { useKeys } from "../lib/use-keys";

export function RailHeader({ minimal, info, onToggle }: { minimal: boolean; info: AppInfo | null; onToggle: () => void }): JSX.Element {
  const keys = useKeys("toggle-rail");
  const edition = info && info.edition !== "main" ? (info.edition === "nightly" ? "Nightly" : "Dev") : null;
  return (
    // 0.4.1 (the Floor session's walk): folded, the 60 px column sits under
    // the window's traffic lights (the overlay title bar's top 28 px), and
    // the Dev mark landed on them; the logo and mark start below that band.
    <div className={"flex shrink-0 items-center overflow-hidden " + (minimal ? "min-h-14 flex-col justify-end gap-0.5 px-1 pb-1 pt-7" : "h-14 gap-2.5 px-4")} data-testid="rail-header">
      <button
        type="button"
        title={minimal ? `Show the list (${keys.words})` : `Icons only (${keys.words})`}
        aria-label={minimal ? "Show the list" : "Show icons only"}
        aria-keyshortcuts={keys.aria}
        onClick={onToggle}
        className={"shrink-0 rounded-md text-accent outline-none hover:bg-surface focus-visible:ring-1 focus-visible:ring-accent/60 " + (minimal && edition ? "p-0.5" : "p-1")}
      >
        <span className="block drop-shadow-[0_0_10px_#7c7cf466]">
          <IconJuggler width={22} height={22} />
        </span>
      </button>
      {!minimal && <span className="whitespace-nowrap text-section font-semibold tracking-tight">AI Juggler</span>}
      {edition && (
        <span
          className={"shrink-0 rounded-full border border-accent/40 bg-accent-soft font-semibold text-ink " + (minimal ? "px-1 text-label leading-4" : "px-1.5 py-0.5 text-label")}
          data-testid="edition-badge"
          title={`${info!.name}: its own data, separate from AI Juggler`}
        >
          {edition}
        </span>
      )}
    </div>
  );
}
