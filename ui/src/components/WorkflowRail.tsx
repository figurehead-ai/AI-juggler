import { ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { IconProjects, IconSessions, IconTerminal } from "./icons";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
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
  unsnoozedNeedsCount,
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
import {
  AccountDisc,
  ChildBadges,
  GitBadge,
  IndicatorLegend,
  StaleChip,
  attentionBadge,
  attentionSuffix,
  attentionTooltip,
  ariaSummary,
  isActiveStatus,
  providerMark,
  rememberResourceSample,
  stateChip,
  tokenSummary,
  ageSummary,
  formatBytes,
  type StateChipTone,
} from "./indicator";
import { QuotaBar, Sparkline } from "./primitives";

/** Freshness timestamps per Instance id (epoch ms) driving the stale rule. */
export type Freshness = Record<string, { resources: number; quota: number }>;

/** The three workflow tabs the whole navbar switches between. */
export type WorkTab = "projects" | "instances" | "sessions";

const TABS: { id: WorkTab; label: string; icon: ReactNode }[] = [
  { id: "projects", label: "Projects", icon: <IconProjects width={15} height={15} /> },
  { id: "instances", label: "Instances", icon: <IconTerminal width={15} height={15} /> },
  { id: "sessions", label: "Sessions", icon: <IconSessions width={15} height={15} /> },
];

const WIDTH_MIN = 208;
const WIDTH_MAX = 360;
const WIDTH_DEFAULT = 264;
const STORE_WIDTH = "aij.rail.width";

function readWidth(): number {
  const raw = Number(localStorage.getItem(STORE_WIDTH));
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
  tab,
  onTab,
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
}: {
  tab: WorkTab;
  onTab: (tab: WorkTab) => void;
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
}) {
  const minimal = density === "minimal";
  // Snooze/mute state (AJ-E404 Req.6): rail-owned, persisted; the tab
  // badge counts UNSNOOZED attention only.
  const [snooze, setSnooze] = useState<SnoozeState>(() => readSnooze());
  const updateSnooze = useCallback((next: SnoozeState) => {
    setSnooze(next);
    storeSnooze(next);
  }, []);
  const unsnoozedNeeds = unsnoozedNeedsCount(sessions, snooze, now);
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

  // Label threshold: below this width, tabs show icons only (with
  // tooltips) instead of squeezing text into clipping.
  const showLabels = !minimal && width >= 244;

  return (
    <aside
      style={{ width: minimal ? 60 : width }}
      className={
        "relative flex h-full shrink-0 select-none flex-col border-r border-hairline bg-surface/30 backdrop-blur-xl transition-[width] duration-150 ease-out " +
        (dragging ? "" : "")
      }
    >
      {header(minimal)}

      {/* Tab switcher — adapts: icon+label → icon-only as the rail narrows */}
      <nav
        className={
          "flex gap-1 px-2 pb-2 pt-0.5 " +
          (minimal ? "flex-col items-center" : "")
        }
        role="tablist"
      >
        {TABS.map((item) => {
          const active = tab === item.id;
          return (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={active}
              title={minimal || !showLabels ? item.label : undefined}
              onClick={() => onTab(item.id)}
              className={
                "group/tab relative flex min-w-0 flex-1 items-center justify-center overflow-hidden rounded-lg text-[12px] font-medium outline-none transition-colors duration-100 " +
                "focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (minimal
                  ? "size-9 "
                  : showLabels
                    ? "gap-1.5 px-2 py-[7px] "
                    : "gap-0 px-0 py-[7px] ") +
                (active
                  ? "border border-accent/25 bg-accent-soft text-accent-hi"
                  : "border border-transparent text-muted hover:bg-surface hover:text-ink")
              }
            >
              <span className="shrink-0 leading-none">{item.icon}</span>
              {showLabels && (
                <span className="min-w-0 truncate opacity-100 transition-opacity duration-150">
                  {item.label}
                </span>
              )}
              {/* Unsnoozed needs-you count rides the tab (Req.6) —
                  absolutely positioned so it never widens the tab. */}
              {item.id === "instances" && unsnoozedNeeds > 0 && (
                <span
                  className="attn-badge absolute right-0 top-0 flex items-center justify-center rounded-full px-1 text-[8.5px] font-bold leading-4 ring-1 ring-canvas"
                  title={`${unsnoozedNeeds} instance${unsnoozedNeeds === 1 ? "" : "s"} need you`}
                >
                  !{unsnoozedNeeds}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      {/* The live list for the active tab */}
      <div className="rail-scroll min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-2 pb-2">
        {minimal ? (
          <MinimalInstanceColumn
            sessions={sessions}
            selectedId={selectedId}
            onSelect={onSelectSession}
          />
        ) : tab === "projects" ? (
          <ProjectList projects={projects} selectedId={selectedId} onSelect={onSelectProject} />
        ) : (
          <InstanceBoard
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
          />
        )}
      </div>

      {/* The workflow's primary action lives in the navigator — on the
          Projects tab it's AJ-E305 flow (b), elsewhere New session. */}
      <div className="border-t border-hairline/60 p-2">
        <button
          type="button"
          onClick={tab === "projects" ? onNewProject : onNewSession}
          title={
            minimal
              ? tab === "projects"
                ? "New project (⌘N)"
                : "New session"
              : undefined
          }
          data-testid={
            tab === "projects" ? "rail-new-project" : "rail-new-session"
          }
          className="flex w-full items-center justify-center gap-2 overflow-hidden rounded-lg border border-accent/30 bg-accent-soft px-3 py-[7px] text-[12px] font-medium text-accent-hi outline-none transition-colors hover:border-accent/50 hover:bg-accent/15 focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          {tab === "projects" ? (
            <IconProjects width={14} height={14} />
          ) : (
            <IconTerminal width={14} height={14} />
          )}
          {!minimal && (
            <span className="truncate">
              {tab === "projects" ? "New project" : "New session"}
            </span>
          )}
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

function ProjectList({
  projects,
  selectedId,
  onSelect,
}: {
  projects: ProjectInfo[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  if (projects.length === 0) {
    return (
      <p className="px-2 py-6 text-center text-[11px] leading-relaxed text-faint">
        Launch a session in any directory — it becomes a project here.
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-0.5">
      {projects.map((project) => {
        const active = project.id === selectedId;
        return (
          <button
            key={project.id}
            type="button"
            onClick={() => onSelect(project.id)}
            className={
              "relative min-w-0 rounded-lg py-[7px] pl-3 pr-2.5 text-left outline-none transition-colors " +
              "focus-visible:ring-1 focus-visible:ring-accent/60 " +
              (active ? "bg-surface" : "hover:bg-surface/60")
            }
          >
            {active && (
              <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
            )}
            <div className="flex min-w-0 items-center gap-2">
              <span className="shrink-0 text-accent-hi/90">
                <IconProjects width={13} height={13} />
              </span>
              <span className="min-w-0 flex-1 truncate text-[12.5px] font-medium text-ink">
                {project.name}
              </span>
              {project.active_instance ? (
                <span className="dot-live size-1.5 shrink-0 rounded-full bg-live" />
              ) : null}
            </div>
            <p className="mt-0.5 truncate pl-[21px] text-[10.5px] leading-tight text-faint">
              {project.accounts_used.length} acct
              {project.accounts_used.length === 1 ? "" : "s"} · {project.instance_count} run
              {project.instance_count === 1 ? "" : "s"}
            </p>
          </button>
        );
      })}
    </div>
  );
}

/** Virtualisation threshold: past this many rows the flat list windows. */
const VIRTUALISE_AT = 50;
const ROW_HEIGHT = { comfortable: 70, compact: 42 } as const;

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
}: {
  /** AJ-E411 Req.5: account expiry feeds the Needs-me tie-break. */
  accounts?: AccountInfo[];
  sessions: InstanceInfo[];
  loginTerminals: { id: string; title: string; ended: boolean }[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  density: Density;
  freshness: Freshness;
  now: number;
  projects: ProjectInfo[];
  snooze: SnoozeState;
  onSnooze: (next: SnoozeState) => void;
}) {
  const [sort, setSort] = useState<SortMode>("needs");
  const [group, setGroup] = useState<GroupBy>("none");
  const [filters, setFilters] = useState<ChipFilters>(NO_CHIP_FILTERS);
  const [query, setQuery] = useState("");
  const [menu, setMenu] = useState<{ row: InstanceInfo; x: number; y: number } | null>(null);
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
  const groups =
    effectiveGroup === "quota-data"
      ? quotaDataGroups(sorted)
      : groupSessions(sorted, effectiveGroup === "none" ? "none" : effectiveGroup, projects);
  const counts = chipCounts(sessions);

  // Windowing (Req.7): only the flat, ungrouped list virtualises — the
  // 200-row scale is ungrouped monitoring; grouped triage views stay
  // fully rendered. Every row is reachable by scroll in both modes.
  const flatRows = groups.length === 1 ? (groups[0]?.rows ?? []) : null;
  let windowed: { start: number; end: number } | null = null;
  if (flatRows && flatRows.length > VIRTUALISE_AT) {
    const rowH = ROW_HEIGHT[effectiveDensity];
    const start = Math.max(0, Math.floor(scrollTop / rowH) - 4);
    const end = Math.min(flatRows.length, Math.ceil((scrollTop + viewportH) / rowH) + 4);
    windowed = { start, end };
  }

  const openMenu = (row: InstanceInfo, x: number, y: number) => setMenu({ row, x, y });
  const snoozeFor = (row: InstanceInfo, ms: number | null, muted = false) => {
    const next = { ...snooze };
    if (muted) next[row.id] = { until: null, muted: true };
    else next[row.id] = { until: ms === null ? null : Date.now() + ms, muted: false };
    onSnooze(next);
    setMenu(null);
  };
  const clearSnooze = (row: InstanceInfo) => {
    const next = { ...snooze };
    delete next[row.id];
    onSnooze(next);
    setMenu(null);
  };

  return (
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
            placeholder="Search ⌘K"
            aria-label="Search instances by account, project, branch, model or provider"
            className="w-full rounded-lg border border-hairline bg-surface/70 py-1.5 pl-6 pr-2 text-[11.5px] text-ink outline-none placeholder:text-faint focus-visible:ring-1 focus-visible:ring-accent/60"
          />
        </div>
        <div className="flex flex-wrap items-center gap-1" role="group" aria-label="Filters">
          <FilterChip active={filters.chips.length === 0 && !filters.provider && !filters.account && !filters.project} onClick={() => setFilters(NO_CHIP_FILTERS)}>
            All
          </FilterChip>
          <FilterChip
            active={filters.chips.includes("needs")}
            onClick={() => toggleChip(setFilters, "needs")}
            count={counts.needs}
          >
            Needs me
          </FilterChip>
          <FilterChip active={filters.chips.includes("running")} onClick={() => toggleChip(setFilters, "running")} count={counts.running}>
            Running
          </FilterChip>
          <FilterChip active={filters.chips.includes("ended")} onClick={() => toggleChip(setFilters, "ended")} count={counts.ended}>
            Ended
          </FilterChip>
        </div>
        <div className="flex flex-wrap items-center gap-1">
          <Select ariaLabel="Sort by" value={sort} onChange={(value) => setSort(value as SortMode)} options={SORT_LABELS.map((entry) => ({ value: entry.id, label: entry.label }))} />
          <Select ariaLabel="Group by" value={group} onChange={(value) => setGroup(value as GroupBy)} options={GROUP_LABELS.map((entry) => ({ value: entry.id, label: `Group: ${entry.label}` }))} />
          <Select
            ariaLabel="Filter by provider"
            value={filters.provider ?? ""}
            onChange={(value) => setFilters((current) => ({ ...current, provider: value || null }))}
            options={[{ value: "", label: "Provider: all" }, ...unique(sessions, (row) => row.provider).map((value) => ({ value, label: value }))]}
          />
          <Select
            ariaLabel="Filter by account"
            value={filters.account ?? ""}
            onChange={(value) => setFilters((current) => ({ ...current, account: value || null }))}
            options={[{ value: "", label: "Account: all" }, ...unique(sessions, (row) => row.account_id).map((value) => ({ value, label: accountLabelOf(sessions, value) }))]}
          />
          <Select
            ariaLabel="Filter by project"
            value={filters.project ?? ""}
            onChange={(value) => setFilters((current) => ({ ...current, project: value || null }))}
            options={[{ value: "", label: "Project: all" }, ...projects.map((project) => ({ value: project.id, label: project.name }))]}
          />
        </div>
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
        {sorted.length === 0 ? (
          sessions.length === 0 ? (
            /* Nothing exists at all — the All-clear explainer (legend
               surface 3). */
            <AllClearExplainer />
          ) : (
            <div className="flex flex-col items-center gap-2 px-2 py-8 text-center">
              <p className="text-[11.5px] font-medium text-muted">No instances match</p>
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
                selected={row.id === selectedId}
                onSelect={onSelect}
                density={effectiveDensity}
                freshness={freshness}
                now={now}
                snooze={snooze}
                onContextMenu={openMenu}
              />
            ))}
            <div style={{ height: (flatRows.length - windowed.end) * ROW_HEIGHT[effectiveDensity] }} />
            <p className="px-3 pt-1 text-center text-[9px] text-faint">
              {flatRows.length} instances
            </p>
          </div>
        ) : (
          groups.map((entry) => (
            <div key={entry.id} className="mb-1 flex flex-col gap-0.5">
              {entry.label !== "" && (
                <p className="flex items-baseline gap-1.5 px-3 pb-0.5 pt-1 text-[10px] font-medium uppercase tracking-[0.08em] text-faint">
                  <span className="truncate normal-case tracking-normal">{entry.label}</span>
                  {entry.rollup && <span className="shrink-0 font-normal normal-case tracking-normal text-faint/80">· {entry.rollup}</span>}
                </p>
              )}
              {entry.rows.map((row) => (
                <BoardRow
                  key={row.id}
                  row={row}
                  selected={row.id === selectedId}
                  onSelect={onSelect}
                  density={effectiveDensity}
                  freshness={freshness}
                  now={now}
                  snooze={snooze}
                  onContextMenu={openMenu}
                />
              ))}
            </div>
          ))
        )}
      </div>

      {menu && (
        <RowContextMenu
          row={menu.row}
          snooze={snooze}
          now={now}
          x={menu.x}
          y={menu.y}
          onSnoozeFor={(ms) => snoozeFor(menu.row, ms)}
          onMute={() => snoozeFor(menu.row, null, true)}
          onClear={() => clearSnooze(menu.row)}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  );
}

function BoardRow({
  row,
  selected,
  onSelect,
  density,
  freshness,
  now,
  snooze,
  onContextMenu,
}: {
  row: InstanceInfo;
  selected: boolean;
  onSelect: (id: string) => void;
  density: "comfortable" | "compact";
  freshness: Freshness;
  now: number;
  snooze: SnoozeState;
  onContextMenu: (row: InstanceInfo, x: number, y: number) => void;
}) {
  const caption = snoozeCaption(row, snooze, now);
  return (
    <div className="group relative">
      {caption && (
        <span className="absolute -top-0.5 right-2 z-10 rounded bg-surface-3 px-1 text-[8.5px] font-medium text-muted" title={caption}>
          {caption}
        </span>
      )}
      <SessionRow
        session={row}
        selected={selected}
        onSelect={onSelect}
        density={density}
        freshness={freshness}
        now={now}
      />
      {/* The I-10 context menu trigger (Req.6): right-click the row, or
          this ⋯ button for pointer-free access. */}
      <button
        type="button"
        aria-label={`Row actions for ${row.account_label}`}
        aria-haspopup="menu"
        onClick={(event) => onContextMenu(row, event.clientX, event.clientY)}
        onContextMenu={(event) => {
          event.preventDefault();
          onContextMenu(row, event.clientX, event.clientY);
        }}
        className="absolute bottom-1 left-0.5 z-10 rounded px-0.5 text-[10px] text-faint opacity-0 transition-opacity outline-none hover:text-ink focus-visible:opacity-100 focus-visible:ring-1 focus-visible:ring-accent/60 group-hover:opacity-100"
      >
        ⋯
      </button>
    </div>
  );
}

/** The I-10 context menu: snooze durations + mute (Req.6). */
function RowContextMenu({
  row,
  snooze,
  now,
  x,
  y,
  onSnoozeFor,
  onMute,
  onClear,
  onClose,
}: {
  row: InstanceInfo;
  snooze: SnoozeState;
  now: number;
  x: number;
  y: number;
  onSnoozeFor: (ms: number | null) => void;
  onMute: () => void;
  onClear: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onClose();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);
  const snoozed = isSnoozed(row, snooze, now);
  const resetAt = row.quota?.resets_at_ms ?? null;
  const style = {
    left: Math.min(x, window.innerWidth - 190),
    top: Math.min(y, window.innerHeight - 190),
  };
  return (
    <div
      ref={ref}
      role="menu"
      aria-label={`Attention options for ${row.account_label}`}
      style={style}
      className="fixed z-50 flex w-44 flex-col gap-0.5 rounded-xl border border-hairline bg-surface p-1.5 shadow-card"
    >
      <span className="px-2 pb-1 pt-0.5 text-[9.5px] font-medium uppercase tracking-wide text-faint">
        {row.account_label}
      </span>
      {snoozed ? (
        <MenuButton onClick={onClear}>Unsnooze / unmute</MenuButton>
      ) : (
        <>
          <MenuButton onClick={() => onSnoozeFor(15 * 60_000)}>Snooze 15m</MenuButton>
          <MenuButton onClick={() => onSnoozeFor(60 * 60_000)}>Snooze 1h</MenuButton>
          <MenuButton
            onClick={() => onSnoozeFor(resetAt !== null ? Math.max(0, resetAt - now) : 24 * 60 * 60_000)}
            disabled={resetAt === null}
            title={resetAt === null ? "No known reset time for this quota window" : undefined}
          >
            Until reset
          </MenuButton>
          <MenuButton onClick={onMute}>Mute this instance</MenuButton>
        </>
      )}
    </div>
  );
}

function MenuButton({ onClick, disabled, title, children }: { onClick: () => void; disabled?: boolean; title?: string; children: ReactNode }) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      disabled={disabled}
      title={title}
      className="rounded-lg px-2 py-1 text-left text-[11.5px] text-ink outline-none transition-colors hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:text-faint disabled:hover:bg-transparent"
    >
      {children}
    </button>
  );
}

function FilterChip({ active, onClick, count, children }: { active: boolean; onClick: () => void; count?: number; children: ReactNode }) {
  return (
    <button
      type="button"
      aria-pressed={active}
      aria-label={count !== undefined ? `${children}, ${count} items` : String(children)}
      onClick={onClick}
      className={
        "rounded-full border px-2 py-0.5 text-[10px] font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (active
          ? "border-accent/40 bg-accent-soft text-accent-hi"
          : "border-hairline text-muted hover:bg-surface hover:text-ink")
      }
    >
      {children}
      {count !== undefined && <span className="ml-1 tabular-nums opacity-80">{count}</span>}
    </button>
  );
}

function Select({ ariaLabel, value, onChange, options }: { ariaLabel: string; value: string; onChange: (value: string) => void; options: { value: string; label: string }[] }) {
  return (
    <select
      aria-label={ariaLabel}
      value={value}
      onChange={(event) => onChange(event.target.value)}
      className="max-w-full truncate rounded-lg border border-hairline bg-surface/70 px-1.5 py-1 text-[10px] text-muted outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
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

/** Quota-risk grouping: known quota first, unknown under `no data` (Req.2). */
function quotaDataGroups(sorted: InstanceInfo[]): RowGroup[] {
  const known = sorted.filter((row) => row.quota !== null);
  const unknown = sorted.filter((row) => row.quota === null);
  const groups: RowGroup[] = [{ id: "quota-known", label: "", rollup: "", rows: known }];
  if (unknown.length > 0) {
    groups.push({
      id: "no-data",
      label: "no data",
      rollup: `${unknown.length} without a quota feed`,
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

  if (density === "compact") {
    return (
      <button
        type="button"
        onClick={() => onSelect(session.id)}
        aria-label={ariaSummary(session)}
        data-instance-id={session.id}
        className={
          "relative flex min-w-0 w-full flex-col gap-0.5 rounded-lg px-2.5 py-1.5 pl-3 text-left outline-none transition-colors " +
          "focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface" : "hover:bg-surface/60")
        }
      >
        {selected && (
          <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
        )}
        {/* L1: I-01 I-02 … I-04/I-10 (text never hidden in Compact) */}
        <span className="flex min-w-0 items-center gap-1.5 overflow-hidden">
          <span
            className="inline-flex min-w-0 shrink items-center gap-1 text-[10.5px] font-medium text-ink"
            title={session.provider}
          >
            <mark.Icon width={11} height={11} className="shrink-0 text-accent-hi" />
            <span className="truncate">{mark.abbrev}</span>
          </span>
          <AccountDisc accountId={session.account_id} label={session.account_label} />
          <span className="ml-auto inline-flex shrink-0 items-center gap-1">
            {badge ? (
              <span
                className={
                  "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                  chipToneClasses[badge.tone]
                }
              >
                {badge.tone === "warning" || badge.tone === "danger" ? "!" : null}
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
            title={session.quota ? `${Math.round(100 - session.quota.used_percentage)}% left · ${session.quota.confidence}` : "Quota: no data"}
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
                <span className="text-faint" title="resources unavailable">—</span>
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
      aria-label={ariaSummary(session)}
      data-instance-id={session.id}
      className={
        "relative flex min-w-0 w-full flex-col gap-1 rounded-lg px-2.5 py-2 pl-3 text-left outline-none transition-colors " +
        "focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (selected ? "bg-surface" : "hover:bg-surface/60")
      }
    >
      {selected && (
        <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
      )}
      {/* I-01 I-02 I-03 … I-04/I-10. Narrow rails make the provider
          abbrev and model yield first (truncate); the disc, chip text and
          glyphs never clip (WCAG 1.4.1). */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden">
        <span
          className="inline-flex min-w-0 shrink items-center gap-1 text-[10.5px] font-medium text-ink"
          title={session.provider}
        >
          <mark.Icon width={11} height={11} className="shrink-0 text-accent-hi" />
          <span className="truncate">{mark.abbrev}</span>
        </span>
        <AccountDisc accountId={session.account_id} label={session.account_label} />
        {session.kind === "desktop" && (
          <span
            title="desktop app instance"
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
          {badge ? (
            <span
              title={attentionTooltip(session)}
              className={
                "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                chipToneClasses[badge.tone]
              }
            >
              {badge.tone === "warning" || badge.tone === "danger" ? "!" : null}
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
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-faint">
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
            aria-label={ariaSummary(s)}
            title={ariaSummary(s)}
            className={
              "group relative flex size-9 items-center justify-center rounded-lg outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
              (s.id === selectedId ? "bg-surface" : "hover:bg-surface")
            }
          >
            <AccountDisc accountId={s.account_id} label={s.account_label} size={22} />
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
            <span className="pointer-events-none absolute left-full top-0 z-30 ml-2 hidden w-56 rounded-lg border border-hairline bg-surface p-2.5 text-left shadow-card group-hover:block group-focus-visible:block">
              <span className="flex items-center gap-1.5">
                <span className="truncate text-[11.5px] font-medium text-ink">
                  {s.account_label}
                </span>
                <AccountDisc accountId={s.account_id} label={s.account_label} size={16} />
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
          title={`${needsYou} instance${needsYou === 1 ? "" : "s"} need you`}
        >
          ! {needsYou}
        </span>
      )}
      {active.length === 0 && (
        <span className="px-1 py-2 text-center text-[9px] leading-relaxed text-faint">
          All clear
        </span>
      )}
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

/** The empty-Inbox "All clear" explainer — carries the shared legend. */
function AllClearExplainer() {
  return (
    <div className="flex flex-col gap-2 px-2 py-2">
      <p className="text-[11px] font-medium text-success">All clear — nothing needs you</p>
      <p className="text-[10px] leading-relaxed text-faint">
        Live instances appear here with their full indicator row. The legend below explains every state, the quota bar and the sparkline.
      </p>
      <IndicatorLegend compact />
    </div>
  );
}

/**
 * The two-line card variant (AJ-E402 Req.4): identical I-01..I-11 fields
 * to the rail row, sparkline widened to the 5-min window. Used by the
 * Overview live list — no number exists here that the rail cannot
 * explain.
 */
export function SessionCard({ session, onSelect }: { session: InstanceInfo; onSelect?: (id: string) => void }) {
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
      aria-label={ariaSummary(session)}
      className="flex w-full min-w-0 flex-col gap-1.5 rounded-xl px-3 py-2.5 text-left outline-none transition-colors hover:bg-surface-2/60 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <span className="flex min-w-0 items-center gap-2">
        <span className="inline-flex shrink-0 items-center gap-1.5 text-[11.5px] font-medium text-ink">
          <mark.Icon width={12} height={12} className="text-accent-hi" />
          {mark.abbrev}
        </span>
        <AccountDisc accountId={session.account_id} label={session.account_label} size={20} />
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
              {badge.tone === "warning" || badge.tone === "danger" ? "!" : null}
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
