/**
 * The canvas's palette: the open sessions this pool could take, to drag
 * onto the canvas (or add with the keyboard: Add puts it in the next free
 * place). While a member is dragged, it turns into the place to drop it
 * to take it out of the pool. Folds to a single line.
 */
import { useState, type JSX, type PointerEvent } from "react";

import { ToolMark } from "../../components/ToolMark";
import { IconGrip } from "../../components/icons-places";
import { IconPlus, IconRemove } from "../../components/icons-actions";
import type { InstanceInfo, ProjectInfo } from "../../lib/bridge";
import { nameOf } from "./actions";

/** Search shows once the palette holds more than this many. */
const SEARCH_FROM = 8;
/** The most shown at once; typing finds the rest. */
const SHOWN = 30;

export function Palette({
  sessions,
  projects,
  removing,
  open,
  onOpen,
  onStartDrag,
  onAdd,
}: {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  /** A member is being dragged: `over` while it's over the palette, `why`
   * when it can't be taken out. */
  removing: { over: boolean; why: string | null } | null;
  /** Unfolded (the canvas decides: folded on a narrow canvas). */
  open: boolean;
  onOpen: (open: boolean) => void;
  onStartDrag: (session: InstanceInfo, event: PointerEvent<HTMLElement>) => void;
  onAdd: (session: InstanceInfo) => void;
}): JSX.Element {
  const [words, setWords] = useState("");
  const query = words.trim().toLowerCase();
  const projectOf = (session: InstanceInfo) => projects.find((project) => project.id === session.project_id)?.name ?? null;
  const shown = sessions
    .filter((session) => !query || `${nameOf(session)} ${projectOf(session) ?? ""} ${session.account_label}`.toLowerCase().includes(query))
    .slice(0, SHOWN);

  if (removing) {
    return (
      <div
        data-canvas-palette
        data-testid="canvas-remove-zone"
        className={
          "pointer-events-auto flex w-60 flex-col items-center gap-1.5 rounded-xl border-2 border-dashed p-4 text-center text-meta shadow-card " +
          (removing.why ? "border-hairline bg-surface text-muted" : removing.over ? "border-danger bg-danger-soft text-danger" : "border-danger/60 bg-surface text-ink")
        }
      >
        <IconRemove width={18} height={18} aria-hidden="true" />
        {removing.why ?? "Drop here to take it out of the pool"}
      </div>
    );
  }
  return (
    <section data-canvas-palette aria-label="Add a session" className="pointer-events-auto flex max-h-full w-60 flex-col rounded-xl border border-hairline bg-surface/95 shadow-card backdrop-blur" data-testid="canvas-palette">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => onOpen(!open)}
        className="flex items-center gap-2 rounded-xl px-3 py-2 text-left text-label font-semibold uppercase tracking-[0.08em] text-faint outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <span className="flex-1">Add a session</span>
        <span className="font-mono normal-case tracking-normal">{sessions.length}</span>
        <svg aria-hidden="true" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
          <path d={open ? "M6 15l6-6 6 6" : "M6 9l6 6 6-6"} />
        </svg>
      </button>
      {open && (
        <div className="flex min-h-0 flex-col gap-1.5 px-2 pb-2">
          {sessions.length === 0 ? (
            <p className="px-1 pb-1 text-label text-muted" data-testid="canvas-palette-empty">
              Every open session that can join is here already. Start one and it shows up here.
            </p>
          ) : (
            <>
              <p className="px-1 text-label text-muted">Drag one onto the canvas, or Add.</p>
              {sessions.length > SEARCH_FROM && (
                <input
                  type="search"
                  value={words}
                  onChange={(event) => setWords(event.target.value)}
                  placeholder="Find a session"
                  aria-label="Find a session to add"
                  className="h-8 rounded-lg border border-hairline bg-surface-2 px-2 text-meta text-ink outline-none placeholder:text-faint focus-visible:ring-1 focus-visible:ring-accent/60"
                />
              )}
              <ul className="flex min-h-0 flex-col gap-1 overflow-y-auto" data-testid="canvas-palette-list">
                {shown.map((session) => (
                  <li
                    key={session.id}
                    onPointerDown={(event) => {
                      if ((event.target as HTMLElement).closest("button")) return;
                      onStartDrag(session, event);
                    }}
                    className="group flex touch-none cursor-grab items-center gap-2 rounded-lg border border-dashed border-hairline bg-surface px-2 py-1.5 hover:border-accent/60 hover:bg-surface-2"
                    data-testid="canvas-palette-item"
                    data-session={session.id}
                  >
                    <IconGrip width={14} height={14} className="shrink-0 text-faint" aria-hidden="true" />
                    <ToolMark tool={session.provider} size={14} className="shrink-0" />
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate text-meta text-ink">{nameOf(session)}</span>
                      <span className="truncate text-label text-faint">{[projectOf(session), session.account_label].filter(Boolean).join(" · ")}</span>
                    </span>
                    <button
                      type="button"
                      onClick={() => onAdd(session)}
                      aria-label={`Add ${nameOf(session)} to this pool`}
                      className="flex size-6 shrink-0 items-center justify-center rounded-md text-muted outline-none hover:bg-accent-soft hover:text-accent-hi focus-visible:ring-1 focus-visible:ring-accent/60"
                      data-testid="canvas-palette-add"
                    >
                      <IconPlus width={14} height={14} />
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </section>
  );
}
