/**
 * The rail's Sign-ins: a sign-in terminal AI Juggler opened rides above
 * the sessions until it's done (moved out of WorkflowRail.tsx, which is
 * at its size budget).
 */
import type { JSX } from "react";

export function RailSignIns({
  terminals,
  selectedId,
  onSelect,
}: {
  terminals: { id: string; title: string; ended: boolean }[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}): JSX.Element | null {
  if (terminals.length === 0) return null;
  return (
    <div className="flex flex-col gap-0.5">
      <p className="px-3 pb-0.5 pt-1 text-meta font-semibold text-muted">Sign-ins</p>
      {terminals.map((t) => (
        <button
          key={t.id}
          type="button"
          onClick={() => onSelect(t.id)}
          className={
            "relative flex min-w-0 w-full items-center gap-2 rounded-lg py-[7px] pl-3 pr-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
            (t.id === selectedId ? "bg-surface" : "hover:bg-surface/60")
          }
        >
          {t.id === selectedId && <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />}
          <span className={"size-1.5 shrink-0 rounded-full " + (t.ended ? "bg-faint" : "bg-live animate-breathe")} />
          <span className="min-w-0 flex-1 truncate text-meta">{t.title}</span>
        </button>
      ))}
    </div>
  );
}
