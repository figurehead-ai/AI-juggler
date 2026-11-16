/**
 * New pool's frame (AJ-E3011): over the whole window, or (round 5) in
 * place on an empty pool's own page, with "Show the diagram" beside its
 * title; and an existing pool's starting point (its projects, its
 * everyone member's role, a project's own fixed scope).
 */
import { useEffect, type JSX, type ReactNode, type RefObject } from "react";

import { keepTabInside } from "../../components/form";
import type { PoolRole } from "../../lib/ipc/pools";
import { EVERYONE, type Draft } from "../../lib/new-pool";

/** The setup's frame: over the whole window (New pool), or in place on an
 * empty pool's page (round 5) with "Show the diagram" beside its title. */
export function NewPoolFrame({ embedded, dialogRef, onClose, title, name, children }: { embedded: boolean; dialogRef: RefObject<HTMLDialogElement | null>; onClose: () => void; title: string; name: string | null; children: ReactNode }): JSX.Element {
  // Over the window: Escape closes it, and Tab stays inside.
  useEffect(() => {
    const frame = dialogRef.current;
    if (embedded || !frame) return () => undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
      keepTabInside(frame, event);
    };
    frame.addEventListener("keydown", onKey);
    return () => frame.removeEventListener("keydown", onKey);
  }, [embedded, dialogRef, onClose]);
  const heading = (
    <h2 className="text-title font-semibold text-ink">
      {title}
      {name && <span className="text-ink-2"> · {name}</span>}
    </h2>
  );
  if (embedded) {
    return (
      <section aria-label={title} className="flex h-full min-h-0 flex-col bg-canvas" data-testid="new-pool" data-embedded="true">
        <header className="flex items-center gap-3 bg-surface px-5 py-3">
          {heading}
          <span className="flex-1" />
          <button type="button" onClick={onClose} className="rounded text-meta font-medium text-ink-2 outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="new-pool-show-diagram">
            Show the diagram
          </button>
        </header>
        {children}
      </section>
    );
  }
  return (
    <dialog ref={dialogRef} open aria-modal="true" aria-label="New pool" className="fixed inset-0 z-50 m-0 flex h-full max-h-none w-full max-w-none flex-col border-0 bg-canvas p-0 text-ink" data-testid="new-pool">
      {/* rc5 walk: the wizard takes the whole window, so its title starts
          below the window's traffic lights (the overlay title bar's top
          28 px, as the rail's does), and the bar moves the window. */}
      <header data-tauri-drag-region className="flex items-center gap-3 bg-surface px-5 pb-3 pt-8">
        {heading}
      </header>
      {children}
    </dialog>
  );
}

/** An existing pool set up from its own page: its projects, its everyone
 * member's role, and whether its scope is its project's (fixed). */
export interface ExistingPool {
  every: boolean;
  projects: (string | null)[];
  everyoneRole: PoolRole | null;
  fixedScope: boolean;
}

/** A draft fitted to the existing pool: its projects, and (its own
 * project's pool) everyone in it, in its role. */
export function fitExisting(draft: Draft, existing: ExistingPool | undefined): Draft {
  if (!existing) return draft;
  const others = draft.members.filter((member) => member.ref !== EVERYONE.ref);
  const everyone = draft.members.find((member) => member.ref === EVERYONE.ref) ?? (existing.fixedScope ? EVERYONE : null);
  const role = existing.everyoneRole ?? everyone?.role ?? "member";
  return { ...draft, every: existing.every, projects: [...existing.projects], members: everyone ? [{ ...everyone, role }, ...others] : others };
}
