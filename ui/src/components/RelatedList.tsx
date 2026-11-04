/**
 * E31 (AJ-E3103; relations-map §5 "One list"; D-62 decisions 20–21) —
 * the one list every related tab uses: a page's sessions (a project's, an
 * App's, an Account's), drawn with the one session row.
 *
 * - **Order**: what needs you, what's working, what's open, what was
 *   active in the last 7 days, then the rest (each newest first), headed
 *   by section once there's more than one.
 * - **Search** past 8 rows (its words, and what was said where the
 *   transcripts are indexed); **filter chips** for state (Open now,
 *   Recent, Ended), App, Account, project and Mac, each offered only when
 *   it could narrow something (two values or more; Mac only when paired
 *   Macs ran its sessions), with how many rows each value shows.
 * - **Paged** by keyset (`sessions_page`), drawn as far as it's scrolled;
 *   opened from memory at once and read again behind it.
 * - **Never jumps**: it follows the app's changes, and new rows wait
 *   behind "N new".
 * - **Empty teaches**: what would appear here and the step that brings
 *   it; a search or filter that empties it says so, with the way back.
 *
 * Its heading's count is the list's own (the relation-count cache for a
 * whole list, AJ-E3102), so the tab's count, the heading and the rows
 * agree.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { hasLiveBackend, type AccountInfo, type ProjectInfo } from "../lib/bridge";
import { sessionsQuery, type SessionsQuery } from "../lib/ipc/sessions";
import { useRelationCounts } from "../lib/relation-counts";
import type { EntityKind } from "../lib/ipc/relations";
import { PagedSessions, type Row } from "../pages/SessionsList";
import { FilterChip } from "./related/FilterChip";
import { NO_FILTERS, STATE_WORDS, filtersOn, stateQuery, type FacetKind, type Filters, type StateFilter } from "./related/order";
import { localRows, useBelongs } from "./related/local";
import { useFacetNames, useFacets } from "./related/useFacets";
import { NothingMatches, RelatedEmpty, type Teach } from "./related/RelatedEmpty";

/** Past this many rows a list has a search and its filters (QUALITY.md
 * "Finding things": search appears at 9). */
export const SEARCH_PAST = 8;

/** What a list is of: one page's entity. */
export type Anchor = Pick<Partial<SessionsQuery>, "project_id" | "app_id" | "subscription_id" | "account_id" | "tool_id" | "machine_id">;

/** The anchor's kind in the relation counts, and the filter it makes
 * pointless (the page's own). */
function anchorKind(anchor: Anchor): { kind: EntityKind; id: string; own: FacetKind | null } {
  if (anchor.project_id) return { kind: "project", id: anchor.project_id, own: "project" };
  if (anchor.app_id) return { kind: "app", id: anchor.app_id, own: "app" };
  if (anchor.subscription_id) return { kind: "subscription", id: anchor.subscription_id, own: "account" };
  if (anchor.account_id) return { kind: "folder", id: anchor.account_id, own: "app" };
  if (anchor.tool_id) return { kind: "tool", id: anchor.tool_id, own: null };
  if (anchor.machine_id) return { kind: "machine", id: anchor.machine_id, own: "machine" };
  return { kind: "all", id: "", own: null };
}

/** The chip's value for "No project" (no project's id). */
const NONE = "(none)";

const CHIPS: { kind: FacetKind; label: string; all: string }[] = [
  { kind: "app", label: "App", all: "All" },
  { kind: "account", label: "Account", all: "All" },
  { kind: "project", label: "Project", all: "All" },
  { kind: "machine", label: "Mac", all: "All" },
];

export interface RelatedListProps {
  /** Names the list (its memory's key, with its filters). */
  listKey: string;
  anchor: Anchor;
  /** The design preview's rows (no backend), every one of the page's. */
  local: () => Row[];
  /** For the filters' names: the folders (Apps) and projects. */
  folders: readonly AccountInfo[];
  projects: ProjectInfo[];
  /** Rows name their project (not on the project's own page). */
  showProjects: boolean;
  /** On an App's page: rows don't repeat the App. */
  inApp?: boolean;
  /** The empty list's lesson. */
  teach: Teach;
  /** A page's own lesson in its place (a project's: what was done there). */
  emptyView?: JSX.Element;
  onOpenSession: (id: string) => void;
  /** How many rows the whole list holds, once read. */
  onTotal?: (total: number) => void;
  /** What the search says it looks in ("Search this project's sessions"). */
  searchLabel: string;
}

export function RelatedList(props: RelatedListProps): JSX.Element {
  const { anchor, listKey } = props;
  const live = hasLiveBackend();
  const entity = anchorKind(anchor);
  const counts = useRelationCounts(live ? entity.kind : null, entity.id);
  const [filters, setFilters] = useState<Filters>(NO_FILTERS);
  const [words, setWords] = useState("");
  const [searched, setSearched] = useState("");
  // The search waits for a pause in typing.
  useEffect(() => {
    const timer = setTimeout(() => setSearched(words.trim()), 200);
    return () => clearTimeout(timer);
  }, [words]);
  // "Recent" is the 7 days before the list was opened.
  const [opened] = useState(() => Date.now());
  const [whole, setWhole] = useState<number | null>(null);
  const narrowed = filtersOn(filters) > 0 || searched !== "";
  const base = counts?.sessions.all ?? (live ? whole : props.local().length);
  const findable = (base ?? 0) > SEARCH_PAST || narrowed || words !== "";
  const belongs = useBelongs(!live && findable);
  const state = stateQuery(filters.state, opened);
  const scoped: SessionsQuery = sessionsQuery({ ...anchor, which: state.which, since_ms: state.since_ms, text: searched || null });
  const query: SessionsQuery = {
    ...scoped,
    project_id: filters.project ?? scoped.project_id,
    app_id: filters.app ?? scoped.app_id ?? null,
    subscription_id: filters.account ?? scoped.subscription_id ?? null,
    machine_id: filters.machine,
  };
  // Every filter in the key, "*" for none ("" is "No project").
  const key = [listKey, filters.state, filters.project ?? "*", filters.app ?? "*", filters.account ?? "*", filters.machine ?? "*", searched].join("|");
  const localShown = (rows: Row[]) => localRows(rows, belongs, filters, searched, opened);
  // The filters' values, over the list in its state and search.
  const facets = useFacets(`${listKey}|${filters.state}|${searched}`, scoped, () => localRows(props.local(), belongs, { ...NO_FILTERS, state: filters.state }, searched, opened).map(belongs), findable, counts?.sessions.all ?? 0);
  const nameOf = useFacetNames(props.folders, props.projects, facets);
  const set = (kind: FacetKind, value: string | null) => setFilters((now) => ({ ...now, [kind]: value }));
  const clear = () => {
    setFilters(NO_FILTERS);
    setWords("");
    setSearched("");
  };
  const empty = useMemo(
    () => (narrowed ? <NothingMatches words={searched} filtered={filtersOn(filters) > 0} onClear={clear} /> : (props.emptyView ?? <RelatedEmpty teach={props.teach} />)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [narrowed, searched, filters, props.teach, props.emptyView],
  );
  return (
    <div className="flex w-full max-w-content flex-col gap-3" data-testid="related-list" data-anchor={entity.kind}>
      {findable && (
        <div className="flex flex-wrap items-center gap-2" data-testid="related-filters">
          <input
            type="search"
            value={words}
            onChange={(event) => setWords(event.target.value)}
            placeholder="Search by title, project, branch, date or what was said"
            aria-label={props.searchLabel}
            className="h-9 min-w-[14rem] flex-1 rounded-lg border border-hairline bg-surface px-3 text-body text-ink placeholder:text-muted"
            data-testid="related-search"
          />
          <FilterChip
            label="State"
            value={filters.state === "all" ? null : filters.state}
            options={(["open", "recent", "ended"] as StateFilter[]).map((id) => ({ id, label: STATE_WORDS[id] }))}
            onChange={(next) => setFilters((now) => ({ ...now, state: (next as StateFilter | null) ?? "all" }))}
            testId="related-filter-state"
          />
          {CHIPS.filter((chip) => chip.kind !== entity.own).map((chip) => {
            const values = facets?.[chip.kind] ?? [];
            // A Mac only when its sessions ran on more than one.
            if (chip.kind === "machine" && values.length < 2 && filters.machine === null) return null;
            // "No project" is `""` to the backend, and All's value to the chip.
            const value = filters[chip.kind];
            return (
              <FilterChip
                key={chip.kind}
                label={chip.label}
                all={chip.all}
                value={value === "" ? NONE : value}
                options={values.map((facet) => ({ id: facet.id || NONE, label: nameOf(chip.kind, facet.id), count: facet.count }))}
                onChange={(next) => set(chip.kind, next === NONE ? "" : next)}
                testId={`related-filter-${chip.kind}`}
              />
            );
          })}
          {narrowed && (
            <button type="button" onClick={clear} className="h-8 rounded-full px-2.5 text-meta text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="related-filter-clear">
              Clear filters
            </button>
          )}
        </div>
      )}
      <PagedSessions
        listKey={key}
        query={query}
        local={() => localShown(props.local())}
        empty={empty}
        projects={props.showProjects ? props.projects : undefined}
        onOpenSession={props.onOpenSession}
        onTotal={(total) => {
          if (!narrowed) {
            setWhole(total);
            props.onTotal?.(total);
          }
        }}
        inApp={props.inApp}
        ordered
      />
    </div>
  );
}
