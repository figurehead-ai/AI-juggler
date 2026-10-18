/**
 * Rail view logic (AJ-E404): needs-me-first sorting, multi-filter chips,
 * ⌘K search, group-by rollups, and I-10 snooze/mute — pure functions so
 * the order contracts are unit-testable, plus snooze persistence.
 *
 * Fleet-shaped (D-13): the filter model carries `machine` as a dimension
 * from day one; with one local machine it simply never splits.
 */

import type { AccountInfo, InstanceInfo, ProjectInfo } from "./bridge";
import { renewalLabel, renewalTieBreak } from "./renewal";

export type SortMode = "needs" | "quota" | "project" | "provider" | "account" | "age" | "name";
export type GroupBy = "none" | "project" | "account" | "provider";
export type ChipId = "needs" | "running" | "ended";

/** Menu labels — the quota sort is honest about its input (AJ-E409 pending). */
export const SORT_LABELS: { id: SortMode; label: string }[] = [
  { id: "needs", label: "Needs me" },
  { id: "quota", label: "Quota risk (quota only)" },
  { id: "project", label: "Project" },
  { id: "provider", label: "Provider" },
  { id: "account", label: "Account" },
  { id: "age", label: "Age" },
  { id: "name", label: "Name" },
];

export const GROUP_LABELS: { id: GroupBy; label: string }[] = [
  { id: "none", label: "None" },
  { id: "project", label: "Project" },
  { id: "account", label: "Account" },
  { id: "provider", label: "Provider" },
];

export type ChipFilters = { chips: ChipId[]; provider: string | null; account: string | null; project: string | null; machine: string | null };

export const NO_CHIP_FILTERS: ChipFilters = { chips: [], provider: null, account: null, project: null, machine: null };

// --- severity (Needs-me sort) --------------------------------------------------

/** Attention severity rank: lower = more urgent (Req.1 order). */
export function severityRank(row: InstanceInfo): number {
  switch (row.needs_attention) {
    case "failed": return 0;
    case "quota_blocked": return 1;
    case "waiting_user": return 2;
    case "stuck": return 3;
    default:
      return row.status === "running" || row.status === "starting" ? 4 : 5;
  }
}

/** Waiting duration in ms (how long the instance has waited for the user). */
function waitingMs(row: InstanceInfo, now: number): number {
  return row.last_activity_ms !== null ? Math.max(0, now - row.last_activity_ms) : 0;
}

/** AJ-E411 Req.5: the account-expiry tie-break rank for a session row. */
function expiryTieBreak(row: InstanceInfo, accounts: AccountInfo[], now: number): number {
  const account = accounts.find((entry) => entry.id === row.account_id);
  if (!account?.renewal_date) return 2;
  return renewalTieBreak(renewalLabel(account.renewal_date, now));
}

// --- snooze ----------------------------------------------------------------------

export type SnoozeState = Record<string, { until: number | null; muted: boolean }>;

const SNOOZE_KEY = "aij.snooze";

export function readSnooze(storage: Storage = localStorage): SnoozeState {
  try {
    const raw = storage.getItem(SNOOZE_KEY);
    return raw ? (JSON.parse(raw) as SnoozeState) : {};
  } catch {
    return {};
  }
}

export function storeSnooze(state: SnoozeState, storage: Storage = localStorage): void {
  storage.setItem(SNOOZE_KEY, JSON.stringify(state));
}

/** Is the row currently snoozed (muted = forever)? */
export function isSnoozed(row: InstanceInfo, snooze: SnoozeState, now: number): boolean {
  const entry = snooze[row.id];
  if (!entry) return false;
  if (entry.muted) return true;
  return entry.until !== null && entry.until > now;
}

/** Remaining snooze caption (`snoozed 12m`, `muted`) or null. */
export function snoozeCaption(row: InstanceInfo, snooze: SnoozeState, now: number): string | null {
  const entry = snooze[row.id];
  if (!entry) return null;
  if (entry.muted) return "muted";
  if (entry.until === null || entry.until <= now) return null;
  const minutes = Math.max(1, Math.round((entry.until - now) / 60_000));
  return minutes >= 60 ? `snoozed ${Math.floor(minutes / 60)}h${minutes % 60}m` : `snoozed ${minutes}m`;
}

// --- sorting ----------------------------------------------------------------------

export function sortSessions(
  rows: InstanceInfo[],
  mode: SortMode,
  snooze: SnoozeState,
  now: number,
  projects: ProjectInfo[] = [],
  accounts: AccountInfo[] = [],
): InstanceInfo[] {
  const projectName = projectNames(projects);
  const snoozed = (row: InstanceInfo) => (isSnoozed(row, snooze, now) ? 1 : 0);
  const sorted = [...rows];
  switch (mode) {
    case "needs":
      // Failed > QuotaBlocked > WaitingUser (wait desc) > Stuck > Running
      // > rest; snoozed sink below unsnoozed peers (Req.1/Req.6).
      // AJ-E411 Req.5: account expiry joins ONLY at <= 7 days and ONLY
      // as a tie-break below the live attention states — it never
      // outranks a blocked agent (severityRank already ordered those).
      sorted.sort(
        (a, b) =>
          severityRank(a) - severityRank(b) ||
          snoozed(a) - snoozed(b) ||
          waitingMs(b, now) - waitingMs(a, now) ||
          expiryTieBreak(a, accounts, now) - expiryTieBreak(b, accounts, now) ||
          a.id.localeCompare(b.id),
      );
      break;
    case "quota": {
      // Lowest % left first; unknown quota would group under `no data`
      // (grouping applied by the view); among known rows, snoozed sink.
      sorted.sort((a, b) => {
        const knownA = a.quota !== null;
        const knownB = b.quota !== null;
        if (knownA !== knownB) return knownA ? -1 : 1;
        if (knownA && knownB) {
          const leftA = 100 - a.quota!.used_percentage;
          const leftB = 100 - b.quota!.used_percentage;
          if (leftA !== leftB) return leftA - leftB;
        }
        return snoozed(a) - snoozed(b) || a.id.localeCompare(b.id);
      });
      break;
    }
    case "project":
      sorted.sort((a, b) => key(a, projectName).localeCompare(key(b, projectName)) || snoozed(a) - snoozed(b));
      break;
    case "provider":
      sorted.sort((a, b) => a.provider.localeCompare(b.provider) || snoozed(a) - snoozed(b));
      break;
    case "account":
      sorted.sort((a, b) => a.account_label.localeCompare(b.account_label) || snoozed(a) - snoozed(b));
      break;
    case "age": // newest first (most recent start at top)
      sorted.sort((a, b) => b.started_at_ms - a.started_at_ms || snoozed(a) - snoozed(b));
      break;
    case "name":
      sorted.sort((a, b) => a.account_label.localeCompare(b.account_label) || a.id.localeCompare(b.id));
      break;
  }
  return sorted;
}

function projectNames(projects: ProjectInfo[]): Map<string, string> {
  return new Map(projects.map((project) => [project.id, project.name]));
}

function key(row: InstanceInfo, names: Map<string, string>): string {
  return (row.project_id !== null ? names.get(row.project_id) : undefined) ?? "—";
}

// --- filtering ----------------------------------------------------------------------

export function filterSessions(
  rows: InstanceInfo[],
  filters: ChipFilters,
  query: string,
  projects: ProjectInfo[] = [],
): InstanceInfo[] {
  const projectName = projectNames(projects);
  const needle = query.trim().toLowerCase();
  return rows.filter((row) => {
    // Chip group (union within the group); empty chips = All.
    if (filters.chips.length > 0) {
      const matches = filters.chips.some((chip) => {
        switch (chip) {
          case "needs": return row.needs_attention !== null;
          case "running": return row.status === "running" || row.status === "starting";
          case "ended": return ["finished", "stopped", "failed", "handed_over"].includes(row.status);
        }
      });
      if (!matches) return false;
    }
    if (filters.provider !== null && row.provider !== filters.provider) return false;
    if (filters.account !== null && row.account_id !== filters.account) return false;
    if (filters.project !== null && row.project_id !== filters.project) return false;
    // Fleet dimension (D-13): the key exists and filters from day one.
    if (filters.machine !== null && row.machine_id !== filters.machine) return false;
    if (needle !== "") {
      const haystack = [
        row.account_label,
        row.provider,
        row.model ?? "",
        row.git_branch ?? "",
        (row.project_id !== null ? projectName.get(row.project_id) : undefined) ?? "",
        row.machine_id,
      ]
        .join(" ")
        .toLowerCase();
      if (!haystack.includes(needle)) return false;
    }
    return true;
  });
}

export function chipCounts(rows: InstanceInfo[]): Record<ChipId, number> {
  return {
    needs: rows.filter((row) => row.needs_attention !== null).length,
    running: rows.filter((row) => row.status === "running" || row.status === "starting").length,
    ended: rows.filter((row) => ["finished", "stopped", "failed", "handed_over"].includes(row.status)).length,
  };
}

/** Unsnoozed attention count — the only number the rail badge shows (Req.6). */
export function unsnoozedNeedsCount(rows: InstanceInfo[], snooze: SnoozeState, now: number): number {
  return rows.filter((row) => row.needs_attention !== null && !isSnoozed(row, snooze, now)).length;
}

// --- grouping ------------------------------------------------------------------------

export type RowGroup = { id: string; label: string; rollup: string; rows: InstanceInfo[] };

/**
 * Group sorted rows (Needs-me order applies INSIDE groups, Req.5) with
 * rollup copy `N running - M needs you - lowest quota X%`.
 */
export function groupSessions(sorted: InstanceInfo[], by: GroupBy, projects: ProjectInfo[]): RowGroup[] {
  if (by === "none") {
    return [{ id: "__all", label: "", rollup: "", rows: sorted }];
  }
  const projectName = projectNames(projects);
  const buckets = new Map<string, InstanceInfo[]>();
  const labels = new Map<string, string>();
  for (const row of sorted) {
    let groupKey = "—";
    let label = "No project";
    if (by === "project") {
      groupKey = row.project_id ?? "none";
      label = row.project_id !== null ? (projectName.get(row.project_id) ?? "Unknown project") : "No project";
    } else if (by === "account") {
      groupKey = row.account_id ?? row.account_label;
      label = row.account_label;
    } else {
      groupKey = row.provider;
      label = row.provider;
    }
    if (!buckets.has(groupKey)) {
      buckets.set(groupKey, []);
      labels.set(groupKey, label);
    }
    buckets.get(groupKey)!.push(row);
  }
  return [...buckets.entries()].map(([id, rows]) => ({
    id,
    label: labels.get(id) ?? id,
    rollup: rollupOf(rows),
    rows,
  }));
}

export function rollupOf(rows: InstanceInfo[]): string {
  const running = rows.filter((row) => row.status === "running" || row.status === "starting").length;
  const needs = rows.filter((row) => row.needs_attention !== null).length;
  const parts = [`${running} running`];
  if (needs > 0) parts.push(`${needs} needs you`);
  const lowestLeft = rows
    .filter((row) => row.quota !== null)
    .reduce((min, row) => Math.min(min, 100 - row.quota!.used_percentage), Number.POSITIVE_INFINITY);
  if (Number.isFinite(lowestLeft)) parts.push(`lowest quota ${Math.round(lowestLeft)}%`);
  return parts.join(" - ");
}
