/**
 * AJ-E1005 — morning triage: one sorted view across every project of
 * what needs the user, what is parked until a reset, what is running and
 * what finished but has not been looked at. Grouped by the action it
 * needs, not by tool. Each row is the E4 rail row (same indicators) plus
 * one primary action, run in place. Fully keyboard operable: j/k or the
 * arrows move, Enter opens, A runs the row's action.
 *
 * A view over E4 data, not a second model: the only state it adds is the
 * explicit "reviewed" step, recorded in the Instance's history.
 */
import { useCallback, useMemo, useRef, useState, type KeyboardEvent, type JSX } from "react";
import { SessionRow } from "../components/WorkflowRail";
import { parkedWords } from "../lib/parked";
import type { Density } from "../lib/density";
import {
  errorText,
  instanceMarkReviewed,
  instancePark,
  parkedResume,
  type InstanceInfo,
  type ProjectInfo,
} from "../lib/bridge";

/** Ended Instances older than this are left to search/history (AJ-E1007). */
const FINISHED_WINDOW_MS = 7 * 24 * 3_600_000;

type GroupId = "needs_you" | "parked" | "running" | "finished";
type SortId = "waiting" | "project" | "quota";

const GROUPS: Array<{ id: GroupId; title: string; icon: string; empty: string }> = [
  { id: "needs_you", title: "Needs you", icon: "!", empty: "Nothing needs you." },
  { id: "parked", title: "Parked until a reset", icon: "⏸", empty: "Nothing parked." },
  { id: "running", title: "Running", icon: "▶", empty: "Nothing running." },
  { id: "finished", title: "Finished — not reviewed", icon: "✓", empty: "Nothing left to review." },
];

const SORTS: Array<{ id: SortId; label: string }> = [
  { id: "waiting", label: "Waiting longest" },
  { id: "project", label: "Project" },
  { id: "quota", label: "Quota risk" },
];

interface Action {
  label: string;
  run: "open" | "park" | "resume" | "reviewed";
}

/** The one primary action a row offers (Enter always opens). */
export function primaryAction(session: InstanceInfo, now: number): Action {
  if (session.parked) {
    const words = parkedWords(session, now);
    return words?.due ? { label: "Resume", run: "resume" } : { label: "Open", run: "open" };
  }
  if (session.status === "quota_blocked") return { label: "Park until reset", run: "park" };
  if (session.status === "finished" || session.status === "stopped") {
    return { label: "Mark reviewed", run: "reviewed" };
  }
  return { label: "Open", run: "open" };
}

/** Which triage group an Instance belongs to, or null when it is not
 * triage work (reviewed, archived, handed over, or ended long ago). */
export function triageGroup(session: InstanceInfo, now: number): GroupId | null {
  if (session.parked) return "parked";
  switch (session.status) {
    case "waiting_user":
    case "stuck":
    case "quota_blocked":
    case "failed":
      return "needs_you";
    case "starting":
    case "running":
      return "running";
    case "finished":
    case "stopped": {
      if (session.lifecycle === "reviewed" || session.lifecycle === "archived") return null;
      const ended = session.last_activity_ms ?? session.started_at_ms;
      return now - ended <= FINISHED_WINDOW_MS ? "finished" : null;
    }
    default:
      return null;
  }
}

/** When it started waiting on the user (oldest first = oldest blocker). */
function waitingSince(session: InstanceInfo): number {
  return session.attention?.since_ms ?? session.last_activity_ms ?? session.started_at_ms;
}

/** Quota risk, most at risk first: a parked Instance by its soonest
 * reset; anything else by the share of its window used (unmeasured
 * last — never guessed). */
function byQuotaRisk(a: InstanceInfo, b: InstanceInfo): number {
  if (a.parked || b.parked) {
    const at = (row: InstanceInfo) => (row.parked ? (row.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER);
    return at(a) - at(b);
  }
  return (b.quota?.used_percentage ?? -1) - (a.quota?.used_percentage ?? -1);
}

export function sortRows(
  rows: InstanceInfo[],
  sort: SortId,
  projectName: (session: InstanceInfo) => string,
): InstanceInfo[] {
  const copy = [...rows];
  if (sort === "waiting") copy.sort((a, b) => waitingSince(a) - waitingSince(b));
  if (sort === "project") {
    copy.sort((a, b) => projectName(a).localeCompare(projectName(b)) || waitingSince(a) - waitingSince(b));
  }
  if (sort === "quota") copy.sort((a, b) => byQuotaRisk(a, b) || waitingSince(a) - waitingSince(b));
  return copy;
}

export function Triage({
  sessions,
  projects,
  density,
  onOpenSession,
  onChanged,
  now = Date.now(),
}: {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  density: Density;
  onOpenSession: (id: string) => void;
  onChanged: () => void;
  now?: number;
}): JSX.Element {
  const [sort, setSort] = useState<SortId>("waiting");
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const names = useMemo(() => new Map(projects.map((project) => [project.id, project.name])), [projects]);
  const projectName = useCallback(
    (session: InstanceInfo) => (session.project_id ? (names.get(session.project_id) ?? "") : ""),
    [names],
  );

  const grouped = useMemo(() => {
    const out: Record<GroupId, InstanceInfo[]> = { needs_you: [], parked: [], running: [], finished: [] };
    for (const session of sessions) {
      const group = triageGroup(session, now);
      if (group) out[group].push(session);
    }
    for (const group of GROUPS) out[group.id] = sortRows(out[group.id], sort, projectName);
    return out;
  }, [sessions, now, sort, projectName]);
  const total = GROUPS.reduce((sum, group) => sum + grouped[group.id].length, 0);

  const run = useCallback(
    async (session: InstanceInfo) => {
      const action = primaryAction(session, now);
      if (action.run === "open") {
        onOpenSession(session.id);
        return;
      }
      setBusy(session.id);
      try {
        if (action.run === "park") await instancePark(session.id);
        if (action.run === "resume") await parkedResume(session.id);
        if (action.run === "reviewed") await instanceMarkReviewed(session.id);
        setStatus(`${session.account_label}: ${action.label.toLowerCase()} — done.`);
        onChanged();
      } catch (cause) {
        setStatus(`${session.account_label}: ${errorText(cause)}`);
      } finally {
        setBusy(null);
      }
    },
    [now, onOpenSession, onChanged],
  );

  // Roving focus: j/k or the arrows move between rows; Enter opens (the
  // row is a button); A runs the focused row's action.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement;
    if (target.tagName === "SELECT" || target.tagName === "INPUT") return;
    const rows = Array.from(listRef.current?.querySelectorAll<HTMLElement>("[data-triage-row]") ?? []);
    const at = rows.findIndex((row) => row.contains(document.activeElement));
    const focusRow = (index: number) =>
      rows[Math.max(0, Math.min(rows.length - 1, index))]?.querySelector<HTMLElement>("button")?.focus();
    if (event.key === "j" || event.key === "ArrowDown") {
      event.preventDefault();
      focusRow(at + 1);
    } else if (event.key === "k" || event.key === "ArrowUp") {
      event.preventDefault();
      focusRow(at < 0 ? 0 : at - 1);
    } else if ((event.key === "a" || event.key === "A") && at >= 0) {
      event.preventDefault();
      const session = sessions.find((row) => row.id === rows[at].dataset.triageRow);
      if (session) void run(session);
    }
  };

  const rowDensity = density === "comfortable" ? "comfortable" : "compact";
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-4" data-testid="triage">
      <header className="flex flex-wrap items-end gap-3">
        <div className="min-w-0 flex-1">
          <h2 className="text-lg font-semibold tracking-tight">Triage</h2>
          <p className="text-xs text-muted">
            Every project, grouped by what each session needs from you.{" "}
            <span className="text-ink">J / K</span> or arrows to move, <span className="text-ink">Enter</span> opens,{" "}
            <span className="text-ink">A</span> runs the row's action.
          </p>
        </div>
        <label className="flex items-center gap-2 text-[11px] text-muted">
          Sort
          <select
            value={sort}
            onChange={(event) => setSort(event.target.value as SortId)}
            className="rounded-md border border-hairline bg-surface px-2 py-1 text-[11px] text-ink"
            data-testid="triage-sort"
          >
            {SORTS.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      </header>
      <p className="sr-only" aria-live="polite" data-testid="triage-status">
        {status}
      </p>
      {total === 0 && (
        <p className="rounded-xl border border-dashed border-hairline p-6 text-center text-sm text-muted" data-testid="triage-empty">
          Nothing to triage — nothing needs you, nothing is parked or running, and everything finished has been reviewed.
        </p>
      )}
      <div ref={listRef} onKeyDown={onKeyDown} className="flex flex-col gap-4">
        {GROUPS.map((group) => {
          const rows = grouped[group.id];
          return (
            <section key={group.id} aria-labelledby={`triage-${group.id}`} data-testid={`triage-group-${group.id}`}>
              <h3
                id={`triage-${group.id}`}
                className="mb-1.5 flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted"
              >
                <span aria-hidden="true">{group.icon}</span>
                {group.title}
                <span className="font-normal normal-case tracking-normal">({rows.length})</span>
              </h3>
              {rows.length === 0 ? (
                <p className="px-3 py-1 text-[11px] text-muted">{group.empty}</p>
              ) : (
                <ul className="flex flex-col gap-1">
                  {rows.map((session) => {
                    const action = primaryAction(session, now);
                    const parked = parkedWords(session, now);
                    return (
                      <li
                        key={session.id}
                        data-triage-row={session.id}
                        data-testid="triage-row"
                        className="flex items-center gap-2 rounded-xl border border-hairline bg-surface/60 pr-2"
                      >
                        <div className="min-w-0 flex-1">
                          <SessionRow
                            session={session}
                            selected={false}
                            onSelect={onOpenSession}
                            density={rowDensity}
                            now={now}
                          />
                        </div>
                        {/* Compact rows drop the rail's parked chip; say it here. */}
                        {parked && rowDensity === "compact" && (
                          <span className="shrink-0 text-[11px] text-ink" data-testid="triage-parked">
                            {parked.short}
                          </span>
                        )}
                        {projectName(session) && (
                          <span className="hidden shrink-0 text-[11px] text-muted lg:inline">{projectName(session)}</span>
                        )}
                        <button
                          type="button"
                          disabled={busy === session.id}
                          onClick={() => void run(session)}
                          aria-keyshortcuts="A"
                          className="shrink-0 rounded-lg border border-hairline px-2.5 py-1 text-[11px] font-medium text-ink hover:border-accent/50 disabled:text-muted"
                          data-testid="triage-action"
                        >
                          {action.label}
                          <kbd className="ml-1.5 rounded border border-hairline px-1 font-mono text-[10px] text-muted">A</kbd>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
