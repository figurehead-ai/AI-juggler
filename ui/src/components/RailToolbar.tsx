/**
 * AJ-E2104 — the rail's toolbar: search (⌘K focuses it; it takes the
 * View's words: `is:ended app:claude …`) and the View button. 0.3: under
 * them, the grouping as a visible switch — App · Project · Account, and
 * Pool once a pool of yours exists (a project's own pool doesn't count)
 * — one click, kept per Mac with the rest of the View.
 */
import { useMemo, useState, type JSX, type RefObject } from "react";
import { useRailAccountOf } from "../lib/rail-accounts";
import { ViewPopover } from "./ViewPopover";
import { Segmented, type Segment } from "./controls";
import { useConversations } from "../lib/conversations";
import { SOME_APP, itemsOf } from "../lib/rail-items";
import { ONLY_KEYS, onlyValue, setGroupBy, useRailView, type OnlyKey } from "../lib/rail-view";
import { hasUserPools, usePools } from "../lib/pools";
import type { AccountOf, GroupBy } from "../lib/rail-places";
import type { InstanceInfo, ProjectInfo, RemoteMachineInfo } from "../lib/bridge";

/** Each Only list: the values the rail has now, sorted. */
export function onlyOptions(
  sessions: InstanceInfo[],
  projects: ProjectInfo[],
  conversations: Parameters<typeof itemsOf>[0]["conversations"],
  machines: RemoteMachineInfo[],
  accountOf?: AccountOf,
): Record<OnlyKey, { value: string; label: string }[]> {
  const items = itemsOf({ sessions, conversations: conversations.filter((c) => c.live || c.inactive), projects, seen: {} });
  const out = {} as Record<OnlyKey, { value: string; label: string }[]>;
  for (const key of ONLY_KEYS) {
    const values = [...new Set(items.map((item) => onlyValue(item, key, accountOf)).filter(Boolean))].sort((a, b) => a.localeCompare(b));
    // The 0.2 walk (F18): "An app" as an app to pick meant nothing.
    out[key] = values.map((value) => ({ value, label: value === "this-mac" ? "This Mac" : key === "app" && value === SOME_APP ? "Not open in an app" : value }));
  }
  if (machines.length > 0) {
    out.mac = [{ value: "this-mac", label: "This Mac" }, ...machines.map((machine) => ({ value: machine.machine_id, label: machine.label }))];
  }
  return out;
}

/** The switch's choices, in its order ("place" is said "App"). */
const GROUP_SEGMENTS: Segment<GroupBy>[] = [
  { value: "place", label: "App", title: "Each app and its profiles" },
  { value: "project", label: "Project", title: "Each project's sessions together" },
  { value: "account", label: "Account", title: "Each subscription, its profiles together" },
  { value: "pool", label: "Pool", title: "Each pool of yours" },
];

/** The rail's grouping switch (0.3). A Pool kept as picked with no pool
 * of yours shows App, as the rail does. */
export function GroupSwitch(): JSX.Element {
  const view = useRailView();
  const withPools = hasUserPools(usePools());
  const segments = withPools ? GROUP_SEGMENTS : GROUP_SEGMENTS.filter((segment) => segment.value !== "pool");
  const grouped = view.group === "pool" && !withPools ? "place" : view.group;
  return <Segmented label="Group sessions by" value={grouped} segments={segments} onChange={setGroupBy} testId="rail-group" />;
}

export function RailToolbar({
  query,
  onQuery,
  searchRef,
  sessions,
  projects,
  machines,
}: {
  query: string;
  onQuery: (query: string) => void;
  searchRef: RefObject<HTMLInputElement | null>;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  machines: RemoteMachineInfo[];
}): JSX.Element {
  const { all } = useConversations();
  // The Accounts to pick from are read once the View opens (or one is picked).
  const [viewOpen, setViewOpen] = useState(false);
  const picked = useRailView().only.account;
  const accountOf = useRailAccountOf(viewOpen || picked !== undefined);
  const options = useMemo(() => onlyOptions(sessions, projects, all, machines, accountOf), [sessions, projects, all, machines, accountOf]);
  return (
    // 0.3 polish: a shorter stack before the first session (the review:
    // ~150 px of search, switch and heads), every control kept, each
    // target 24 px or more.
    <div className="flex flex-col gap-1.5" data-testid="rail-toolbar">
      <div className="flex items-center gap-1.5">
        <div className="relative min-w-0 flex-1">
          <span aria-hidden="true" className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted">
            ⌕
          </span>
          <input
            ref={searchRef}
            type="search"
            value={query}
            onChange={(event) => onQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                onQuery("");
              }
            }}
            placeholder="Search ⌘K"
            aria-label="Search sessions: words, or is:ended, app:, profile:, account:, project:, mac:"
            className="h-7 w-full rounded-lg border border-hairline bg-surface/70 pl-6 pr-2 text-meta text-ink outline-none placeholder:text-muted focus-visible:ring-1 focus-visible:ring-accent/60"
          />
        </div>
        <ViewPopover options={options} onOpenChange={setViewOpen} />
      </div>
      <GroupSwitch />
    </div>
  );
}
