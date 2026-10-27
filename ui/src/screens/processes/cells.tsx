/**
 * AJ-E2404 — one row's cells in the Processes tree table: the process
 * (indented, with its role in words and what started it), its state (an
 * icon and a word), CPU with its line, memory, ports, age and whose. App
 * and session rows sum their processes ("6 processes · 1.2 GB · 4 %").
 */
import type { JSX } from "react";
import { AppIcon, OWN_APP_ID } from "../../components/AppIcon";
import { IconComputer } from "../../components/icons-actions";
import { memorySize } from "../../lib/compact";
import { ageWords, cpuWords, portWords, processState, roleWords, sessionState, ZOMBIE_WORDS, type ProcessTone } from "../../lib/process-states";
import type { ProcessGraphRowInfo } from "../../lib/ipc/processes";

/** The table's columns: Process, State, CPU, Memory, Ports, Age, Owner,
 * and the row's actions. The table is a container: Owner shows once the
 * table itself is wide enough (whatever the window and the rail). */
export const COLUMNS =
  "grid-cols-[minmax(13.5rem,1fr)_10.5rem_7rem_5.5rem_5rem_4rem_9rem] @min-[72rem]:grid-cols-[minmax(18rem,1fr)_11.5rem_7.5rem_6rem_6rem_4.5rem_12rem_9rem]";

/** The Owner column: only in a wide table. */
export const OWNER_CELL = "hidden @min-[72rem]:block";

const TONE: Record<ProcessTone, string> = {
  neutral: "text-muted",
  live: "text-live",
  warning: "text-warning",
  danger: "text-danger",
  accent: "text-accent-hi",
};

/** A state as an icon and a word, with what it means on hover. */
export function StateChip({ word, session = false }: { word: string; session?: boolean }): JSX.Element {
  const state = session ? sessionState(word) : processState(word);
  const Icon = state.Icon;
  return (
    <span className={`inline-flex min-w-0 items-center gap-1.5 text-meta font-medium ${TONE[state.tone]}`} title={state.hint} data-testid="process-state" data-state={word}>
      <Icon width={14} height={14} aria-hidden="true" className="shrink-0" />
      <span className="truncate">{state.label}</span>
    </span>
  );
}

/** The last minute of CPU as a small line (the number beside it says it). */
export function CpuLine({ samples }: { samples: readonly number[] }): JSX.Element | null {
  if (samples.length < 2) return null;
  const width = 44;
  const height = 16;
  const top = Math.max(...samples, 5);
  const step = width / (samples.length - 1);
  const points = samples.map((value, at) => `${(at * step).toFixed(1)},${(height - 1 - (value / top) * (height - 3)).toFixed(1)}`).join(" ");
  return (
    <svg width={width} height={height} aria-hidden="true" focusable="false" className="shrink-0 text-live">
      <polyline points={points} fill="none" stroke="currentColor" strokeWidth={1.4} strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

/** "6 processes · 1.2 GB · 4 %". */
export function rollupWords(row: ProcessGraphRowInfo): string {
  const rollup = row.rollup;
  if (!rollup) return "";
  const parts = [`${rollup.processes} ${rollup.processes === 1 ? "process" : "processes"}`];
  if (rollup.rss_bytes > 0) parts.push(memorySize(rollup.rss_bytes));
  parts.push(cpuWords(rollup.cpu_pct));
  return parts.join(" · ");
}

function Chevron({ row, open, onToggle }: { row: ProcessGraphRowInfo; open: boolean; onToggle: () => void }): JSX.Element {
  if (!row.has_children) return <span className="inline-block w-5 shrink-0" aria-hidden="true" />;
  return (
    <button
      type="button"
      tabIndex={-1}
      onClick={(event) => {
        event.stopPropagation();
        onToggle();
      }}
      aria-label={open ? "Fold" : "Unfold"}
      className="inline-flex size-5 shrink-0 items-center justify-center rounded text-muted hover:bg-surface-3 hover:text-ink"
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" className={`transition-transform ${open ? "rotate-90" : ""}`}>
        <path d="M3 1.5 6.5 5 3 8.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </button>
  );
}

/** The first cell: indented, a chevron for what it holds, then the name
 * and its second line. */
export function NameCell({ row, open, onToggle }: { row: ProcessGraphRowInfo; open: boolean; onToggle: () => void }): JSX.Element {
  const indent = { paddingLeft: `${Math.min(row.depth, 8) * 16}px` };
  if (row.kind === "program" && row.app) {
    const count = row.rollup?.processes ?? 1;
    return (
      <div role="gridcell" className="flex min-w-0 items-center gap-2" style={indent}>
        <Chevron row={row} open={open} onToggle={onToggle} />
        <span className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-body text-ink">{row.app.name}</span>
          <span className="truncate text-label text-muted">{count === 1 ? "1 process" : `${count} processes`}</span>
        </span>
      </div>
    );
  }
  if (row.app?.kind === "other") {
    const rollup = row.rollup;
    return (
      <div role="gridcell" aria-colspan={2} className="col-span-2 flex min-w-0 items-center gap-2" style={indent}>
        <Chevron row={row} open={open} onToggle={onToggle} />
        <IconComputer width={20} height={20} aria-hidden="true" className="shrink-0 text-muted" />
        <span className="shrink-0 text-body font-semibold text-ink">{row.app.name}</span>
        <span className="min-w-0 truncate text-meta text-muted">
          {rollup ? `${rollup.processes} processes · ${memorySize(rollup.rss_bytes)} · names and memory only` : ""}
        </span>
      </div>
    );
  }
  if (row.app) {
    return (
      <div role="gridcell" aria-colspan={2} className="col-span-2 flex min-w-0 items-center gap-2" style={indent}>
        <Chevron row={row} open={open} onToggle={onToggle} />
        <AppIcon appId={row.app.ours ? OWN_APP_ID : row.app.app_id} appPid={row.app.pid} name={row.app.name} size={20} />
        <span className="shrink-0 text-body font-semibold text-ink">{row.app.name}</span>
        <span className="min-w-0 truncate text-meta text-muted">
          {row.app.sessions} {row.app.sessions === 1 ? "session" : "sessions"} · {rollupWords(row)}
        </span>
      </div>
    );
  }
  if (row.session) {
    const session = row.session;
    const second = [session.folder_name, rollupWords(row)].filter(Boolean).join(" · ");
    return (
      <div role="gridcell" className="flex min-w-0 items-center gap-2" style={indent}>
        <Chevron row={row} open={open} onToggle={onToggle} />
        <span className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-body font-medium text-ink">{session.title || "Untitled session"}</span>
          <span className="truncate text-label text-muted">{second}</span>
        </span>
      </div>
    );
  }
  const proc = row.process!;
  const zombie = row.state === "zombie";
  const second = zombie
    ? `Finished; only ${proc.waits_for_name ?? "its parent"}${proc.waits_for_pid ? ` (${proc.waits_for_pid})` : ""} can clear it`
    : [roleWords(proc.role, proc.role_words, proc.role_sure), proc.label, proc.failed].filter(Boolean).join(" · ");
  return (
    <div role="gridcell" className="flex min-w-0 items-center gap-2" style={indent}>
      <Chevron row={row} open={open} onToggle={onToggle} />
      <span className="flex min-w-0 flex-col leading-tight">
        <span className="truncate text-body text-ink" title={`PID ${proc.pid}`}>
          {proc.name}
        </span>
        <span className="truncate text-label text-muted" title={zombie ? ZOMBIE_WORDS : second}>
          {second}
        </span>
      </span>
    </div>
  );
}

/** The number cells after the state: CPU, memory, ports, age, whose. */
export function NumberCells({ row }: { row: ProcessGraphRowInfo }): JSX.Element {
  const proc = row.process;
  const age = proc?.age_ms ?? 0;
  const ended = row.state === "ended";
  return (
    <>
      <div role="gridcell" className="flex items-center gap-2 text-body tabular-nums text-ink">
        <span className="w-12 text-right">{row.kind === "app" || row.kind === "program" || ended ? "" : cpuWords(row.cpu_pct)}</span>
        {proc && !ended && <CpuLine samples={proc.spark} />}
      </div>
      {ended ? (
        <div role="gridcell" className="text-right text-body tabular-nums text-muted" title={proc && proc.peak_rss > 0 ? "The most it used" : undefined}>
          {proc && proc.peak_rss > 0 ? memorySize(proc.peak_rss) : ""}
        </div>
      ) : (
        <div role="gridcell" className="text-right text-body tabular-nums text-ink" title={proc && proc.peak_rss > row.rss_bytes ? `At most ${memorySize(proc.peak_rss)}` : undefined}>
          {row.kind === "app" || (row.rss_bytes === 0 && row.state === "zombie") ? "" : row.rss_bytes > 0 ? memorySize(row.rss_bytes) : "—"}
        </div>
      )}
      <div role="gridcell" className="truncate text-body tabular-nums text-ink">
        {row.kind === "process" ? portWords(row.ports) : ""}
      </div>
      <div role="gridcell" className="text-right text-body tabular-nums text-muted" title={ended && age > 0 ? `Ran for ${ageWords(age)}` : undefined}>
        {proc && age > 0 ? ageWords(age) : ""}
      </div>
      <div role="gridcell" className={`${OWNER_CELL} truncate text-meta text-muted`} title={row.owner_words}>
        {row.kind === "app" ? "" : row.owner_words}
      </div>
    </>
  );
}
