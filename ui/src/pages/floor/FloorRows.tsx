/**
 * AJ-E3512 — one floor of the Floor list: its one header (sticky: the
 * floor, its Accounts' room said once, and its columns), then its Apps
 * and their sessions in the shared paged list. Each App's header is its
 * own mark and name (a door to its page) and its Show on the Floor; a
 * session row shows its App on the Floor too, by click or ↩ (the 0.4
 * review: one header per floor, not per App; the room once; columns with
 * no value on the floor left out).
 */
import { useEffect, useMemo, type JSX } from "react";
import { AppIcon } from "../../components/AppIcon";
import { PagedList } from "../../components/PagedList";
import { ProfileBadge } from "../../components/ProfileBadge";
import { Button } from "../../components/primitives";
import { openApp } from "../../lib/apps";
import { appOfTool } from "../../lib/app-of";
import type { FloorApp, FloorSession, FloorSnapshot } from "../../lib/ipc/floor";
import { pageOf, usePagedList } from "../../lib/paged";
import { appNote, appParts, columnTemplate, floorAccounts, floorColumns, floorRows, GLYPH, isWaiting, type FloorColumns, type FloorGroup, type FloorRow } from "./floor-list-model";

/** Rows a page of a floor holds. */
const FLOOR_PAGE = 50;

/** An App's mark and name, as the rail's: its app's own icon, its
 * profile's dot, then its name (a door to its page). */
function AppHeader({ app, now, onShow }: { app: FloorApp; now: number; onShow: () => void }): JSX.Element {
  const parts = appParts(app);
  const icon = app.host_app ? { appId: app.host_app, name: parts.app } : { appId: appOfTool(app.tool).appId, name: appOfTool(app.tool).name || parts.app };
  const note = appNote(app, now);
  return (
    <div className="flex flex-wrap items-center gap-x-2 gap-y-1 bg-surface-2 px-3 py-1.5" data-testid="floor-list-app">
      <span className="flex shrink-0 items-center gap-1.5" data-testid="floor-list-app-mark">
        <AppIcon appId={icon.appId} name={icon.name} size={20} />
        {parts.profile && <ProfileBadge app={app.colour_slot} name={parts.profile} />}
      </span>
      <h4 className="min-w-0 text-body font-semibold text-ink">
        <button type="button" onClick={() => openApp(app.id)} className="max-w-full truncate rounded text-left outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" title={`Open ${app.name}'s page`}>
          {app.name}
        </button>
      </h4>
      {note && <span className="text-meta text-muted">{note}</span>}
      <Button size="sm" variant="ghost" className="ml-auto" onClick={onShow} aria-label={`Show on the Floor: ${app.name}`}>
        Show on the Floor
      </Button>
    </div>
  );
}

/** One session: its state (a glyph and a word), its name, and the
 * figures its floor has columns for, only where there is one. A click
 * or ↩ shows its App on the Floor. */
function SessionRow({ session, columns, template, onShow }: { session: FloorSession; columns: FloorColumns; template: string; onShow: () => void }): JSX.Element {
  const waiting = isWaiting(session);
  return (
    <button
      type="button"
      onClick={onShow}
      title="Show on the Floor (↩)"
      className="grid w-full items-center gap-x-3 px-3 py-2 text-left text-meta outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60"
      style={{ gridTemplateColumns: template }}
      data-testid="floor-list-row"
      data-state={session.state}
    >
      <span className="flex min-w-0 items-center gap-2">
        <span aria-hidden="true" className={"w-3 shrink-0 text-center font-semibold " + (waiting ? "text-warning" : "text-muted")}>
          {GLYPH[session.state]}
        </span>
        <span className={"truncate " + (waiting ? "font-semibold text-ink" : "text-ink")}>{session.word}</span>
      </span>
      <span className="truncate text-ink" title={session.title}>
        <span className="sr-only">, </span>
        {session.title}
      </span>
      {columns.project && <span className="truncate text-ink-2">{session.project && <><span className="sr-only">, project </span>{session.project}</>}</span>}
      {columns.subagents && <span className="text-right tabular-nums text-ink-2">{session.subagents > 0 && <><span className="sr-only">, </span>{session.subagents}<span className="sr-only"> subagents</span></>}</span>}
      {columns.tasks && <span className="text-right tabular-nums text-ink-2">{session.tasks > 0 && <><span className="sr-only">, </span>{session.tasks}<span className="sr-only"> tasks</span></>}</span>}
      <span className="sr-only">. Show on the Floor</span>
    </button>
  );
}

export function FloorSection({ group, snapshot, now, query, waitingOnly, onShow }: { group: FloorGroup; snapshot: FloorSnapshot; now: number; query: string; waitingOnly: boolean; onShow: (appId: string) => void }): JSX.Element | null {
  const columns = useMemo(() => floorColumns(group), [group]);
  const template = columnTemplate(columns);
  const rows = useMemo(() => floorRows(group, query, waitingOnly), [group, query, waitingOnly]);
  // The rows are the UI's (the Floor's snapshot): paged as every growing
  // list is, the first page in the first render.
  const list = usePagedList<FloorRow>(`floor-list:${group.floor}:${waitingOnly ? "waiting" : "all"}:${query.trim().toLowerCase()}`, async (page) => pageOf(rows, page), {
    pageSize: FLOOR_PAGE,
    keyOf: (row) => row.key,
    seed: (size) => pageOf(rows, { cursor: null, limit: size }),
  });
  // A new snapshot: read again, keeping as many rows as are shown.
  const { refresh } = list;
  useEffect(() => refresh(), [rows, refresh]);
  if (rows.length === 0) return null;
  const accounts = floorAccounts(group, snapshot);
  const id = `floor-${group.floor}`;
  return (
    <section aria-labelledby={id} className="flex flex-col" data-testid="floor-list-floor">
      <div className="sticky top-0 z-10 flex flex-col gap-1.5 bg-canvas pb-1.5 pt-1" data-testid="floor-list-floor-head">
        <div className="flex flex-wrap items-baseline gap-x-3 gap-y-0.5">
          <h3 id={id} className="text-section font-semibold text-ink">
            Floor {group.floor + 1}
          </h3>
          {accounts.length > 0 && (
            <span className="text-meta text-ink-2" data-testid="floor-list-accounts">
              {accounts.join("  ·  ")}
            </span>
          )}
        </div>
        {/* The floor's one column header; each row says its own figures in words to a screen reader. */}
        <div aria-hidden="true" className="grid gap-x-3 px-3 text-label font-semibold text-muted" style={{ gridTemplateColumns: template }} data-testid="floor-list-columns">
          <span>State</span>
          <span>Session</span>
          {columns.project && <span>Project</span>}
          {columns.subagents && <span className="text-right">Subagents</span>}
          {columns.tasks && <span className="text-right">Tasks</span>}
        </div>
      </div>
      <PagedList
        list={list}
        keyOf={(row) => row.key}
        label={`Floor ${group.floor + 1}`}
        rowHeight={38}
        skeletonRows={3}
        empty={null}
        testId={`floor-list-${group.floor}`}
        renderRow={(row) => (row.kind === "app" ? <AppHeader app={row.app} now={now} onShow={() => onShow(row.app.id)} /> : <SessionRow session={row.session} columns={columns} template={template} onShow={() => onShow(row.app.id)} />)}
      />
    </section>
  );
}
