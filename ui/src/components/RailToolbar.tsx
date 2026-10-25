/**
 * AJ-E2104 — the rail's toolbar: search (⌘K focuses it; it takes the
 * View's words: `is:ended app:claude …`) and the View button.
 */
import { useMemo, type JSX, type RefObject } from "react";
import { ViewPopover } from "./ViewPopover";
import { useConversations } from "../lib/conversations";
import { itemsOf } from "../lib/rail-items";
import { ONLY_KEYS, onlyValue, type OnlyKey } from "../lib/rail-view";
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
    out[key] = values.map((value) => ({ value, label: value === "this-mac" ? "This Mac" : value }));
  }
  if (machines.length > 0) {
    out.mac = [{ value: "this-mac", label: "This Mac" }, ...machines.map((machine) => ({ value: machine.machine_id, label: machine.label }))];
  }
  return out;
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
    <div className="flex items-center gap-1.5" data-testid="rail-toolbar">
      <div className="relative min-w-0 flex-1">
        <span aria-hidden="true" className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-faint">
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
          className="h-[30px] w-full rounded-lg border border-hairline bg-surface/70 pl-6 pr-2 text-[11.5px] text-ink outline-none placeholder:text-faint focus-visible:ring-1 focus-visible:ring-accent/60"
        />
      </div>
      <ViewPopover options={options} />
    </div>
  );
}
