import { AccountAvatar } from "./Avatar";
import { ProjectMark } from "./ProjectMark";
import { AppIcon } from "./AppIcon";
import { appOfSession } from "../lib/app-of";
import { markOf, swatchColor } from "../lib/palette";
import { RailQuietNotes } from "./RailNotes";
import { parkedWords } from "../lib/parked";
import { providerDisplayName } from "../lib/providers";
import { ReactNode, useCallback, useEffect, useId, useRef, useState } from "react";
import { IconProjects, IconTerminal } from "./icons";
import { IconHistory } from "./icons-places";
import { PinMark, ProjectRow, RailProjectHeader } from "./RailProjectRows";
import { RailCalm } from "./RailCalm";
import { FoldButton, FoldedNeeds, HISTORY_GROUP, HISTORY_SHOWN, readFolded, repoGroups, safeId, writeFolded } from "./rail-groups";
import { RowPlaceholders } from "./Placeholder";
import { showToast } from "./Toast";
import { sessionLabel } from "../lib/stop";

/** "15 min", "1 h", "2 h 30 min" (AJ-E1503's snooze toast). */
function durationWords(ms: number): string {
  const minutes = Math.max(1, Math.round(ms / 60_000));
  const hours = Math.floor(minutes / 60);
  if (hours === 0) return `${minutes} min`;
  return minutes % 60 === 0 ? `${hours} h` : `${hours} h ${minutes % 60} min`;
}
import type { AccountInfo, InstanceInfo, ProjectInfo, RemoteMachineInfo } from "../lib/bridge";
import { shortHeavy, useHeavyNote } from "../lib/machine";
import type { Density } from "../lib/density";
import { REFRESH_INTERVALS, staleCaption } from "../lib/density";
import {
  NO_CHIP_FILTERS,
  isSnoozed,
  readSnooze,
  snoozeCaption,
  sortSessions,
  storeSnooze,
  filterSessions,
  groupSessions,
  type ChipFilters,
  type GroupBy,
  type SnoozeState,
} from "../lib/railview";
import { Button } from "./primitives";
import { sessionTitle, useProjectManage, useSessionManage } from "./ManageWork";
import { ActionHostProvider, useItemMenu } from "./ItemMenu";
import { OutsideRows, outsideScope, railSlice, withOutsideGroups } from "./OutsideConversations";
import { AlsoLine, NeedsYouSection, OPEN_NOW, OpenNowLabel, openGroups, railOrder } from "./RailSections";
import { RailToolbar } from "./RailToolbar";
import { PlaceBoard } from "./PlaceBoard";
import { OtherMacs } from "./RemoteBoard";
import { modelDrift } from "./SessionCards";
import { parseQuery, useRailView } from "../lib/rail-view";
import { goToWhere, sessionItem, sessionState } from "../lib/rail-items";
// AJ-E2104: the indicator row and card moved to their own files.
export { SessionRow, modelDrift } from "./SessionCards";
export { SessionCard } from "./SessionCard";
import { useConversations } from "../lib/conversations";
import { sessionWhen } from "../lib/time-words";
import type { ActionHost } from "../lib/actions";
import {
  StaleChip,
  attentionBadge,
  attentionTooltip,
  isActiveStatus,
  lowLeftWords,
  stateChip,
} from "./indicator";
import { ariaSummary } from "./row-details";

/** Freshness timestamps per Instance id (epoch ms) driving the stale rule. */
export type { Freshness } from "../lib/live-deltas";
import type { Freshness } from "../lib/live-deltas";

const WIDTH_MIN = 208;
const WIDTH_MAX = 360;
const WIDTH_DEFAULT = 264;
const STORE_WIDTH = "aij.rail.width";

/** The rail's stored width; nothing stored (or junk) is the default —
 * `Number(null)` is 0, which clamped every install to the minimum. */
export function readWidth(): number {
  const stored = localStorage.getItem(STORE_WIDTH);
  const raw = stored === null || stored.trim() === "" ? Number.NaN : Number(stored);
  if (!Number.isFinite(raw)) return WIDTH_DEFAULT;
  return Math.min(WIDTH_MAX, Math.max(WIDTH_MIN, raw));
}

/**
 * The workflow navigator (ADR-0007): one rail, three tabs, and the live
 * list of whichever the user is working through. The rail drag-resizes
 * (persisted), collapses to icons, and every element — tabs first —
 * reflows gracefully at any width: labels truncate cleanly, then fade to
 * icon-only below the label threshold. Nothing ever clips.
 */
export function WorkflowRail({
  density,
  onCycleDensity,
  projects,
  sessions,
  accounts = [],
  loginTerminals,
  selectedId,
  onSelectSession,
  onSelectProject,
  onNewSession,
  onNewProject,
  header,
  footer,
  freshness = {},
  now = Date.now(),
  remoteMachines = [],
  selectedRemote = null,
  onSelectRemote,
  loading = false,
  onChanged,
}: {
  /** AJ-E1503: the first load hasn't answered yet (placeholders). */
  loading?: boolean;
  /** AJ-E1502: a rename, pin or forget from a row's menu — the app reads
   * its lists again. */
  onChanged?: () => void;
  density: Density;
  onCycleDensity: () => void;
  projects: ProjectInfo[];
  sessions: InstanceInfo[];
  /** AJ-E411 Req.5: account expiry feeds the Needs-me tie-break. */
  accounts?: AccountInfo[];
  loginTerminals: { id: string; title: string; ended: boolean }[];
  selectedId: string | null;
  onSelectSession: (id: string) => void;
  onSelectProject: (id: string) => void;
  onNewSession: () => void;
  /** AJ-E305 flow (b): the Projects tab's primary action. */
  onNewProject: () => void;
  header: (minimal: boolean) => ReactNode;
  footer: (minimal: boolean) => ReactNode;
  freshness?: Freshness;
  now?: number;
  /** AJ-E709: paired machines' Instances on the board. */
  remoteMachines?: RemoteMachineInfo[];
  selectedRemote?: { machineId: string; instanceId: string | null } | null;
  onSelectRemote?: (machineId: string, instanceId: string) => void;
}) {
  const minimal = density === "minimal";
  // Snooze/mute state (AJ-E404 Req.6): rail-owned, persisted; the tab
  // badge counts UNSNOOZED attention only.
  const [snooze, setSnooze] = useState<SnoozeState>(() => readSnooze());
  // AJ-E1503: an updater form too, so Undo applies to the state as it
  // is then, not as it was when the toast opened.
  const updateSnooze = useCallback((next: SnoozeState | ((current: SnoozeState) => SnoozeState)) => {
    setSnooze((current) => {
      const value = typeof next === "function" ? next(current) : next;
      storeSnooze(value);
      return value;
    });
  }, []);
  const [width, setWidth] = useState<number>(readWidth);
  const [dragging, setDragging] = useState(false);
  const dragState = useRef<{ startX: number; startWidth: number } | null>(null);

  const beginDrag = useCallback(
    (event: React.PointerEvent) => {
      if (minimal) return;
      event.preventDefault();
      dragState.current = { startX: event.clientX, startWidth: width };
      setDragging(true);
    },
    [minimal, width],
  );

  useEffect(() => {
    if (!dragging) return;
    const onMove = (move: PointerEvent) => {
      const state = dragState.current;
      if (!state) return;
      const next = Math.min(
        WIDTH_MAX,
        Math.max(WIDTH_MIN, state.startWidth + (move.clientX - state.startX)),
      );
      setWidth(next);
    };
    const onUp = () => {
      setDragging(false);
      dragState.current = null;
      setWidth((current) => {
        localStorage.setItem(STORE_WIDTH, String(current));
        return current;
      });
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, [dragging]);

  return (
    <aside
      data-testid="rail"
      data-density={density}
      style={{ width: minimal ? 60 : width }}
      className={
        "relative flex h-full shrink-0 select-none flex-col border-r border-hairline bg-surface/30 backdrop-blur-xl transition-[width] duration-150 ease-out " +
        (dragging ? "" : "")
      }
    >
      {header(minimal)}

      {/* The live list for the active tab */}
      <div className="rail-scroll min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-2 pb-2">
        {minimal ? (
          <MinimalInstanceColumn
            sessions={sessions}
            selectedId={selectedId}
            onSelect={onSelectSession}
          />
        ) : (
          <InstanceBoard
            onSelectProject={onSelectProject}
            sessions={sessions}
            accounts={accounts}
            loginTerminals={loginTerminals}
            selectedId={selectedId}
            onSelect={onSelectSession}
            density={density}
            freshness={freshness}
            now={now}
            projects={projects}
            snooze={snooze}
            onSnooze={updateSnooze}
            remoteMachines={remoteMachines}
            selectedRemote={selectedRemote}
            onSelectRemote={onSelectRemote}
            loading={loading}
            onChanged={onChanged}
          />
        )}
      </div>

      {/* D-25: one primary action; New project stays one click away. */}
      <div className={"flex gap-1.5 border-t border-hairline/60 p-2 " + (minimal ? "flex-col" : "")}>
        <button
          type="button"
          onClick={onNewSession}
          title={minimal ? "New session" : undefined}
          aria-label={minimal ? "New session" : undefined}
          data-testid="rail-new-session"
          className="flex min-w-0 flex-1 items-center justify-center gap-2 overflow-hidden rounded-lg border border-accent/30 bg-accent-soft px-3 py-[7px] text-meta font-medium text-accent-hi outline-none transition-colors hover:border-accent/50 hover:bg-accent/15 focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <IconTerminal width={14} height={14} />
          {!minimal && <span className="truncate">New session</span>}
        </button>
        {/* AJ-E2104: History, one click from the list's foot. */}
        <button
          type="button"
          onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history" } }))}
          aria-label="History"
          title="History"
          data-testid="rail-history"
          className="flex shrink-0 items-center justify-center rounded-lg border border-hairline px-2.5 py-[7px] text-muted outline-none transition-colors hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <IconHistory width={14} height={14} />
        </button>
        <button
          type="button"
          onClick={onNewProject}
          aria-label="New project"
          title="New project"
          data-testid="rail-new-project"
          className="flex shrink-0 items-center justify-center rounded-lg border border-hairline px-2.5 py-[7px] text-muted outline-none transition-colors hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <IconProjects width={14} height={14} />
        </button>
      </div>

      {footer(minimal)}

      {/* Drag-resize handle: hairline that lights up on hover/drag */}
      {!minimal && (
        <div
          onPointerDown={beginDrag}
          onDoubleClick={onCycleDensity}
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize navigator"
          className={
            "absolute right-0 top-0 z-20 h-full w-1.5 cursor-col-resize " +
            (dragging ? "" : "transition-colors")
          }
        >
          <div
            className={
              "ml-auto h-full w-px " +
              (dragging ? "bg-accent/70" : "bg-transparent hover:bg-accent/50")
            }
          />
        </div>
      )}
    </aside>
  );
}

const VIRTUALISE_AT = 50;
export const ROW_HEIGHT = { comfortable: 50, compact: 32 } as const;

/**
 * The sorted/filtered/searchable Instance board (AJ-E404): toolbar above
 * the list, Needs-me sort by default (snoozed sink below peers, never
 * disappear), quota-risk with a `no data` group, group-by rollups, ⌘K
 * search, and windowed rendering past 50 rows. The legacy slice caps are
 * gone — every matching row is reachable.
 */
export function InstanceBoard({
  sessions,
  loginTerminals,
  selectedId,
  onSelect,
  density,
  freshness,
  now,
  projects,
  accounts = [],
  snooze,
  onSnooze,
  remoteMachines = [],
  selectedRemote = null,
  onSelectRemote,
  onSelectProject,
  loading = false,
  onChanged = () => {},
}: {
  /** AJ-E1503: placeholders until the first load answers. */
  loading?: boolean;
  /** AJ-E1502: something was renamed, pinned or forgotten. */
  onChanged?: () => void;
  /** AJ-E411 Req.5: account expiry feeds the Needs-me tie-break. */
  accounts?: AccountInfo[];
  /** D-25: a project group's header opens the project. */
  onSelectProject?: (id: string) => void;
  remoteMachines?: RemoteMachineInfo[];
  selectedRemote?: { machineId: string; instanceId: string | null } | null;
  onSelectRemote?: (machineId: string, instanceId: string) => void;
  sessions: InstanceInfo[];
  loginTerminals: { id: string; title: string; ended: boolean }[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  density: Density;
  freshness: Freshness;
  now: number;
  projects: ProjectInfo[];
  snooze: SnoozeState;
  onSnooze: (next: SnoozeState | ((current: SnoozeState) => SnoozeState)) => void;
}) {
  // AJ-E2104: the View (Group by, Show, Only) and the search's words.
  const view = useRailView();
  const [query, setQuery] = useState("");
  const parsed = parseQuery(query);
  // Folded groups (a project's sessions, or "Your projects"): remembered.
  const [folded, setFolded] = useState<Set<string>>(readFolded);
  const toggleFold = useCallback((id: string) => {
    setFolded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      writeFolded(next);
      return next;
    });
  }, []);
  const [allProjects, setAllProjects] = useState(false);
  // AJ-E1534: the open page's row is in view (its selection, whatever its type).
  useEffect(() => {
    document.querySelector<HTMLElement>("[data-testid='rail'] [data-rail-selected='true']")?.scrollIntoView?.({ block: "nearest" });
  }, [selectedId]);
  // AJ-E709: Only › Mac narrows to this Mac or one paired Mac.
  const hasRemote = remoteMachines.length > 0;
  const mac = view.only.mac;
  const showLocal = mac === undefined || mac === "this-mac";
  const remoteShown = remoteMachines.filter((machine) => mac === undefined || mac === machine.machine_id);
  const { all: conversations } = useConversations();
  // AJ-E1502/E1537: rows name, pin and tidy through the action registry.
  const sessionManage = useSessionManage(onChanged);
  const projectManage = useProjectManage(onChanged);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(640);

  // ⌘K focuses the rail search from anywhere (App dispatches the event).
  useEffect(() => {
    const focus = () => searchRef.current?.focus();
    window.addEventListener("aij:focus-search", focus);
    return () => window.removeEventListener("aij:focus-search", focus);
  }, []);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const observe = () => setViewportH(element.clientHeight || 640);
    observe();
    // jsdom has no ResizeObserver; the initial measure above suffices.
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(observe);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const effectiveDensity = density === "minimal" ? "compact" : density;
  // The Project lens: the View's Only, as the project board's filters.
  const filters: ChipFilters = {
    ...NO_CHIP_FILTERS,
    account: sessions.find((row) => row.account_label === view.only.account)?.account_id ?? (view.only.account ? "-" : null),
    project: projects.find((project) => project.name === view.only.project)?.id ?? (view.only.project ? "-" : null),
  };
  const filtered = filterSessions(sessions, filters, parsed.text, projects);
  const sorted = sortSessions(filtered, "needs", snooze, now, projects, accounts);
  const effectiveGroup: GroupBy = "project";
  const sessionGroups = groupSessions(sorted, effectiveGroup, projects);
  // AJ-E1529: sessions started outside AI Juggler show under their project.
  const scope = outsideScope(filters, parsed.text);
  // AJ-E2103: the default view reads Needs you, Open now, then "Also:".
  // AJ-E2104: the Project lens with Show › Ended on is the whole board,
  // every session under its project (its Also line's parts stay its own).
  const order = quietView(effectiveGroup, filters, parsed.text) && !view.show.ended ? railOrder(sorted, snooze, now) : null;
  const groups = openGroups(order, withOutsideGroups(sessionGroups, conversations, projects, scope), conversations, scope, new Set(projects.filter((p) => p.pinned).map((p) => p.id)));
  const shownRows = order?.shown ?? ((rows: InstanceInfo[]) => rows);
  // D-25: a project with no sessions still has a place in the rail (one
  // click opens it) — in the plain project view, not while filtering.
  const quiet = quietView(effectiveGroup, filters, parsed.text);
  const idleProjects = quiet && onSelectProject ? projects.filter((p) => !groups.some((group) => group.id === p.id)) : [];
  const byProject = view.group === "project";

  const boardRow = (row: InstanceInfo) => <BoardRow key={row.id} row={row} showProject selected={row.id === selectedId} onSelect={onSelect} density={effectiveDensity} freshness={freshness} now={now} snooze={snooze} />;
  // Windowing (Req.7): only the flat, ungrouped list virtualises — the
  // 200-row scale is ungrouped monitoring; grouped triage views stay
  // fully rendered. Every row is reachable by scroll in both modes.
  const flatRows = groups.length === 1 ? shownRows(groups[0]?.rows ?? []) : null;
  let windowed: { start: number; end: number } | null = null;
  if (flatRows && flatRows.length > VIRTUALISE_AT) {
    const rowH = ROW_HEIGHT[effectiveDensity];
    const start = Math.max(0, Math.floor(scrollTop / rowH) - 4);
    const end = Math.min(flatRows.length, Math.ceil((scrollTop + viewportH) / rowH) + 4);
    windowed = { start, end };
  }

  // AJ-E1503: every snooze, mute and unsnooze says so, with Undo (the
  // row's earlier state comes back).
  const withUndo = (row: InstanceInfo, message: string) => {
    const before = snooze[row.id];
    showToast({
      id: `snooze:${row.id}`,
      message,
      undo: {
        onUndo: () =>
          onSnooze((current) => {
            const restored = { ...current };
            if (before) restored[row.id] = before;
            else delete restored[row.id];
            return restored;
          }),
        done: "Undone.",
      },
    });
  };
  const snoozeFor = (row: InstanceInfo, ms: number | null, muted = false) => {
    const label = sessionLabel(row, projects);
    withUndo(row, muted ? `Muted ${label}.` : `Snoozed ${label} for ${durationWords(ms ?? 0)}.`);
    onSnooze((current) => ({
      ...current,
      [row.id]: muted ? { until: null, muted: true } : { until: ms === null ? null : Date.now() + ms, muted: false },
    }));
  };
  const clearSnooze = (row: InstanceInfo) => {
    withUndo(row, `${sessionLabel(row, projects)} is back on the list.`);
    onSnooze((current) => {
      const next = { ...current };
      delete next[row.id];
      return next;
    });
  };
  // AJ-E1537: what the rail gives every session and project menu.
  const host: ActionHost = {
    ...sessionManage.host,
    ...projectManage.host,
    openSession: (row) => onSelect(row.id),
    openProject: onSelectProject ? (project) => onSelectProject(project.id) : undefined,
    isSnoozed: (row) => isSnoozed(row, snooze, now),
    // "Until reset" is null: the window's reset when known, else a day.
    snoozeSession: (row, ms) => {
      const resetAt = row.quota?.resets_at_ms ?? null;
      snoozeFor(row, ms ?? (resetAt !== null ? Math.max(0, resetAt - now) : 24 * 60 * 60_000));
    },
    muteSession: (row) => snoozeFor(row, null, true),
    unsnoozeSession: clearSnooze,
  };

  return (
    <ActionHostProvider host={host}>
    <div className="flex min-h-0 flex-1 flex-col gap-2" data-testid="instance-board">
      {/* AJ-E2104: search (the View's words too) and the View. */}
      <RailToolbar query={query} onQuery={setQuery} searchRef={searchRef} sessions={sessions} projects={projects} machines={remoteMachines} />

      {/* Sign-ins ride above the board on the Sessions tab. */}
      {loginTerminals.length > 0 && (
        <ListSection label="Sign-ins">
          {loginTerminals.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => onSelect(t.id)}
              className={
                "relative flex min-w-0 w-full items-center gap-2 rounded-lg py-[7px] pl-3 pr-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (t.id === selectedId ? "bg-surface" : "hover:bg-surface/60")
              }
            >
              {t.id === selectedId && (
                <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
              )}
              <span className={"size-1.5 shrink-0 rounded-full " + (t.ended ? "bg-faint" : "bg-live animate-breathe")} />
              <span className="min-w-0 flex-1 truncate text-meta">{t.title}</span>
            </button>
          ))}
        </ListSection>
      )}

      <div
        ref={scrollRef}
        className="rail-scroll min-h-0 flex-1 overflow-y-auto overflow-x-hidden"
        onScroll={(event) => setScrollTop((event.target as HTMLDivElement).scrollTop)}
      >
        {hasRemote && showLocal && (
          <p className="px-3 pb-0.5 pt-1 text-meta font-semibold text-muted" data-testid="board-machine-local">
            This Mac
          </p>
        )}
        {showLocal && !byProject && (loading && sessions.length === 0 ? <RowPlaceholders shape="rail" rows={4} label="Loading sessions" /> :
          <PlaceBoard sessions={sessions} projects={projects} snooze={snooze} now={now} selectedId={selectedId} onSelect={onSelect} query={query} scroll={{ top: scrollTop, height: viewportH, box: scrollRef.current }} />
        )}
        {byProject && <>
        {showLocal && order && <NeedsYouSection rows={order.needs} selectedId={selectedId} density={effectiveDensity} now={now} renderRow={boardRow} />}
        {!showLocal ? null : loading && sessions.length === 0 ? (
          <RowPlaceholders shape="rail" rows={4} label="Loading sessions" />
        ) : sorted.length === 0 && idleProjects.length === 0 && !groups.some((g) => railSlice(conversations, g.id, scope).shown.length > 0) ? (
          sessions.length === 0 ? (
            /* D-25: empty means "start here" — the legend lives behind
               the footer's "?". */
            <div className="flex flex-col gap-1.5 px-3 py-6" data-testid="rail-empty">
              <p className="text-label font-medium text-ink">No sessions yet</p>
              <p className="text-label leading-relaxed text-muted">Start one with New session.</p>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-2 px-2 py-8 text-center">
              <p className="text-meta font-medium text-muted">No sessions match</p>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  setQuery("");
                  searchRef.current?.focus();
                }}
              >
                Clear filters
              </Button>
            </div>
          )
        ) : flatRows && windowed ? (
          <div className="flex flex-col gap-0.5">
            <div style={{ height: windowed.start * ROW_HEIGHT[effectiveDensity] }} />
            {flatRows.slice(windowed.start, windowed.end).map((row) => (
              <BoardRow
                key={row.id}
                row={row}
                showProject
                selected={row.id === selectedId}
                onSelect={onSelect}
                density={effectiveDensity}
                freshness={freshness}
                now={now}
                snooze={snooze}
              />
            ))}
            <div style={{ height: (flatRows.length - windowed.end) * ROW_HEIGHT[effectiveDensity] }} />
            <p className="px-3 pt-1 text-center text-label text-muted">
              {flatRows.length} sessions
            </p>
          </div>
        ) : (
          [order && groups.length > 0 && <OpenNowLabel key="open-now" open={!folded.has(OPEN_NOW)} onToggle={() => toggleFold(OPEN_NOW)} />, ...(order && folded.has(OPEN_NOW) ? [] : groups).map((entry) => (
            <div key={entry.id} className="mb-1 flex flex-col gap-0.5">
              {entry.label !== "" && (
                <div className="flex min-w-0 items-center gap-0.5 pt-1.5">
                  <FoldButton
                    open={!folded.has(entry.id)}
                    label={entry.label}
                    controls={safeId("rail-group", entry.id)}
                    onToggle={() => toggleFold(entry.id)}
                  />
                  {effectiveGroup === "project" && onSelectProject && projects.some((p) => p.id === entry.id) ? (
                    <RailProjectHeader
                      project={projects.find((p) => p.id === entry.id)!}
                      rows={entry.rows}
                      label={entry.label}
                      rollup={entry.rollup}
                      count={entry.rows.length + railSlice(conversations, entry.id, scope).shown.length}
                      selected={entry.id === selectedId}
                      onSelect={() => onSelectProject(entry.id)}
                    >
                      {folded.has(entry.id) && <FoldedNeeds rows={entry.rows} snooze={snooze} now={now} />}
                    </RailProjectHeader>
                  ) : (
                    <p
                      className="flex min-w-0 flex-1 items-baseline justify-between gap-2 py-0.5 pl-1 pr-2.5 text-meta font-semibold text-muted"
                      title={entry.rollup || undefined}
                    >
                      <span className="min-w-0 truncate">{entry.label}</span>
                      {entry.rollup && <span className="sr-only">· {entry.rollup}</span>}
                      <span className="flex shrink-0 items-center gap-1.5">
                        {folded.has(entry.id) && <FoldedNeeds rows={entry.rows} snooze={snooze} now={now} />}
                        <span className="text-label font-normal text-muted">{entry.rows.length}</span>
                      </span>
                    </p>
                  )}
                </div>
              )}
              {/* AJ-E1532: a folded project's rows, as a stripe in its colour. */}
              {folded.has(entry.id) && markOf(entry.id) && (
                <span aria-hidden="true" data-testid="rail-folded-stripe" className="ml-7 mr-2.5 h-[3px] rounded-full opacity-70" style={{ background: swatchColor(markOf(entry.id)!.swatch) }} />
              )}
              <div id={safeId("rail-group", entry.id)} className="flex flex-col gap-0.5" hidden={folded.has(entry.id)}>
              {shownRows(entry.rows).map((row) => (
                <BoardRow
                  key={row.id}
                  row={row}
                  showProject={effectiveGroup !== "project"}
                  selected={row.id === selectedId}
                  onSelect={onSelect}
                  density={effectiveDensity}
                  freshness={freshness}
                  now={now}
                  snooze={snooze}
                  />
              ))}
              <OutsideRows projectId={entry.id} selectedId={selectedId} density={effectiveDensity} now={now} scope={scope} />
              </div>
            </div>
          ))]
        )}
        {showLocal && order && <AlsoLine ended={order.endedToday} renderRow={boardRow} selectedId={selectedId} density={effectiveDensity} now={now} />}
        {idleProjects.length > 0 && (
          /* Projects with no session here yet (from the setups' history):
             the same rows as sessions, a folder mark instead of a state,
             and what was done there instead of a state line. Clones of one
             repository group like sessions group under a project. */
          <div className="mt-1 flex flex-col gap-0.5" data-testid="rail-history-projects">
            <div className="flex min-w-0 items-center gap-0.5 pt-1.5">
              <FoldButton
                open={!folded.has(HISTORY_GROUP)}
                label="Projects"
                controls="rail-history-list"
                onToggle={() => toggleFold(HISTORY_GROUP)}
              />
              <p className="flex min-w-0 flex-1 items-baseline justify-between gap-2 py-0.5 pl-1 pr-2.5 text-meta font-semibold text-muted">
                <span className="min-w-0 truncate">Projects</span>
                <span className="shrink-0 text-label font-normal text-muted">{repoGroups(idleProjects).length}</span>
              </p>
            </div>
            <div id="rail-history-list" className="flex flex-col gap-0.5" hidden={folded.has(HISTORY_GROUP)}>
              {repoGroups(idleProjects)
                .slice(0, allProjects ? undefined : HISTORY_SHOWN)
                .map((group) =>
                  group.projects.length === 1 ? (
                    <ProjectRow
                      key={group.key}
                      project={group.projects[0]!}
                      density={effectiveDensity === "compact" ? "compact" : "comfortable"}
                      now={now}
                      selected={group.projects[0]!.id === selectedId}
                      onSelect={(id) => onSelectProject?.(id)}
                    />
                  ) : (
                    <div key={group.key} className="flex flex-col gap-0.5" data-testid="rail-repo-group">
                      <div className="flex min-w-0 items-center gap-0.5 pl-3">
                        <FoldButton
                          open={!folded.has(group.key)}
                          label={group.name}
                          controls={safeId("rail-repo", group.key)}
                          onToggle={() => toggleFold(group.key)}
                        />
                        <p className="flex min-w-0 flex-1 items-center justify-between gap-2 py-0.5 pl-1 pr-2.5 text-meta font-semibold text-muted" title={group.key}>
                          <span className="flex min-w-0 items-center gap-1.5">
                            <ProjectMark projectId={group.projects[0]!.id} size={14} />
                            <span className="min-w-0 truncate">{group.name}</span>
                          </span>
                          <span className="shrink-0 text-label font-normal text-muted">{group.projects.length}</span>
                        </p>
                      </div>
                      <div id={safeId("rail-repo", group.key)} className="flex flex-col gap-0.5 pl-3" hidden={folded.has(group.key)}>
                        {group.projects.map((project) => (
                          <ProjectRow
                            key={project.id}
                            project={project}
                            density={effectiveDensity === "compact" ? "compact" : "comfortable"}
                            now={now}
                            selected={project.id === selectedId}
                            onSelect={(id) => onSelectProject?.(id)}
                                />
                        ))}
                      </div>
                    </div>
                  ),
                )}
              {repoGroups(idleProjects).length > HISTORY_SHOWN && (
                <button
                  type="button"
                  onClick={() => setAllProjects((open) => !open)}
                  className="self-start rounded-md py-1 pl-7 pr-2 text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
                  aria-expanded={allProjects}
                  data-testid="rail-history-all"
                >
                  {allProjects ? "Show fewer" : `Show all ${repoGroups(idleProjects).length}`}
                </button>
              )}
            </div>
          </div>
        )}
        </>}
        <OtherMacs machines={remoteShown} query={parsed.text} unfolded={view.show.otherMacs || (mac !== undefined && mac !== "this-mac")} selected={selectedRemote} onSelect={(machineId, instanceId) => onSelectRemote?.(machineId, instanceId)} />
      </div>

      {sessionManage.element}
      {projectManage.element}
    </div>
    </ActionHostProvider>
  );
}

function BoardRow({
  row,
  showProject = false,
  selected,
  onSelect,
  density,
  freshness,
  now,
  snooze,
}: {
  row: InstanceInfo;
  /** Not grouped by project: the row shows its project's mark. */
  showProject?: boolean;
  selected: boolean;
  onSelect: (id: string) => void;
  density: "comfortable" | "compact";
  freshness: Freshness;
  now: number;
  snooze: SnoozeState;
}) {
  const caption = snoozeCaption(row, snooze, now);
  // AJ-E1537: right-click, ⋯ or Shift+F10 — the session's one menu.
  const menu = useItemMenu("session", row, sessionTitle(row));
  // Its ↗ while it runs in another app's window (always shown when it needs you).
  const where = goToWhere(sessionItem(row, () => null, false)), needs = row.needs_attention !== null && !isSnoozed(row, snooze, now);
  return (
    <div className="group relative" {...menu.triggerProps}>
      {caption && (
        <span className="absolute -top-0.5 right-2 z-10 rounded bg-surface-3 px-1 text-label font-medium text-muted" title={caption}>
          {caption}
        </span>
      )}
      <RailRow session={row} showProject={showProject} selected={selected} onSelect={onSelect} density={density} now={now} freshness={freshness} trailing={where !== null && needs} />
      {where !== null && menu.goTo({ where, shown: needs, className: "absolute right-1 top-1/2 -translate-y-1/2" })}
      {/* I-03: a model that drifted from its launch settings is worth a
          glance (cost and behaviour differ) — beside the row, focusable,
          clear of the tool's mark and the avatar. Compact rows
          have no room beside the time; the session view says it there. */}
      {density === "comfortable" && modelDrift(row) && (
        <span
          title={modelDrift(row)!}
          aria-label={modelDrift(row)!}
          tabIndex={0}
          data-testid="model-drift"
          className="absolute right-12 top-1.5 z-10 rounded-full border border-warning/25 bg-warning-soft px-1 text-label font-medium text-warning outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          ⇄
        </span>
      )}
      {/* The ⋯ button for pointer-free access: 24 px square, the WCAG
          2.2 minimum target (AJ-E804). */}
      {menu.button({ label: `Session actions for ${sessionTitle(row)}`, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}

/**
 * D-25 — the rail row: a state shape, what it is (the branch, else the
 * folder), one line of state and time, and the Account's colour. Quota
 * shows only when it's low (25 % or less left). Everything else — model,
 * tokens, processes, resources — is one click away in the session view.
 * Comfortable is two lines; compact is one.
 */
export function RailRow({
  session,
  showProject = false,
  selected,
  onSelect,
  density,
  now,
  freshness = {},
  trailing = false,
}: {
  /** Room for an always-shown ↗ at its end. */
  trailing?: boolean;
  session: InstanceInfo;
  showProject?: boolean;
  selected: boolean;
  onSelect: (id: string) => void;
  density: "comfortable" | "compact";
  now: number;
  freshness?: Freshness;
}) {
  const chip = stateChip(session.status);
  // Never crash on a missing folder (it names the row only as a fallback).
  const cwd = session.cwd ?? "";
  const folder = cwd.replace(/\/+$/, "").split("/").pop() || cwd || session.account_label;
  // AJ-E1502: the user's name for it shows over its own title.
  const title = session.label ?? session.git_branch ?? folder;
  const ended = ["finished", "stopped", "handed_over"].includes(session.status);
  const parked = parkedWords(session, now);
  // AJ-E2104 Req.3: the place rows' words (Asks you, Working, Idle,
  // Ended; why it needs you when it does), the glyph a detail beside them.
  const said = sessionState(session);
  const state = parked ? parked.chip : said.word;
  const low = lowLeftWords(session);
  const when = sessionWhen(session, now);
  // D-25: the row stays calm — these markers appear only when they are
  // the exception worth a glance (a sign-in to check, uncommitted work,
  // a value that stopped updating); the session view has the rest.
  // E14 (AJ-E1405): resources show only when the session is heavy.
  const heavyNote = useHeavyNote(session.id);
  const heavy = heavyNote ? shortHeavy(heavyNote) : null;
  const sampled = freshness[session.id];
  const stale = sampled && isActiveStatus(session.status)
    ? ((session.quota !== null ? staleCaption(sampled.quota, REFRESH_INTERVALS.quota, now) : null) ??
      (session.cpu_pct !== null ? staleCaption(sampled.resources, REFRESH_INTERVALS.resources, now) : null))
    : null;
  const Icon = chip.Icon;
  return (
    <button
      type="button"
      data-instance-id={session.id}
      aria-current={selected ? "true" : undefined}
      data-rail-selected={selected ? "true" : undefined}
      onClick={() => onSelect(session.id)}
      className={
        "relative flex w-full min-w-0 items-center gap-2.5 rounded-lg pl-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (trailing ? "pr-9 " : "pr-2 ") +
        (density === "compact" ? "py-1.5 " : "py-2 ") +
        (selected ? "bg-surface" : "hover:bg-surface/60") +
        // Ended rows are quieter by their title colour, never by opacity:
        // faded small text would fall below 4.5:1 (WCAG 1.4.3).
        ""
      }
    >
      {selected && <span aria-hidden="true" className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />}
      <span aria-hidden="true" data-state-glyph={said.state} className={"flex w-4 shrink-0 justify-center " + said.tone}>
        <Icon width={13} height={13} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="flex min-w-0 items-baseline gap-1.5">
          {showProject && session.project_id && <ProjectMark projectId={session.project_id} size={12} />}
          <span className={"min-w-0 truncate text-meta font-medium " + (ended ? "text-muted" : "text-ink")}>{title}</span>
          {session.pinned && <PinMark />}
          {session.kind === "desktop" && (
            <span title="Runs in the Claude desktop app" data-testid="rail-app-chip" className="shrink-0 rounded border border-hairline px-1 text-label font-medium text-muted">
              app
            </span>
          )}
          {density === "compact" && <span className="ml-auto shrink-0 text-label tabular-nums text-muted">{when}</span>}
        </span>
        {density === "comfortable" && (
          <span className="flex min-w-0 items-center gap-1 overflow-hidden text-label text-muted">
            <span
              title={parked ? parked.long : attentionTooltip(session)}
              data-testid={parked ? "rail-parked" : "rail-state-word"}
              className={"shrink-0 whitespace-nowrap " + (parked ? "text-ink" : said.tone)}
            >
              {state}
            </span>
            {low && <span className="shrink-0 whitespace-nowrap text-warning">· {low}</span>}
            {heavy && (
              <span title={`Working your Mac hard: ${heavyNote}`} data-testid="rail-heavy" className="shrink-0 whitespace-nowrap text-warning">
                · {heavy}
              </span>
            )}
            <RailQuietNotes session={session} />
            <StaleChip caption={stale} />
            {/* The time yields first: the notes above matter more. */}
            <span className="min-w-0 truncate">· {when}</span>
          </span>
        )}
        <span className="sr-only">
          {session.pinned ? ", pinned" : ""}, {state}, {providerDisplayName(session.provider)} on {session.account_label}
          {low ? `, ${low}` : ""}
          {heavyNote ? `, working your Mac hard: ${heavyNote}` : ""}
        </span>
      </span>
      {density === "compact" && low && <span className="shrink-0 text-label tabular-nums text-warning">{low}</span>}
      {density === "compact" && !low && heavy && (
        <span data-testid="rail-heavy" className="shrink-0 font-mono text-label text-warning">
          {heavy}
        </span>
      )}
      {/* AJ-E1532 with D-38: the tool (its own app's icon), and the account serving it now (its avatar
          follows the session when it switches accounts). */}
      <AppIcon {...appOfSession(session)} size={16} />
      <AccountAvatar accountId={session.account_id} label={session.account_label} size={16} />
    </button>
  );
}

/** D-25: the plain project view — no filter, no search. */
function quietView(group: GroupBy, filters: ChipFilters, query: string): boolean {
  return group === "project" && filters.chips.length === 0 && filters.provider === null && filters.account === null && filters.project === null && query.trim() === "";
}

function ListSection({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <p className="px-3 pb-0.5 pt-1 text-meta font-semibold text-muted">
        {label}
      </p>
      {children}
    </div>
  );
}

/**
 * Minimal density (AJ-E403 Req.2): the 36 px icon column — avatar disc +
 * state glyph + `!` badge per Instance. The full I-01..I-11 text rides
 * in the tooltip (CSS hover panel) and the `aria-label` (Req.3).
 */
export function MinimalInstanceColumn({
  sessions,
  selectedId,
  onSelect,
}: {
  sessions: InstanceInfo[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  // Active rows only — waiting/stuck/quota-blocked instances are the
  // ones that must scream `!` in the collapsed rail (user story).
  const active = sessions.filter((s) => isActiveStatus(s.status));
  const needsYou = active.filter((s) => s.needs_attention !== null).length;
  const railId = useId();
  return (
    <div className="flex flex-col items-center gap-1 pt-1">
      {active.map((s) => {
        const chip = stateChip(s.status);
        const badge = attentionBadge(s);
        const said = sessionState(s); // AJ-E2104 Req.3: the rail's words
        return (
          <button
            key={s.id}
            type="button"
            data-instance-id={s.id}
            onClick={() => onSelect(s.id)}
            aria-describedby={`${railId}-${s.id}`}
            title={ariaSummary(s)}
            className={
              "group relative flex size-9 items-center justify-center rounded-lg outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
              (s.id === selectedId ? "bg-surface" : "hover:bg-surface")
            }
          >
            {/* Icon-only: nothing else is shown, so the description is the full summary. */}
            <span id={`${railId}-${s.id}`} hidden>
              {ariaSummary(s)}
            </span>
            {/* WCAG 2.5.3: an icon-only button's name, in words. */}
            <span className="sr-only">
              {s.account_label}, {said.word}
            </span>
            <AccountAvatar accountId={s.account_id} label={s.account_label} size={22} />
            <chip.Icon
              width={10}
              height={10}
              className={"absolute bottom-0 right-0 rounded-full bg-surface p-px " + said.tone}
            />
            {badge ? (
              <span
                className={
                  "absolute right-0 top-0 flex size-3.5 items-center justify-center rounded-full text-label font-bold " +
                  (badge.tone === "danger"
                    ? "bg-danger text-white"
                    : "bg-warning text-black")
                }
                aria-hidden="true"
              >
                !
              </span>
            ) : null}
            {/* Hover/focus tooltip: the full I-01..I-11 field text for THIS
                instance (Req.3) — never a bare disc. */}
            <span aria-hidden="true" className="pointer-events-none absolute left-full top-0 z-30 ml-2 hidden w-56 rounded-lg border border-hairline bg-surface p-2.5 text-left shadow-card group-hover:block group-focus-visible:block">
              <span className="flex items-center gap-1.5">
                <span className="truncate text-meta font-medium text-ink">
                  {s.account_label}
                </span>
                <AccountAvatar accountId={s.account_id} label={s.account_label} size={16} />
              </span>
              <span className="mt-1 block text-label leading-relaxed text-muted">
                {ariaSummary(s)}
              </span>
            </span>
          </button>
        );
      })}
      {needsYou > 0 && (
        <span
          className="mt-1 rounded-full border border-warning/40 bg-warning-soft px-1.5 py-0.5 text-label font-bold text-warning"
          title={`${needsYou} session${needsYou === 1 ? "" : "s"} need you`}
        >
          ! {needsYou}
        </span>
      )}
      {active.length === 0 && <RailCalm />}
    </div>
  );
}

