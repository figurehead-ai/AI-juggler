/**
 * AJ-E2104 — the rail's toolbar: search (⌘K focuses it; it takes the
 * View's words: `is:ended app:claude …`) and the View button. 0.3: under
 * them, the grouping as a visible switch — App · Project · Account, and
 * Pool once a pool of yours exists (a project's own pool doesn't count)
 * — one click, kept per Mac with the rest of the View.
 */
import { useMemo, type JSX, type RefObject } from "react";
import { ViewPopover } from "./ViewPopover";
import { Segmented, type Segment } from "./controls";
import { useConversations } from "../lib/conversations";
import { SOME_APP, itemsOf } from "../lib/rail-items";
import { ONLY_KEYS, onlyValue, setGroupBy, useRailView, type OnlyKey } from "../lib/rail-view";
import { hasUserPools, usePools } from "../lib/pools";
import type { GroupBy } from "../lib/rail-places";
import type { InstanceInfo, ProjectInfo, RemoteMachineInfo } from "../lib/bridge";

/** Each Only list: the values the rail has now, sorted. */
export function onlyOptions(
  sessions: InstanceInfo[],
  projects: ProjectInfo[],
  conversations: Parameters<typeof itemsOf>[0]["conversations"],
  machines: RemoteMachineInfo[],
): Record<OnlyKey, { value: string; label: string }[]> {
  const items = itemsOf({ sessions, conversations: conversations.filter((c) => c.live || c.inactive), projects, seen: {} });
  const out = {} as Record<OnlyKey, { value: string; label: string }[]>;
  for (const key of ONLY_KEYS) {
    const values = [...new Set(items.map((item) => onlyValue(item, key)).filter(Boolean))].sort((a, b) => a.localeCompare(b));
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
  const options = useMemo(() => onlyOptions(sessions, projects, all, machines), [sessions, projects, all, machines]);
  return (
    <div className="flex flex-col gap-2" data-testid="rail-toolbar">
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
            className="h-[30px] w-full rounded-lg border border-hairline bg-surface/70 pl-6 pr-2 text-meta text-ink outline-none placeholder:text-muted focus-visible:ring-1 focus-visible:ring-accent/60"
          />
        </div>
        <ViewPopover options={options} />
      </div>
      <GroupSwitch />
    </div>
  );
}
