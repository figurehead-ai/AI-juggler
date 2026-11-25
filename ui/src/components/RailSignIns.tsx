/**
 * The rail's Sign-ins: a sign-in terminal AI Juggler opened rides above
 * the sessions until it's done (moved out of WorkflowRail.tsx, which is
 * at its size budget). AJ-E1553: one that finishes leaves by itself; one
 * that ended early stays, saying so, until Try again or Dismiss.
 */
import type { JSX } from "react";
import { SELECTED_FILL } from "./controls/selected";
import { signInShown } from "../lib/rail-sign-ins";

export function RailSignIns({
  terminals,
  selectedId,
  onSelect,
}: {
  terminals: { id: string; title: string; ended: boolean; state?: string }[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}): JSX.Element | null {
  const shown = terminals.filter(signInShown);
  if (shown.length === 0) return null;
  return (
    <div className="flex flex-col gap-0.5">
      <p className="px-3 pb-0.5 pt-1 text-meta font-semibold text-ink-2">Sign-ins</p>
      {shown.map((t) => (
        <button
          key={t.id}
          type="button"
          onClick={() => onSelect(t.id)}
          className={
            "relative flex min-w-0 w-full items-center gap-2 rounded-lg py-[7px] pl-1.5 pr-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
            (t.id === selectedId ? SELECTED_FILL : "hover:bg-surface/60")
          }
          data-testid="rail-sign-in"
        >
          {/* AJ-E1554 R1: the rail's slots: its dot in the gutter, its title,
              its state at the end; selected by a fill, never an accent line (PAT-SURF). */}
          <span aria-hidden="true" className="flex w-4 shrink-0 justify-center">
            <span className={"size-1.5 rounded-full " + (t.ended ? "bg-faint" : "bg-live animate-breathe")} />
          </span>
          <span className="min-w-0 flex-1 truncate text-meta">{t.title}</span>
          {t.state === "ended" && <span className="shrink-0 text-label text-muted">Didn't finish</span>}
        </button>
      ))}
    </div>
  );
}
