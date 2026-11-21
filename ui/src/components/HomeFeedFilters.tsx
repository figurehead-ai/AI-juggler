/**
 * D-118 (AJ-E3702 R7) — the feed's filters, one row: the type chips as a
 * filled segmented control (All · Needs you · Finished · Switches & limits
 * · Pools & tasks · This Mac), one Project, App, Pool or Account picker
 * (the pickers rule: the most related first, then All, search past 8),
 * and a ⋯ with "Include my actions" (off by default), "Mark all read", and
 * under Finished "Mark all reviewed" ("Pick up where you left off" folded
 * in, D-118 point 3). Remembered per viewer; when filtered, a muted
 * "Filtered: … ×" line says so. The pinned zone is never filtered.
 */
import { useState, type JSX } from "react";
import { Picker, Segmented, type PickerOption } from "./controls";
import { MenuItem, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { cliAppName } from "../lib/app-name";
import { filtered } from "../lib/feed-state";
import { THIS_MACHINE } from "../lib/machine-words";
import type { FeedFiltersInfo } from "../lib/ipc/feed";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { PoolsInfo } from "../types/generated/PoolsInfo";
import type { SubscriptionInfo } from "../lib/subscriptions";
import { accountNamesApart } from "../lib/subscriptions";

type Chip = "all" | "needs" | "finished" | "switches" | "pools" | "mac";

const CHIPS: readonly { value: Chip; label: string }[] = [
  { value: "all", label: "All" },
  { value: "needs", label: "Needs you" },
  { value: "finished", label: "Finished" },
  { value: "switches", label: "Switches & limits" },
  { value: "pools", label: "Pools & tasks" },
  { value: "mac", label: THIS_MACHINE },
];

/** What the facet picker offers: a project, an App (its folder), a pool
 * or an Account (its folders), each a value `kind:id`. */
export interface Facets {
  projects: readonly ProjectInfo[];
  accounts: readonly AccountInfo[];
  subscriptions: readonly SubscriptionInfo[] | null;
  pools: PoolsInfo | null;
}

function facetOptions(facets: Facets): { options: PickerOption[]; suggested: string[] } {
  const projects = [...facets.projects].sort((a, b) => (b.last_activity_ms ?? 0) - (a.last_activity_ms ?? 0));
  const options: PickerOption[] = [{ value: "all", label: "Everything", pinned: true }];
  for (const project of projects) options.push({ value: `project:${project.id}`, label: project.name, hint: "Project" });
  for (const account of facets.accounts) options.push({ value: `app:${account.id}`, label: cliAppName(account.provider, account.label), hint: "App" });
  for (const pool of facets.pools?.pools ?? []) options.push({ value: `pool:${pool.id}`, label: pool.name, hint: "Pool" });
  const named = accountNamesApart(facets.subscriptions ?? []);
  for (const sub of facets.subscriptions ?? []) if (sub.profiles.length > 1) options.push({ value: `account:${sub.key}`, label: named(sub), hint: "Account" });
  return { options, suggested: projects.slice(0, 4).map((project) => `project:${project.id}`) };
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
  const next: FeedFiltersInfo = { category: filters.category ?? null, include_mine: filters.include_mine, project: null, pool: null, folders: null };
  const [kind, ...rest] = value.split(":");
  const id = rest.join(":");
  if (kind === "project") next.project = id;
  if (kind === "pool") next.pool = id;
  if (kind === "app") next.folders = [id];
  if (kind === "account") next.folders = (facets.subscriptions ?? []).find((one) => one.key === id)?.profiles.map((profile) => profile.id) ?? null;
  return next;
}

export function HomeFeedFilters({ filters, onChange, facets, toReview, onReviewAll, onReadAll }: {
  filters: FeedFiltersInfo;
  onChange: (next: FeedFiltersInfo) => void;
  facets: Facets;
  /** Finished sessions not reviewed yet (Mark all reviewed): the
   * backend's count, `null` until it is read, so no number is said
   * before it is counted (F-K2-7a, 0.4.6). */
  toReview: number | null;
  onReviewAll: () => void;
  onReadAll: () => void;
}): JSX.Element {
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const chip: Chip = CHIPS.find((one) => one.value === filters.category)?.value ?? "all";
  const { options, suggested } = facetOptions(facets);
  const facet = facetOf(filters, facets);
  const facetName = options.find((option) => option.value === facet)?.label ?? null;
  const chipName = CHIPS.find((one) => one.value === chip)?.label ?? "All";
  const what = [chip !== "all" ? chipName : null, facet !== "all" ? facetName : null].filter(Boolean).join(" · ");
  return (
    <div className="flex flex-col gap-1.5" data-testid="feed-filters">
      {/* The ⋯ holds the row's end at every feed width (down to D-118's
          480 px column): the chips and the picker wrap among themselves
          beside it, the picker under the chips first, so no line is left
          holding the ⋯ alone (the empty band at 1200 with the rail open). */}
      <div className="flex items-start gap-2" data-testid="feed-toolbar">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-1.5">
          <Segmented label="Show in the feed" value={chip} segments={CHIPS} wrap testId="feed-chip" onChange={(value) => onChange({ ...filters, category: value === "all" ? null : value })} />
          <Picker
            value={facet}
            options={options}
            groups={[{ label: "Recent projects", values: suggested }]}
            onChange={(value) => onChange(withFacet(filters, value, facets))}
            ariaLabel="Show one project, App, pool or Account"
            heading="Show one"
            allHeading="All"
            searchLabel="Search projects, Apps and pools"
            size="sm"
            look="quiet"
            testId="feed-facet"
          />
        </div>
        <button type="button" aria-label="More feed options" aria-haspopup="menu" aria-expanded={menu !== null} onClick={(event) => setMenu(menuPoint(event))} className="btn-ghost flex size-7 shrink-0 items-center justify-center rounded-md text-label outline-none focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="feed-more">
          ⋯
        </button>
        {menu && (
          <RowMenu heading="Feed" ariaLabel="Feed options" at={menu} onClose={() => setMenu(null)} testId="feed-menu">
            <MenuItem checked={filters.include_mine} onClick={() => onChange({ ...filters, include_mine: !filters.include_mine })} testId="feed-include-mine">
              Include my actions
            </MenuItem>
            <MenuItem onClick={onReadAll} testId="feed-read-all">Mark all read</MenuItem>
            {chip === "finished" && (
              <MenuItem disabled={toReview === 0} onClick={onReviewAll} testId="feed-review-all">
                {toReview === null ? "Mark all reviewed" : toReview > 0 ? `Mark all ${toReview} reviewed` : "Nothing to review"}
              </MenuItem>
            )}
          </RowMenu>
        )}
      </div>
      {filtered(filters) && (
        <p className="flex items-center gap-1.5 text-label text-muted" data-testid="feed-filtered">
          Filtered: {what}
          <button type="button" aria-label="Show everything" onClick={() => onChange({ include_mine: filters.include_mine })} className="rounded px-1 outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
            ×
          </button>
          <span className="text-muted">· Needs you now shows every need</span>
        </p>
      )}
    </div>
  );
}
