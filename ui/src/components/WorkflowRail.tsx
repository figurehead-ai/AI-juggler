import { AccountAvatar } from "./Avatar";
import { ProjectMark } from "./ProjectMark";
import { ToolMark } from "./ToolMark";
import { markOf, swatchColor } from "../lib/palette";
import { identityWords } from "../lib/identity";
import { RailQuietNotes } from "./RailNotes";
import { parkedWords } from "../lib/parked";
import { providerDisplayName } from "../lib/providers";
import { ReactNode, useCallback, useEffect, useId, useRef, useState, type JSX } from "react";
import { IconProjects, IconTerminal } from "./icons";
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
import type { AccountInfo, InstanceInfo, ProjectInfo, RemoteInstanceInfo, RemoteMachineInfo } from "../lib/bridge";
import { shortHeavy, useHeavyNote } from "../lib/machine";
import type { Density } from "../lib/density";
import { REFRESH_INTERVALS, staleCaption } from "../lib/density";
import {
  GROUP_LABELS,
  NO_CHIP_FILTERS,
  SORT_LABELS,
  isSnoozed,
  readSnooze,
  snoozeCaption,
  sortSessions,
  storeSnooze,
  chipCounts,
  filterSessions,
  groupSessions,
  type ChipFilters,
  type ChipId,
  type GroupBy,
  type RowGroup,
  type SnoozeState,
  type SortMode,
} from "../lib/railview";
import { Button } from "./primitives";
import { sessionTitle, useProjectManage, useSessionManage } from "./ManageWork";
import { ActionHostProvider, useItemMenu } from "./ItemMenu";
import { OutsideRows, outsideScope, railSlice, withOutsideGroups } from "./OutsideConversations";
import { AlsoLine, NeedsYouSection, OpenNowLabel, openGroups, railOrder } from "./RailSections";
import { FilterChip, Select } from "./RailControls";
import { liveOutside, useConversations } from "../lib/conversations";
import { sessionWhen } from "../lib/time-words";
import type { ActionHost } from "../lib/actions";
import {
  ChildBadges,
  GitBadge,
  StaleChip,
  STATE_WORDS,
  attentionBadge,
  attentionSuffix,
  attentionTooltip,
  isActiveStatus,
  lowLeftWords,
  providerMark,
  rememberResourceSample,
  stateChip,
  tokenSummary,
  ageSummary,
  formatBytes,
  type StateChipTone,
} from "./indicator";
import { QuotaBar, Sparkline } from "./primitives";
import { ariaSummary, RowDetails } from "./row-details";

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
          data-testid="rail-new-session"
          className="flex min-w-0 flex-1 items-center justify-center gap-2 overflow-hidden rounded-lg border border-accent/30 bg-accent-soft px-3 py-[7px] text-[12px] font-medium text-accent-hi outline-none transition-colors hover:border-accent/50 hover:bg-accent/15 focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <IconTerminal width={14} height={14} />
          {!minimal && <span className="truncate">New session</span>}
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
  const [sort, setSort] = useState<SortMode>("needs");
  // D-25: one rail — sessions grouped under their project by default.
  const [group, setGroup] = useState<GroupBy>("project");
  // D-25: the quick filters show; the rest wait behind Filters.
  const [moreFilters, setMoreFilters] = useState(false);
  const [filters, setFilters] = useState<ChipFilters>(NO_CHIP_FILTERS);
  const [query, setQuery] = useState("");
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
  // AJ-E709 Req.2: machines filter the board; "All machines" is the
  // default and the choice persists across restarts.
  const [machineFilter, setMachineFilter] = useState<string>(() => {
    try {
      return localStorage.getItem(MACHINE_FILTER_KEY) ?? "all";
    } catch {
      return "all";
    }
  });
  const chooseMachine = (value: string) => {
    setMachineFilter(value);
    try {
      localStorage.setItem(MACHINE_FILTER_KEY, value);
    } catch {
      // Not persisted this time; the filter still applies.
    }
  };
  const hasRemote = remoteMachines.length > 0;
  const machineChoice =
    machineFilter === "all" ||
    machineFilter === "this-mac" ||
    remoteMachines.some((machine) => machine.machine_id === machineFilter)
      ? machineFilter
      : "all";
  const showLocal = !hasRemote || machineChoice === "all" || machineChoice === "this-mac";
  const remoteShown = hasRemote
    ? remoteMachines.filter((machine) => machineChoice === "all" || machineChoice === machine.machine_id)
    : [];
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
  const filtered = filterSessions(sessions, filters, query, projects);
  const sorted = sortSessions(filtered, sort, snooze, now, projects, accounts);
  // Quota-risk groups the unknowns under `no data` (Req.2); other sorts
  // group only when the user picked a Group-by.
  const effectiveGroup: GroupBy | "quota-data" =
    sort === "quota" && group === "none" ? "quota-data" : group;
  const sessionGroups =
    effectiveGroup === "quota-data"
      ? quotaDataGroups(sorted)
      : groupSessions(sorted, effectiveGroup === "none" ? "none" : effectiveGroup, projects);
  // AJ-E1529: sessions started outside AI Juggler show under their project.
  const scope = effectiveGroup === "project" ? outsideScope(filters, query) : "none";
  // AJ-E2103: the default view reads Needs you, Open now, then "Also:".
  const order = quietView(effectiveGroup, filters, query) && sort === "needs" ? railOrder(sorted, snooze, now) : null;
  const groups = openGroups(order, withOutsideGroups(sessionGroups, conversations, projects, scope), conversations, scope);
  const shownRows = order?.shown ?? ((rows: InstanceInfo[]) => rows);
  // AJ-E1529: the Running chip counts sessions started outside AI Juggler.
  const counts = { ...chipCounts(sessions), running: chipCounts(sessions).running + liveOutside(conversations).length };
  // D-25: a project with no sessions still has a place in the rail (one
  // click opens it) — in the plain project view, not while filtering.
  const quiet = quietView(effectiveGroup, filters, query);
  const idleProjects = quiet && onSelectProject ? projects.filter((p) => !groups.some((group) => group.id === p.id)) : [];

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
      {/* Toolbar: search ⌘K, filter chips with counts, Sort/Group + key
          dropdowns (native selects: accessible + keyboard in a narrow rail). */}
      <div className="flex flex-col gap-1.5">
        <div className="relative">
          <span className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-faint">⌕</span>
          <input
            ref={searchRef}
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                setQuery("");
                searchRef.current?.focus();
              }
            }}
            placeholder="Search sessions ⌘K"
            aria-label="Search sessions by account, project, branch, model or tool"
            className="w-full rounded-lg border border-hairline bg-surface/70 py-1.5 pl-6 pr-2 text-[11.5px] text-ink outline-none placeholder:text-faint focus-visible:ring-1 focus-visible:ring-accent/60"
          />
        </div>
        {/* Wraps rather than clip Filters when the counts don't fit the
            rail's width (0.0.11 walk: "Needs you 5 · Running 3" at 208 px). */}
        <div className="flex min-w-0 flex-wrap items-center gap-1" role="group" aria-label="Show">
          <FilterChip
            active={filters.chips.includes("needs")}
            onClick={() => setFilters((current) => ({ ...current, chips: current.chips.includes("needs") ? [] : ["needs"] }))}
            count={counts.needs}
          >
            Needs you
          </FilterChip>
          <FilterChip
            active={filters.chips.includes("running")}
            onClick={() => setFilters((current) => ({ ...current, chips: current.chips.includes("running") ? [] : ["running"] }))}
            count={counts.running}
          >
            Running
          </FilterChip>
          <FilterChip active={filters.chips.length === 0} onClick={() => setFilters((current) => ({ ...current, chips: [] }))}>
            All
          </FilterChip>
          <button
            type="button"
            aria-expanded={moreFilters}
            aria-label={narrowed(filters, sort, group, machineChoice) ? "Filters (some on)" : "Filters"}
            title="Filters"
            onClick={() => setMoreFilters((open) => !open)}
            className={
              "relative ml-auto flex size-6 shrink-0 items-center justify-center rounded-md outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
              (moreFilters || narrowed(filters, sort, group, machineChoice) ? "bg-accent-soft text-accent-hi" : "text-muted hover:bg-surface hover:text-ink")
            }
          >
            <svg aria-hidden="true" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
              <path d="M4 6h16M7 12h10M10 18h4" />
            </svg>
            {narrowed(filters, sort, group, machineChoice) && (
              <span aria-hidden="true" className="absolute right-0.5 top-0.5 size-1.5 rounded-full bg-accent" />
            )}
          </button>
        </div>
        {moreFilters && (
        <div className="flex flex-col gap-1 rounded-lg border border-hairline/60 bg-surface/40 p-1.5 [&>select]:w-full" data-testid="rail-more-filters">
          <div className="flex items-center justify-between gap-1">
            <FilterChip active={filters.chips.includes("ended")} onClick={() => toggleChip(setFilters, "ended")} count={counts.ended}>
              Ended
            </FilterChip>
            {narrowed(filters, sort, group, machineChoice) && (
              <button
                type="button"
                data-testid="rail-filters-reset"
                onClick={() => {
                  setFilters(NO_CHIP_FILTERS);
                  setSort("needs");
                  setGroup("project");
                  if (machineChoice !== "all") chooseMachine("all");
                }}
                className="rounded px-1.5 py-0.5 text-[10px] text-muted underline decoration-hairline underline-offset-2 outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
              >
                Reset
              </button>
            )}
          </div>
          <Select ariaLabel="Sort by" value={sort} onChange={(value) => setSort(value as SortMode)} options={SORT_LABELS.map((entry) => ({ value: entry.id, label: `Sort: ${entry.label}` }))} />
          <Select ariaLabel="Group by" value={group} onChange={(value) => setGroup(value as GroupBy)} options={GROUP_LABELS.map((entry) => ({ value: entry.id, label: `Group: ${entry.label}` }))} />
          <Select
            ariaLabel="Filter by tool"
            value={filters.provider ?? ""}
            onChange={(value) => setFilters((current) => ({ ...current, provider: value || null }))}
            options={[{ value: "", label: "Tool: all" }, ...unique(sessions, (row) => row.provider).map((value) => ({ value, label: providerDisplayName(value) }))]}
          />
          <Select
            ariaLabel="Filter by account"
            value={filters.account ?? ""}
            onChange={(value) => setFilters((current) => ({ ...current, account: value || null }))}
            options={[{ value: "", label: "Account: all" }, ...unique(sessions, (row) => row.account_id).map((value) => ({ value, label: accountLabelOf(sessions, value) }))]}
          />
          {hasRemote && (
            <Select
              ariaLabel="Filter by machine"
              value={machineChoice}
              onChange={chooseMachine}
              options={[
                { value: "all", label: "Machine: all" },
                { value: "this-mac", label: "This Mac" },
                ...remoteMachines.map((machine) => ({ value: machine.machine_id, label: machine.label })),
              ]}
            />
          )}
          <Select
            ariaLabel="Filter by project"
            value={filters.project ?? ""}
            onChange={(value) => setFilters((current) => ({ ...current, project: value || null }))}
            options={[{ value: "", label: "Project: all" }, ...projects.map((project) => ({ value: project.id, label: project.name }))]}
          />
        </div>
        )}
      </div>

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
              <span className="min-w-0 flex-1 truncate text-[12.5px]">{t.title}</span>
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
          <p className="px-3 pb-0.5 pt-1 text-[10px] font-medium uppercase tracking-[0.08em] text-muted" data-testid="board-machine-local">
            This Mac
          </p>
        )}
        {showLocal && order && <NeedsYouSection rows={order.needs} selectedId={selectedId} density={effectiveDensity} now={now} renderRow={boardRow} />}
        {!showLocal ? null : loading && sessions.length === 0 ? (
          <RowPlaceholders shape="rail" rows={4} label="Loading sessions" />
        ) : sorted.length === 0 && idleProjects.length === 0 && !groups.some((g) => railSlice(conversations, g.id, scope).shown.length > 0) ? (
          sessions.length === 0 ? (
            /* D-25: empty means "start here" — the legend lives behind
               the footer's "?". */
            <div className="flex flex-col gap-1.5 px-3 py-6" data-testid="rail-empty">
              <p className="text-[12px] font-medium text-ink">No sessions yet</p>
              <p className="text-[11px] leading-relaxed text-muted">Start one with New session.</p>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-2 px-2 py-8 text-center">
              <p className="text-[11.5px] font-medium text-muted">No sessions match</p>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  setFilters(NO_CHIP_FILTERS);
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
            <p className="px-3 pt-1 text-center text-[9px] text-faint">
              {flatRows.length} sessions
            </p>
          </div>
        ) : (
          [order && groups.length > 0 && <OpenNowLabel key="open-now" />, ...groups.map((entry) => (
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
                      className="flex min-w-0 flex-1 items-baseline justify-between gap-2 py-0.5 pl-1 pr-2.5 text-[11.5px] font-semibold text-muted"
                      title={entry.rollup || undefined}
                    >
                      <span className="min-w-0 truncate">{entry.label}</span>
                      {entry.rollup && <span className="sr-only">· {entry.rollup}</span>}
                      <span className="flex shrink-0 items-center gap-1.5">
                        {folded.has(entry.id) && <FoldedNeeds rows={entry.rows} snooze={snooze} now={now} />}
                        <span className="text-[10px] font-normal text-faint">{entry.rows.length}</span>
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
              <p className="flex min-w-0 flex-1 items-baseline justify-between gap-2 py-0.5 pl-1 pr-2.5 text-[11.5px] font-semibold text-muted">
                <span className="min-w-0 truncate">Projects</span>
                <span className="shrink-0 text-[10px] font-normal text-faint">{repoGroups(idleProjects).length}</span>
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
                        <p className="flex min-w-0 flex-1 items-center justify-between gap-2 py-0.5 pl-1 pr-2.5 text-[11.5px] font-semibold text-muted" title={group.key}>
                          <span className="flex min-w-0 items-center gap-1.5">
                            <ProjectMark projectId={group.projects[0]!.id} size={14} />
                            <span className="min-w-0 truncate">{group.name}</span>
                          </span>
                          <span className="shrink-0 text-[10px] font-normal text-faint">{group.projects.length}</span>
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
                  className="self-start rounded-md py-1 pl-7 pr-2 text-[11px] text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
                  aria-expanded={allProjects}
                  data-testid="rail-history-all"
                >
                  {allProjects ? "Show fewer" : `Show all ${repoGroups(idleProjects).length}`}
                </button>
              )}
            </div>
          </div>
        )}
        {remoteShown.map((machine) => (
          <RemoteBoardSection
            key={machine.machine_id}
            machine={machine}
            rows={filterRemote(machine.instances, filters, query)}
            selected={selectedRemote}
            onSelect={(instanceId) => onSelectRemote?.(machine.machine_id, instanceId)}
          />
        ))}
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
  return (
    <div className="group relative" {...menu.triggerProps}>
      {caption && (
        <span className="absolute -top-0.5 right-2 z-10 rounded bg-surface-3 px-1 text-[8.5px] font-medium text-muted" title={caption}>
          {caption}
        </span>
      )}
      <RailRow session={row} showProject={showProject} selected={selected} onSelect={onSelect} density={density} now={now} freshness={freshness} />
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
          className="absolute right-12 top-1.5 z-10 rounded-full border border-warning/25 bg-warning-soft px-1 text-[10px] font-medium text-warning outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
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

/** D-25: what a rail row says about its state, in plain words. */
// D-25: the state words live in indicator.tsx (shared with the legend).
const RAIL_STATE = STATE_WORDS;


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
}: {
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
  const state = parked ? parked.chip : (RAIL_STATE[session.status] ?? chip.label);
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
        "relative flex w-full min-w-0 items-center gap-2.5 rounded-lg pl-2.5 pr-2 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (density === "compact" ? "py-1.5 " : "py-2 ") +
        (selected ? "bg-surface" : "hover:bg-surface/60") +
        // Ended rows are quieter by their title colour, never by opacity:
        // faded small text would fall below 4.5:1 (WCAG 1.4.3).
        ""
      }
    >
      {selected && <span aria-hidden="true" className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />}
      <span aria-hidden="true" className={"flex w-4 shrink-0 justify-center " + minimalGlyphTone(chip.tone)}>
        <Icon width={13} height={13} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="flex min-w-0 items-baseline gap-1.5">
          {showProject && session.project_id && <ProjectMark projectId={session.project_id} size={12} />}
          <span className={"min-w-0 truncate text-[12.5px] font-medium " + (ended ? "text-muted" : "text-ink")}>{title}</span>
          {session.pinned && <PinMark />}
          {session.kind === "desktop" && (
            <span title="Runs in the Claude desktop app" data-testid="rail-app-chip" className="shrink-0 text-[9.5px] font-medium uppercase tracking-wide text-muted">
              app
            </span>
          )}
          {density === "compact" && <span className="ml-auto shrink-0 font-mono text-[10px] text-muted">{when}</span>}
        </span>
        {density === "comfortable" && (
          <span className="flex min-w-0 items-center gap-1 overflow-hidden text-[11px] text-muted">
            <span
              title={parked ? parked.long : attentionTooltip(session)}
              data-testid={parked ? "rail-parked" : undefined}
              className={"shrink-0 whitespace-nowrap " + (parked ? "text-ink" : chip.tone === "warning" || chip.tone === "danger" ? minimalGlyphTone(chip.tone) : "")}
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
      {density === "compact" && low && <span className="shrink-0 font-mono text-[10px] text-warning">{low}</span>}
      {density === "compact" && !low && heavy && (
        <span data-testid="rail-heavy" className="shrink-0 font-mono text-[10px] text-warning">
          {heavy}
        </span>
      )}
      {/* AJ-E1532: the tool, and the account serving it now (its avatar
          follows the session when it switches accounts). */}
      <ToolMark tool={session.provider} size={12} className="shrink-0 text-muted" />
      <AccountAvatar accountId={session.account_id} label={session.account_label} size={16} />
    </button>
  );
}


/** Whether anything behind Filters narrows or reorders the list. */
/** D-25: the plain project view — no filter, no search. */
function quietView(group: GroupBy | "quota-data", filters: ChipFilters, query: string): boolean {
  return group === "project" && filters.chips.length === 0 && filters.provider === null && filters.account === null && filters.project === null && query.trim() === "";
}

function narrowed(filters: ChipFilters, sort: SortMode, group: GroupBy, machine: string): boolean {
  return (
    machine !== "all" ||
    filters.chips.includes("ended") ||
    filters.provider !== null ||
    filters.account !== null ||
    filters.project !== null ||
    sort !== "needs" ||
    group !== "project"
  );
}

function toggleChip(
  setFilters: React.Dispatch<React.SetStateAction<ChipFilters>>,
  chip: ChipId,
) {
  setFilters((current) => {
    const active = current.chips.includes(chip);
    return {
      ...current,
      chips: active
        ? current.chips.filter((entry) => entry !== chip)
        : [...current.chips, chip],
    };
  });
}

function unique(rows: InstanceInfo[], pick: (row: InstanceInfo) => string | null): string[] {
  return [...new Set(rows.map(pick).filter((value): value is string => value !== null))].sort();
}

function accountLabelOf(rows: InstanceInfo[], accountId: string): string {
  return rows.find((row) => row.account_id === accountId)?.account_label ?? accountId;
}

/** Quota-risk grouping: known quota first, unknown under "Not measured
 * yet" (Req.2; D-25's words, as on Accounts and Home). */
function quotaDataGroups(sorted: InstanceInfo[]): RowGroup[] {
  const known = sorted.filter((row) => row.quota !== null);
  const unknown = sorted.filter((row) => row.quota === null);
  const groups: RowGroup[] = [{ id: "quota-known", label: "", rollup: "", rows: known }];
  if (unknown.length > 0) {
    groups.push({
      id: "no-data",
      label: "Not measured yet",
      rollup: `${unknown.length} without a quota reading`,
      rows: unknown,
    });
  }
  return groups;
}

function ListSection({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <p className="px-3 pb-0.5 pt-1 text-[10px] font-medium uppercase tracking-[0.08em] text-faint">
        {label}
      </p>
      {children}
    </div>
  );
}

/** Tone classes for the compact state/attention chip (token-driven). */
const chipToneClasses: Record<StateChipTone, string> = {
  neutral: "bg-surface-2 text-muted border-hairline",
  success: "bg-success-soft text-success border-success/25",
  warning: "bg-warning-soft text-warning border-warning/25",
  danger: "bg-danger-soft text-danger border-danger/25",
  live: "bg-live-soft text-live border-live/25",
  accent: "bg-accent-soft text-accent-hi border-accent/25",
};

/**
 * The indicator row (AJ-E402/E403): I-01..I-11 in pass2/04 §3 render
 * order — provider mark, account disc, model chip, state chip, quota bar
 * with tier tag, tokens, children, git, attention, age, resource
 * sparkline. Absent data renders the exact honest copy (`Quota: no
 * data`, `default model`, `Unknown`, `—`), never fabricated numbers.
 *
 * Densities: Comfortable shows all I-01..I-11; Compact (~36 px, 2
 * lines) shows I-01, I-02, I-04, I-05 bar-only, I-09, I-10 plus CPU/RAM
 * as text. I-04 text and I-10 are NEVER hidden in either. Values older
 * than 2× their refresh interval carry the `stale - Nm` chip (AJ-E403
 * Req.5) — the value keeps its colour (contrast-safe), the caption
 * carries the meaning.
 */
/**
 * I-03 drift (AJ-E412 Req.3): true when a running model was observed
 * and differs from the launch overlay. The marker's text is the full
 * `Launched with X - now running Y` (WCAG 1.3.1: text, not just a dot).
 */
export function modelDrift(session: InstanceInfo): string | null {
  if (
    session.running_model !== null &&
    session.launch_model !== null &&
    session.running_model !== session.launch_model
  ) {
    return `Launched with ${session.launch_model} - now running ${session.running_model}`;
  }
  return null;
}

export function SessionRow({
  session,
  selected,
  onSelect,
  density = "comfortable",
  freshness = {},
  now = Date.now(),
}: {
  session: InstanceInfo;
  selected: boolean;
  onSelect: (id: string) => void;
  density?: "comfortable" | "compact";
  freshness?: Freshness;
  now?: number;
}) {
  const mark = providerMark(session.provider);
  const badge = attentionBadge(session);
  const chip = stateChip(session.status);
  const suffix = attentionSuffix(session);
  const tokens = tokenSummary(session.tokens);
  const samples = rememberResourceSample(session.id, session.cpu_pct, session.rss_bytes);
  const resources =
    session.cpu_pct !== null && session.rss_bytes !== null
      ? `${Math.round(session.cpu_pct)}% · ${formatBytes(session.rss_bytes)}`
      : null;
  const sampled = freshness[session.id];
  const quotaStale =
    session.quota !== null && sampled
      ? staleCaption(sampled.quota, REFRESH_INTERVALS.quota, now)
      : null;
  const resourceStale =
    resources !== null && sampled
      ? staleCaption(sampled.resources, REFRESH_INTERVALS.resources, now)
      : null;
  // Terminal rows never show resource slots: a finished Instance is not
  // "unavailable" — it is done. Active-but-unsampled keeps the honest `—`.
  const showResources = isActiveStatus(session.status);
  // WCAG 2.5.3 (AJ-E804): the row's name is its visible content; the
  // rest is its description.
  const detailsId = useId();

  if (density === "compact") {
    return (
      <button
        type="button"
        onClick={() => onSelect(session.id)}
        aria-describedby={detailsId}
        data-instance-id={session.id}
        className={
          "relative flex min-w-0 w-full flex-col gap-0.5 rounded-lg px-2.5 py-1.5 pl-3 text-left outline-none transition-colors " +
          "focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface" : "hover:bg-surface/60")
        }
      >
        <RowDetails id={detailsId} session={session} />
        {selected && (
          <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
        )}
        {/* L1: I-01 I-02 … I-04/I-10 (text never hidden in Compact) */}
        <span className="flex min-w-0 items-center gap-1.5 overflow-hidden">
          <span
            className="inline-flex min-w-0 shrink items-center gap-1 text-[10.5px] font-medium text-ink"
            title={providerDisplayName(session.provider)}
          >
            <mark.Icon width={11} height={11} className="shrink-0 text-accent-hi" />
            <span className="truncate">{mark.abbrev}</span>
          </span>
          <AccountAvatar accountId={session.account_id} label={session.account_label} />
          <span className="sr-only">{session.account_label}</span>
          <span className="ml-auto inline-flex shrink-0 items-center gap-1">
            {badge ? (
              <span
                className={
                  "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                  chipToneClasses[badge.tone]
                }
              >
                {badge.tone === "warning" || badge.tone === "danger" ? <span aria-hidden="true">!</span> : null}
                {badge.text}
              </span>
            ) : (
              <span
                className={
                  "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                  chipToneClasses[chip.tone]
                }
              >
                <chip.Icon width={10} height={10} />
                {chip.label}
              </span>
            )}
          </span>
        </span>
        {/* L2: I-05 bar-only + I-09 + CPU/RAM text (sparkline → text) */}
        <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-muted">
          <span
            className="relative inline-block h-1.5 w-12 shrink-0 overflow-hidden rounded-full bg-surface-3"
            title={session.quota ? `${Math.round(100 - session.quota.used_percentage)}% left · ${session.quota.confidence}` : "Quota: not measured yet"}
          >
            {session.quota ? (
              <span
                className={
                  "absolute inset-y-0 left-0 rounded-full " +
                  (100 - session.quota.used_percentage <= 10
                    ? "bg-danger"
                    : 100 - session.quota.used_percentage <= 30
                      ? "bg-warning"
                      : "bg-accent")
                }
                style={{ width: `${Math.max(2, session.quota.used_percentage)}%` }}
              />
            ) : (
              <span className="absolute inset-0 rounded-full border border-dashed border-faint/70" />
            )}
          </span>
          <StaleChip caption={quotaStale} />
          <GitBadge session={session} />
          {showResources && (
            <span className="ml-auto inline-flex min-w-0 shrink items-center gap-1">
              {resources ? (
                <span className="min-w-0 truncate tabular-nums">{resources}</span>
              ) : (
                <span className="text-faint" title="resources unavailable">
                  <span aria-hidden="true">—</span>
                  <span className="sr-only">resources unavailable</span>
                </span>
              )}
              <StaleChip caption={resourceStale} />
            </span>
          )}
        </span>
      </button>
    );
  }

  return (
    <button
      type="button"
      onClick={() => onSelect(session.id)}
      aria-describedby={detailsId}
      data-instance-id={session.id}
      className={
        "relative flex min-w-0 w-full flex-col gap-1 rounded-lg px-2.5 py-2 pl-3 text-left outline-none transition-colors " +
        "focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (selected ? "bg-surface" : "hover:bg-surface/60")
      }
    >
      <RowDetails id={detailsId} session={session} />
      {selected && (
        <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
      )}
      {/* I-01 I-02 I-03 … I-04/I-10. Narrow rails make the provider
          abbrev and model yield first (truncate); the disc, chip text and
          glyphs never clip (WCAG 1.4.1). */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden">
        <span
          className="inline-flex min-w-0 shrink items-center gap-1 text-[10.5px] font-medium text-ink"
          title={providerDisplayName(session.provider)}
        >
          <mark.Icon width={11} height={11} className="shrink-0 text-accent-hi" />
          <span className="truncate">{mark.abbrev}</span>
        </span>
        <AccountAvatar accountId={session.account_id} label={session.account_label} />
          <span className="sr-only">{session.account_label}</span>
        {identityWords(session) && (
          <span
            title={identityWords(session)!.text}
            aria-label={identityWords(session)!.text}
            tabIndex={0}
            data-testid="rail-identity"
            data-state={identityWords(session)!.state}
            className="shrink-0 rounded-full border border-hairline px-1 text-[9.5px] font-medium text-muted outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            {identityWords(session)!.glyph}
          </span>
        )}
        {session.kind === "desktop" && (
          <span
            title="desktop app session"
            className="shrink-0 rounded-full border border-accent/25 bg-accent-soft px-1 text-[9.5px] font-medium text-accent-hi"
            data-testid="rail-app-chip"
          >
            app
          </span>
        )}
        <span className="min-w-0 shrink truncate text-[10.5px] text-muted" title={session.model ?? "default model"}>
          {session.model ?? <span className="text-faint">default model</span>}
        </span>
        {modelDrift(session) && (
          <span
            title={modelDrift(session)!}
            aria-label={modelDrift(session)!}
            tabIndex={0}
            data-testid="model-drift"
            className="shrink-0 rounded-full border border-warning/25 bg-warning-soft px-1 text-[10px] font-medium text-warning outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            ⇄
          </span>
        )}
        <span className="ml-auto inline-flex shrink-0 items-center gap-1">
          {parkedWords(session, now) ? (
            <span
              title={parkedWords(session, now)!.long}
              aria-label={parkedWords(session, now)!.long}
              data-testid="rail-parked"
              className="inline-flex items-center gap-1 rounded-full border border-hairline px-1.5 py-px text-[10px] font-medium text-ink"
            >
              <span aria-hidden="true">⏸</span>
              {parkedWords(session, now)!.chip}
            </span>
          ) : badge ? (
            <span
              title={attentionTooltip(session)}
              className={
                "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                chipToneClasses[badge.tone]
              }
            >
              {badge.tone === "warning" || badge.tone === "danger" ? <span aria-hidden="true">!</span> : null}
              {badge.text}
            </span>
          ) : (
            <span
              className={
                "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                chipToneClasses[chip.tone]
              }
            >
              <chip.Icon width={10} height={10} />
              {chip.label}
            </span>
          )}
        </span>
      </span>
      {/* I-10 suffix (wait duration / reset countdown) + I-09 I-11. The
          age text yields (truncate) before anything else clips. */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-muted">
        {badge && suffix ? (
          <span
            className={
              "shrink-0 whitespace-nowrap font-medium " +
              (badge.tone === "danger" ? "text-danger" : "text-warning")
            }
          >
            {suffix}
          </span>
        ) : null}
        <GitBadge session={session} />
        <span className="ml-auto min-w-0 shrink truncate whitespace-nowrap tabular-nums">
          {ageSummary(session.started_at_ms, session.last_activity_ms)}
        </span>
      </span>
      {/* I-05 (+ stale chip when the sample expired) */}
      <span className="flex min-w-0 items-center gap-1.5">
        <QuotaBar quota={session.quota} />
        <StaleChip caption={quotaStale} />
      </span>
      {/* I-06 I-08 I-07 — right-hand resource text yields before anything
          clips. */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-muted">
        {tokens ? <span className="shrink-0 tabular-nums">{tokens}</span> : null}
        <ChildBadges session={session} />
        {showResources && (
          <span className="ml-auto inline-flex min-w-0 shrink items-center gap-1.5">
            <Sparkline
              series={samples}
              title={resources ? `ram/cpu ${resources}` : "resources unavailable"}
            />
            {resources ? <span className="min-w-0 truncate tabular-nums">{resources}</span> : null}
            <StaleChip caption={resourceStale} />
          </span>
        )}
      </span>
      {session.kind === "desktop" && (
        <span className="self-start text-[9px] font-medium uppercase tracking-wide text-accent-hi">
          app
        </span>
      )}
    </button>
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
        return (
          <button
            key={s.id}
            type="button"
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
              {s.account_label}, {badge ? badge.text : chip.label}
            </span>
            <AccountAvatar accountId={s.account_id} label={s.account_label} size={22} />
            <chip.Icon
              width={10}
              height={10}
              className={"absolute bottom-0 right-0 rounded-full bg-surface p-px " + minimalGlyphTone(chip.tone)}
            />
            {badge ? (
              <span
                className={
                  "absolute right-0 top-0 flex size-3.5 items-center justify-center rounded-full text-[8px] font-bold " +
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
                <span className="truncate text-[11.5px] font-medium text-ink">
                  {s.account_label}
                </span>
                <AccountAvatar accountId={s.account_id} label={s.account_label} size={16} />
              </span>
              <span className="mt-1 block text-[10px] leading-relaxed text-muted">
                {ariaSummary(s)}
              </span>
            </span>
          </button>
        );
      })}
      {needsYou > 0 && (
        <span
          className="mt-1 rounded-full border border-warning/40 bg-warning-soft px-1.5 py-0.5 text-[9px] font-bold text-warning"
          title={`${needsYou} session${needsYou === 1 ? "" : "s"} need you`}
        >
          ! {needsYou}
        </span>
      )}
      {active.length === 0 && <RailCalm />}
    </div>
  );
}

function minimalGlyphTone(tone: StateChipTone): string {
  switch (tone) {
    case "live": return "text-live";
    case "success": return "text-success";
    case "warning": return "text-warning";
    case "danger": return "text-danger";
    case "accent": return "text-accent-hi";
    default: return "text-muted";
  }
}

/**
 * The two-line card variant (AJ-E402 Req.4): identical I-01..I-11 fields
 * to the rail row, sparkline widened to the 5-min window. Used by the
 * Overview live list — no number exists here that the rail cannot
 * explain.
 */
export function SessionCard({ session, onSelect }: { session: InstanceInfo; onSelect?: (id: string) => void }) {
  const detailsId = useId();
  const mark = providerMark(session.provider);
  const badge = attentionBadge(session);
  const chip = stateChip(session.status);
  const suffix = attentionSuffix(session);
  const tokens = tokenSummary(session.tokens);
  const samples = rememberResourceSample(session.id, session.cpu_pct, session.rss_bytes);
  const resources =
    session.cpu_pct !== null && session.rss_bytes !== null
      ? `${Math.round(session.cpu_pct)}% · ${formatBytes(session.rss_bytes)}`
      : null;
  return (
    <button
      type="button"
      onClick={onSelect ? () => onSelect(session.id) : undefined}
      aria-describedby={detailsId}
      className="flex w-full min-w-0 flex-col gap-1.5 rounded-xl px-3 py-2.5 text-left outline-none transition-colors hover:bg-surface-2/60 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <RowDetails id={detailsId} session={session} />
      <span className="flex min-w-0 items-center gap-2">
        <span className="inline-flex shrink-0 items-center gap-1.5 text-[11.5px] font-medium text-ink">
          <mark.Icon width={12} height={12} className="text-accent-hi" />
          {mark.abbrev}
        </span>
        <AccountAvatar accountId={session.account_id} label={session.account_label} size={20} />
        <span className="min-w-0 truncate text-[12px] text-ink">{session.account_label}</span>
        <span className="min-w-0 shrink-0 truncate text-[11px] text-muted">
          {session.model ?? <span className="text-faint">default model</span>}
        </span>
        <span className="ml-auto inline-flex shrink-0 items-center gap-1">
          {badge ? (
            <span
              title={attentionTooltip(session)}
              className={
                "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium " +
                chipToneClasses[badge.tone]
              }
            >
              {badge.tone === "warning" || badge.tone === "danger" ? <span aria-hidden="true">!</span> : null}
              {badge.text}
            </span>
          ) : (
            <span
              className={
                "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium " +
                chipToneClasses[chip.tone]
              }
            >
              <chip.Icon width={11} height={11} />
              {chip.label}
            </span>
          )}
        </span>
      </span>
      <span className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted">
        {badge && suffix ? (
          <span
            className={
              "shrink-0 whitespace-nowrap font-medium " +
              (badge.tone === "danger" ? "text-danger" : "text-warning")
            }
          >
            {suffix}
          </span>
        ) : null}
        <QuotaBar quota={session.quota} />
        {tokens ? <span className="tabular-nums">{tokens}</span> : null}
        <ChildBadges session={session} />
        <GitBadge session={session} />
        <span className="ml-auto inline-flex shrink-0 items-center gap-2">
          <Sparkline
            series={samples}
            width={64}
            title={resources ? `ram/cpu ${resources}` : "resources unavailable"}
          />
          {resources ? <span className="tabular-nums">{resources}</span> : null}
        </span>
        <span className="shrink-0 whitespace-nowrap tabular-nums text-faint">
          {ageSummary(session.started_at_ms, session.last_activity_ms)}
        </span>
      </span>
    </button>
  );
}

const MACHINE_FILTER_KEY = "aij.machine.filter";

/** The board's chip filters and search, applied to a peer's rows. */
function filterRemote(rows: RemoteInstanceInfo[], filters: ChipFilters, query: string): RemoteInstanceInfo[] {
  const needle = query.trim().toLowerCase();
  return rows.filter((row) => {
    if (filters.provider && row.provider !== filters.provider) return false;
    if (filters.chips.includes("needs") && row.status !== "waiting_user") return false;
    if (filters.chips.includes("running") && row.status !== "running") return false;
    if (filters.chips.includes("ended") && !["exited", "ended", "stopped"].includes(row.status)) return false;
    if (!needle) return true;
    return [row.account_label, row.provider, row.project_name ?? "", row.model ?? ""].some((field) =>
      field.toLowerCase().includes(needle),
    );
  });
}

/** AJ-E709: one paired machine's Instances on the board — the machine is
 * named on every row (text, never colour alone), quota is "unknown" until
 * that machine shares it (D-12: never inferred). */
function RemoteBoardSection({
  machine,
  rows,
  selected,
  onSelect,
}: {
  machine: RemoteMachineInfo;
  rows: RemoteInstanceInfo[];
  selected: { machineId: string; instanceId: string | null } | null;
  onSelect: (instanceId: string) => void;
}): JSX.Element {
  const state =
    machine.freshness === "live"
      ? "● live"
      : machine.freshness === "stale"
        ? "◐ stale · last known"
        : "○ offline · last known";
  return (
    <div className="mt-1 flex flex-col gap-0.5" data-testid="board-machine" data-machine={machine.machine_id}>
      <p className="flex items-baseline gap-1.5 px-3 pb-0.5 pt-1 text-[10px] font-medium uppercase tracking-[0.08em] text-muted">
        <span className="truncate normal-case tracking-normal">{machine.label}</span>
        <span className="shrink-0 font-normal normal-case tracking-normal">{state}</span>
      </p>
      {rows.length === 0 ? (
        <p className="px-3 py-1 text-[11px] text-muted">
          {machine.freshness === "offline" ? "Offline — nothing to show." : "No sessions match."}
        </p>
      ) : (
        rows.map((row) => {
          const active = selected?.machineId === machine.machine_id && selected.instanceId === row.id;
          // AJ-E708: an away machine's rows are its last known state.
          const lastKnown = machine.freshness !== "live";
          const status = (lastKnown ? "was " : "") + row.status.replace(/_/g, " ");
          return (
            <button
              key={row.id}
              type="button"
              onClick={() => onSelect(row.id)}
              className={
                "relative flex w-full min-w-0 flex-col gap-0.5 rounded-lg py-1.5 pl-3 pr-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (active ? "bg-surface" : "hover:bg-surface/60")
              }
              // WCAG 2.5.3: named by its own words ("Work Max on Mac mini …").
              data-testid="board-remote-row"
            >
              {active && (
                <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
              )}
              <span className="flex min-w-0 items-center gap-1.5">
                <span
                  className={
                    "size-1.5 shrink-0 rounded-full " +
                    (lastKnown
                      ? "bg-faint"
                      : row.status === "running"
                        ? "bg-live"
                        : row.status === "waiting_user"
                          ? "bg-warning"
                          : "bg-faint")
                  }
                  aria-hidden="true"
                />
                <span className="min-w-0 flex-1 truncate text-[12.5px]">
                  {row.account_label}
                  {" "}<span className="sr-only">on</span>
                </span>
                <span className="shrink-0 rounded border border-hairline px-1 text-[9.5px] text-muted">{machine.label}</span>
              </span>
              <span className="flex min-w-0 gap-1 truncate pl-3 text-[10.5px] text-muted">
                <span>{row.provider}</span>
                <span>· {row.model ?? "model unknown"}</span>
                <span>· {status}</span>
                <span>· quota unknown</span>
              </span>
            </button>
          );
        })
      )}
    </div>
  );
}
