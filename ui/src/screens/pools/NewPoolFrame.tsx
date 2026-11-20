/**
 * New pool's frame (AJ-E3011): over the whole window, or (round 5) in
 * place on an empty pool's own page, with "Show the diagram" beside its
 * title; and an existing pool's starting point (its projects, its
 * everyone member's role, a project's own fixed scope).
 */
import { useEffect, type JSX, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";

import { keepTabInside } from "../../components/form";
import { IconPool } from "../../components/icons-actions";
import { LogoTile } from "../../components/ToolLogo";
import type { PoolRole } from "../../lib/ipc/pools";
import { EVERYONE, type Draft, type NotAdded } from "../../lib/new-pool";
import { showToast } from "../../components/Toast";
import { unavailableWords } from "../../lib/connector";
import { connectorStatus } from "../../lib/ipc/connector";
import type { AgentBridgeInfo } from "../../lib/ipc/agent-bridge";

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
  // D-126: the pool's tile in the accent, its name beside the title.
  const heading = (
    <span className="flex min-w-0 items-center gap-3">
      <LogoTile icon={IconPool} size={32} on />
      <h2 className="min-w-0 truncate type-section">
        {title}
        {name && <span className="font-medium text-muted"> · {name}</span>}
      </h2>
    </span>
  );
  if (embedded) {
    return (
      <section aria-label={title} className="flex h-full min-h-0 flex-col bg-canvas" data-testid="new-pool" data-embedded="true">
        <header className="flex items-center gap-3 border-b border-divider bg-surface px-6 py-3">
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
  // Over the window, it hangs off the page's body, not the screen that
  // opened it. Drawn inside that screen, its accessibility parent was the
  // window's main column, which starts right of the app's list: a press
  // by place (the walk's, Voice Control's) left of it found the column
  // missing and never reached the steps beside or Cancel ("nothing to
  // press there", the 0.4.7 nightly walk). Under the toasts and the
  // dialogs opened over it (z-50), so a toast still shows over its foot.
  return createPortal(
    <dialog ref={dialogRef} open aria-modal="true" aria-label="New pool" className="fixed inset-0 z-40 m-0 flex h-full max-h-none w-full max-w-none flex-col border-0 bg-canvas p-0 text-ink" data-testid="new-pool">
      {/* rc5 walk: the wizard takes the whole window, so its title starts
          below the window's traffic lights (the overlay title bar's top
          28 px, as the rail's does), and the bar moves the window. */}
      <header data-tauri-drag-region className="flex items-center gap-3 border-b border-divider bg-surface px-6 pb-3 pt-8">
        {heading}
      </header>
      {children}
    </dialog>,
    document.body,
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

/** Round 6 (Casey's run: two ticked Apps missing after Make pool, without
 * a word): says which chosen members the made pool doesn't hold, and why
 * — for a desktop App whose connector can't go in, that reason in words;
 * else what the pool said. Each can still be added on the diagram. */
export async function sayNotAdded(rows: readonly NotAdded[], bridge: AgentBridgeInfo | null): Promise<void> {
  if (rows.length === 0) return;
  const reasons = await Promise.all(
    rows.map(async (row) => {
      const desktop = row.ref.startsWith("app:") && !row.ref.startsWith("app:cli:");
      const status = desktop ? await connectorStatus(row.ref.slice("app:".length)).catch(() => null) : null;
      const why = status?.state === "unavailable" ? unavailableWords(status, bridge) : row.why;
      return `${row.label}: ${why}`;
    }),
  );
  const names = rows.map((row) => row.label);
  showToast({
    message: names.length === 1 ? `${names[0]} isn't in the pool` : `${names.length} you chose aren't in the pool`,
    detail: `${reasons.join(" · ")} · Add them on its diagram once that's fixed.`,
    tone: "warning",
    testId: "new-pool-not-added",
  });
}

/** Round 6 (Sam): from a project or its own pool, the templates fold away
 * behind "Or start from a template". */
export const foldsTemplates = (fromProject: boolean | undefined, existing: ExistingPool | undefined): boolean => fromProject === true || existing?.fixedScope === true;
