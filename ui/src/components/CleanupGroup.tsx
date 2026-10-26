import { useState } from "react";
import { ProcessRow, ProcessRowsHeader, type CleanupRowInfo } from "./ProcessRow";
import { sessionTitle } from "./ManageWork";
import type { InstanceInfo } from "../lib/bridge";
import { providerDisplayName } from "../lib/providers";
import { agoWords } from "../lib/time-words";
import { startedWords } from "../lib/session-titles";

/**
 * One session's group on the Cleanup screen (AJ-E506): a title that
 * names the session and how it ended, then the shared rows under one
 * set of column headings.
 */

export interface CleanupGroupInfo {
  session: string;
  provider: string;
  account: string | null;
  project: string | null;
  machine: string;
  healthy: boolean;
  /** Names, not ids, for the title (absent from older payloads). */
  account_label?: string | null;
  project_name?: string | null;
  /** The user's name for the session. */
  label?: string | null;
  /** How it ended: the session's status (`failed`, `stopped`, …). */
  status?: string | null;
  started_at_ms?: number | null;
  /** True while the session's own process runs. */
  live?: boolean;
  ended_at_ms?: number | null;
  rows: CleanupRowInfo[];
}

/** How a session ended, in words ("crashed"); null when nothing says. */
function endedHow(status: string | null | undefined): string | null {
  switch (status) {
    case "failed":
      return "crashed";
    case "stopped":
      return "stopped";
    case "finished":
      return "finished";
    case "quota_blocked":
      return "hit its limit";
    case "handed_over":
      return "moved on";
    default:
      return null;
  }
}

/** A group's title names the session and how it ended: "web-app · Fix
 * the login test · ended 2 h ago (crashed)" — never provider, account or
 * project ids. */
export function groupTitle(group: CleanupGroupInfo, session: InstanceInfo | undefined, now: number): string {
  const project = group.project_name ?? null;
  const named =
    group.label ??
    (session ? sessionTitle(session) : null) ??
    (group.started_at_ms ? `Session ${startedWords(group.started_at_ms, now)}` : "A session");
  const how = endedHow(group.status);
  const adopted = group.rows.some((row) => row.state === "adopted");
  const state =
    (group.live ?? group.healthy)
      ? adopted
        ? "picked up after restart"
        : "running"
      : group.ended_at_ms
        ? `ended ${agoWords(group.ended_at_ms, now)}${how ? ` (${how})` : ""}`
        : how !== null
          ? `ended (${how})`
          : "ended";
  return [project, named === project ? null : named, state].filter((part) => part !== null).join(" · ");
}

/** A machine in words: This Mac, else its paired name, else its id. */
export function machineWords(id: string, local: string | undefined, names: Record<string, string>): string {
  if (local !== undefined && local !== "" && id === local) return "This Mac";
  return names[id] ?? id;
}

export function GroupBlock(props: {
  group: CleanupGroupInfo;
  /** AJ-E511: this node's machine id — foreign rows are display-only. */
  localMachine?: string;
  /** The machine's name, shown only when more than one Mac is in view. */
  machineName?: (id: string) => string;
  /** The session behind the group, when the app has it (its title). */
  session?: InstanceInfo;
  now: number;
  onKill?: (row: CleanupRowInfo, session: string, trigger: HTMLElement) => void;
}) {
  const group = props.group;
  const [collapsed, setCollapsed] = useState(group.healthy);
  const who = [providerDisplayName(group.provider), group.account_label ?? null].filter(Boolean).join(" · ");
  return (
    <section
      className="flex flex-col rounded-xl border border-hairline bg-panel p-3"
      data-testid={`cleanup-group-${group.session}`}
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => setCollapsed((open) => !open)}
          aria-expanded={!collapsed}
          className="flex min-w-0 flex-1 items-center gap-2 rounded-md px-1 py-0.5 text-left outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          <span aria-hidden="true" className="w-3 text-center text-label text-muted">
            {collapsed ? "▸" : "▾"}
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-body font-medium text-ink" data-testid={`cleanup-group-title-${group.session}`}>
              {groupTitle(group, props.session, props.now)}
            </span>
            <span className="truncate text-label text-muted">
              {who}
              {props.machineName !== undefined && ` · ${props.machineName(group.machine)}`}
              {` · ${group.rows.length} ${group.rows.length === 1 ? "process" : "processes"}`}
            </span>
          </span>
          {group.healthy && (
            <span className="ml-auto rounded-full border border-hairline px-1.5 py-[1px] text-label text-muted">
              healthy
            </span>
          )}
        </button>
      </div>
      {!collapsed && (
        <div className="mt-2">
          <ProcessRowsHeader />
          <ul>
            {group.rows.map((row) => (
              <ProcessRow
                key={row.pid}
                row={row}
                now={props.now}
                machineLabel={
                  props.machineName !== undefined && row.machine_id !== undefined
                    ? props.machineName(row.machine_id)
                    : undefined
                }
                onKill={
                  // AJ-E511: foreign-machine rows are display-only —
                  // never even an affordance. Rows WITHOUT a machine id
                  // are legacy-local: the backend stamps every row now.
                  props.onKill !== undefined &&
                  !(
                    row.machine_id !== undefined &&
                    props.localMachine !== undefined &&
                    row.machine_id !== props.localMachine
                  )
                    ? (row, trigger) => props.onKill?.(row, group.session, trigger)
                    : undefined
                }
              />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

