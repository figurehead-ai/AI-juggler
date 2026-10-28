/**
 * The rail's head: the logo (it folds and unfolds the rail, ⌘B), the
 * name, and which edition this window is when it isn't AI Juggler itself
 * (D-23: Nightly and Dev run side by side). Moved out of App.tsx.
 *
 * Folded, the 0.2 walk (F2) found the logo and the Dev mark cut off at
 * the left: the two sat side by side in a 60 px column. Folded, the mark
 * now sits under the logo, both whole.
 */
import type { JSX } from "react";
import { IconJuggler } from "./icons";
import type { AppInfo } from "../lib/bridge";

export function RailHeader({ minimal, info, onToggle }: { minimal: boolean; info: AppInfo | null; onToggle: () => void }): JSX.Element {
  const edition = info && info.edition !== "main" ? (info.edition === "nightly" ? "Nightly" : "Dev") : null;
  return (
    <div className={"flex h-14 shrink-0 items-center overflow-hidden " + (minimal ? "flex-col justify-center gap-0.5 px-1" : "gap-2.5 px-4")} data-testid="rail-header">
      <button
        type="button"
        title={minimal ? "Show the list (⌘B)" : "Icons only (⌘B)"}
        aria-label={minimal ? "Show the list" : "Show icons only"}
        aria-keyshortcuts="Meta+B"
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
