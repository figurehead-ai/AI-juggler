/**
 * D-118, AJ-E3713 R5 — the feed's filters, as the story-feed mock draws
 * them: the tabs (All · Needs you · Sessions · Tasks · Pools · Switches &
 * limits · This Mac) with a ⋯ at the row's end ("Include my actions",
 * "Mark all read", and under Sessions "Mark all reviewed", "Pick up where
 * you left off" folded in, D-118 point 3); then the state chips (Running,
 * Finished, Failed, each in its own tint with its count today) and
 * "Showing:" one App, Account or project (the pickers rule: the ones in
 * the feed most first, then All, search past 8). Remembered per viewer.
 * The pinned zone is never filtered.
 */
import { useState, type JSX } from "react";
import { Picker, type PickerOption } from "./controls";
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { IconProfiles, IconProjects, IconSessions } from "./icons";
import { IconPool } from "./icons-actions";
import { IconCheck } from "./icons-controls";
import { IconRead } from "./icons-menu";
import { cliAppName } from "../lib/app-name";
import { MARK_HINTS } from "../lib/feed-words";
import { STATES, TABS, type StateChip, type Tab } from "../lib/feed-chips";
import type { FeedFiltersInfo } from "../lib/ipc/feed";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { PoolsInfo } from "../types/generated/PoolsInfo";
import type { SubscriptionInfo } from "../lib/subscriptions";
import { accountNamesApart } from "../lib/subscriptions";

/** What the Showing picker offers: a project, an App (its folder), a
 * pool or an Account (its folders), each a value `kind:id`. */
export interface Facets {
  projects: readonly ProjectInfo[];
  accounts: readonly AccountInfo[];
  subscriptions: readonly SubscriptionInfo[] | null;
  pools: PoolsInfo | null;
}

/** How often each picker value comes up in the posts loaded (`app:…`,
 * `project:…`): the most first in the picker. */
export type Seen = ReadonlyMap<string, number>;

/** The picked one with its kind ("billing · Project"); `undefined` for every app. */
function facetLabel(options: readonly PickerOption[], facet: string): string | undefined {
  const picked = options.find((option) => option.value === facet);
  return picked?.hint ? `${picked.label} · ${picked.hint}` : undefined;
}

function facetOptions(facets: Facets, seen: Seen): { options: PickerOption[]; suggested: string[] } {
  const projects = [...facets.projects].sort((a, b) => (b.last_activity_ms ?? 0) - (a.last_activity_ms ?? 0));
  const options: PickerOption[] = [{ value: "all", label: "Every app", pinned: true }];
  for (const account of facets.accounts) options.push({ value: `app:${account.id}`, label: cliAppName(account.provider, account.label), hint: "App", icon: IconSessions });
  const named = accountNamesApart(facets.subscriptions ?? []);
  for (const sub of facets.subscriptions ?? []) if (sub.profiles.length > 1) options.push({ value: `account:${sub.key}`, label: named(sub), hint: "Account", icon: IconProfiles });
  for (const project of projects) options.push({ value: `project:${project.id}`, label: project.name, hint: "Project", icon: IconProjects });
  for (const pool of facets.pools?.pools ?? []) options.push({ value: `pool:${pool.id}`, label: pool.name, hint: "Pool", icon: IconPool });
  // Suggested: what the feed shows most, then the most recent projects.
  const inFeed = [...seen.entries()].filter(([value]) => options.some((option) => option.value === value)).sort((a, b) => b[1] - a[1]).map(([value]) => value);
  const suggested = [...new Set([...inFeed, ...projects.slice(0, 2).map((project) => `project:${project.id}`)])].slice(0, 5);
  return { options, suggested };
}

/** The picker's value for `filters`. */
function facetOf(filters: FeedFiltersInfo, facets: Facets): string {
  if (filters.project) return `project:${filters.project}`;
  if (filters.pool) return `pool:${filters.pool}`;
  if (filters.folders?.length === 1) return `app:${filters.folders[0]}`;
  const sub = (facets.subscriptions ?? []).find((one) => filters.folders && one.profiles.every((profile) => filters.folders!.includes(profile.id)));
  return sub ? `account:${sub.key}` : "all";
}

/** `filters` with the facet `value` chosen (one at a time). */
function withFacet(filters: FeedFiltersInfo, value: string, facets: Facets): FeedFiltersInfo {
  const next: FeedFiltersInfo = { category: filters.category ?? null, state: filters.state ?? null, include_mine: filters.include_mine, project: null, pool: null, folders: null };
  const [kind, ...rest] = value.split(":");
  const id = rest.join(":");
  if (kind === "project") next.project = id;
  if (kind === "pool") next.pool = id;
  if (kind === "app") next.folders = [id];
  if (kind === "account") next.folders = (facets.subscriptions ?? []).find((one) => one.key === id)?.profiles.map((profile) => profile.id) ?? null;
  return next;
}

/** The tab a viewer's filters are on (0.5.1's Finished chip is now the
 * Finished state: `tabOf` reads it as All). */
export function tabOf(filters: FeedFiltersInfo): Tab {
  return TABS.find((one) => one.value === filters.category)?.value ?? "all";
}

/** The state chip on, if any (0.5.1's kept Finished chip included). */
export function stateOf(filters: FeedFiltersInfo): StateChip | null {
  if (filters.category === "finished") return "finished";
  return STATES.find((one) => one.value === filters.state)?.value ?? null;
}

const TAB = "rounded-full px-[11px] py-[5px] text-meta outline-none focus-visible:ring-1 focus-visible:ring-accent/60";
const TINT: Record<StateChip, string> = {
  running: "bg-live-soft text-live",
  finished: "bg-success-soft text-success",
  failed: "bg-danger-soft text-danger",
};

const NO_COUNTS: Readonly<Record<StateChip, number | null>> = { running: null, finished: null, failed: null };

export function HomeFeedFilters({ filters, onChange, facets, seen = new Map(), counts = NO_COUNTS, toReview, onReviewAll, onReadAll }: {
  filters: FeedFiltersInfo;
  onChange: (next: FeedFiltersInfo) => void;
  facets: Facets;
  seen?: Seen;
  /** Each state's count today, `null` when it isn't known yet. */
  counts?: Readonly<Record<StateChip, number | null>>;
  /** Finished sessions not reviewed yet (Mark all reviewed): the
   * backend's count, `null` until it is read (F-K2-7a, 0.4.6). */
  toReview: number | null;
  onReviewAll: () => void;
  onReadAll: () => void;
}): JSX.Element {
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const tab = tabOf(filters);
  const state = stateOf(filters);
  const { options, suggested } = facetOptions(facets, seen);
  const facet = facetOf(filters, facets);
  const setTab = (value: Tab) => onChange({ ...filters, category: value === "all" ? null : value, state: state ?? null });
  const setState = (value: StateChip) => onChange({ ...filters, category: tab === "all" ? null : tab, state: state === value ? null : value });
  return (
    <div className="flex flex-col gap-2.5" data-testid="feed-filters">
      <div className="flex items-start gap-2" data-testid="feed-toolbar">
        <fieldset className="m-0 flex min-w-0 flex-1 flex-wrap items-center gap-1.5 border-0 p-0" data-testid="feed-chip">
          <legend className="sr-only">Show in the feed</legend>
          {TABS.map((one) => (
            <button
              key={one.value}
              type="button"
              aria-pressed={tab === one.value}
              onClick={() => setTab(one.value)}
              className={TAB + (tab === one.value ? " bg-accent-soft font-semibold text-accent-hi" : " text-muted hover:bg-surface-2 hover:text-ink")}
              data-testid={`feed-chip-${one.value}`}
            >
              {one.label}
            </button>
          ))}
        </fieldset>
        <button type="button" aria-label="More feed options" aria-haspopup="menu" aria-expanded={menu !== null} onClick={(event) => setMenu(menuPoint(event))} className="btn-ghost flex size-7 shrink-0 items-center justify-center rounded-md text-label outline-none focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-more">
          ⋯
        </button>
        {menu && (
          <RowMenu heading="Feed" ariaLabel="Feed options" at={menu} onClose={() => setMenu(null)} testId="feed-menu">
            {/* 0.5.1 F60 (N8): every line carries its icon, as every menu's. */}
            <MenuItem checked={filters.include_mine} onClick={() => onChange({ ...filters, include_mine: !filters.include_mine })} testId="feed-include-mine">
              <MenuLine icon={IconProfiles} label="Include my actions" tick={filters.include_mine} />
            </MenuItem>
            {/* AJ-E1548 #19: each "mark all" says what it clears; done once chosen. */}
            <MenuItem onClick={() => { onReadAll(); setMenu(null); }} testId="feed-read-all">
              <MenuLine icon={IconRead} label="Mark all read" labelTestId="feed-read-all-label" note={MARK_HINTS.read} />
            </MenuItem>
            {(tab === "sessions" || state === "finished") && (
              <MenuItem disabled={toReview === 0} onClick={() => { onReviewAll(); setMenu(null); }} testId="feed-review-all">
                <MenuLine
                  icon={IconCheck}
                  label={toReview === null ? "Mark all reviewed" : toReview > 0 ? `Mark all ${toReview} reviewed` : "Nothing to review"}
                  labelTestId="feed-review-all-label"
                  note={toReview !== 0 ? MARK_HINTS.reviewed : undefined}
                />
              </MenuItem>
            )}
          </RowMenu>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-1.5" data-testid="feed-states">
        {STATES.map((one) => {
          const count = counts[one.value];
          const on = state === one.value;
          return (
            <button
              key={one.value}
              type="button"
              aria-pressed={on}
              onClick={() => setState(one.value)}
              title={count === null ? undefined : `${count} today`}
              className={"rounded-full px-[9px] py-0.5 text-label outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " + TINT[one.value] + (on ? " inline-flex items-center gap-1 font-semibold" : "")}
              data-testid={`feed-state-${one.value}`}
            >
              {on && <IconCheck width={11} height={11} aria-hidden="true" />}
              {one.label}
              {count !== null && <span className="tabular-nums"> {count}</span>}
            </button>
          );
        })}
        <span className="flex-1" />
        <Picker
          value={facet}
          options={options}
          groups={[{ label: "In your feed", values: suggested }]}
          onChange={(value) => onChange(withFacet(filters, value, facets))}
          // AJ-E1548 #21: closed, it says what it narrows ("Showing: Every
          // app", or the picked one with its kind); its name reads the same.
          ariaLabel="Showing"
          prefix="Showing:"
          label={facetLabel(options, facet)}
          heading="Show one"
          allHeading="All"
          searchLabel="Search Apps, Accounts and projects"
          size="sm"
          look="quiet"
          testId="feed-facet"
        />
      </div>
    </div>
  );
}
