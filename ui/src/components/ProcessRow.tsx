import { useState } from "react";
import { agoWords } from "../lib/time-words";
import { killDisplayName, probablyWords } from "../lib/process-words";

// The Centre reuses the shared primitive; re-export keeps one source.
export { Button } from "./primitives";
import { Button } from "./primitives";
import { memorySize } from "../lib/compact";

/**
 * AJ-E506/E507: the shared Cleanup row (the Cleanup Centre AND the
 * session's Processes tab render THIS component — no copies, no
 * divergent labels; divergence is how T3 survivors get mislabelled).
 *
 * The stop affordance (AJ-E507) exists ONLY on rows that verified at
 * last read AND are still live — Unverifiable/not-managed/exited rows,
 * and a live session's own main process (its session's Stop ends it),
 * render NONE (Req.2: not disabled-with-tooltip). It always opens the
 * verified sheet, which re-verifies at click time; ITS refusal is final.
 */

export interface VerificationBadgeInfo {
  verdict: string;
  detail: string;
}

export interface EvidenceChainInfo {
  root_pid: number | null;
  parent_pid: number | null;
  pgid: number | null;
  escaped_group: boolean;
  verification: VerificationBadgeInfo;
  first_seen: number;
  last_seen: number;
}

export interface CleanupRowInfo {
  /** AJ-E511: the machine that owns this row; equals the inventory's
   * `local_machine` on this node, anything else is display-only. */
  machine_id?: string;
  pid: number;
  pgid: number | null;
  role: string;
  state: string;
  state_label: string;
  /** AJ-E510: helpers of roots that died while Juggler was closed. */
  down_period?: boolean;
  /** What it probably is, in plain words ("dev server"); empty when
   * nothing is known. Always shown as a guess. */
  role_guess: string;
  exe_path: string;
  cmdline: string;
  age_ms: number | null;
  rss_bytes: number | null;
  cpu_pct: number | null;
  ports: string | null;
  hog: boolean;
  hog_rules: string[];
  evidence: EvidenceChainInfo;
}

function ageText(ms: number | null): string {
  if (ms === null) return "—";
  const secs = Math.max(0, Math.floor(ms / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3_600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86_400) return `${Math.floor(secs / 3_600)}h`;
  return `${Math.floor(secs / 86_400)}d`;
}

function rssText(bytes: number | null): string {
  return bytes === null ? "—" : memorySize(bytes);
}

/** State badge icon per class — icon+text, never colour alone (1.4.1). */
function stateIcon(state: string): string {
  switch (state) {
    case "orphaned":
      return "⚑";
    case "adopted":
      return "↻";
    case "unverifiable":
      return "⊘";
    case "exited":
      return "✓";
    default:
      return "●";
  }
}

/** Rows that may offer the verified kill sheet (AJ-E507 Req.2):
 * verified at last read AND still live — but never a live session's own
 * main process (the session's Stop ends it, with its undo window).
 * Everything else — Unverifiable, exited, starting — renders no
 * affordance at all. */
export function killableRow(row: CleanupRowInfo): boolean {
  if (row.role === "root" && row.state === "running") return false;
  return (
    row.evidence.verification.verdict === "verified" &&
    (row.state === "running" || row.state === "orphaned" || row.state === "adopted")
  );
}

/** The one column layout: the headings and every row share it. */
const COLUMNS =
  "grid grid-cols-[minmax(0,1fr)_minmax(0,17.5rem)_3.5rem_4.5rem_5rem_3rem_6.5rem] items-center gap-x-3";

/** The column headings over a list of rows (visual; each cell also names
 * itself to a screen reader). */
export function ProcessRowsHeader() {
  return (
    <div
      aria-hidden="true"
      className={`${COLUMNS} border-b border-hairline px-1 pb-1.5 text-label font-medium text-muted`}
      data-testid="process-rows-header"
    >
      <span className="pl-5">Process</span>
      <span>State</span>
      <span className="text-right">CPU</span>
      <span className="text-right">Memory</span>
      <span>Ports</span>
      <span className="text-right">Age</span>
      <span />
    </div>
  );
}

/** The identity check in plain words (the technical detail on hover). */
function identityWords(verification: VerificationBadgeInfo): string {
  switch (verification.verdict) {
    case "verified":
      return "Matches what AI Juggler recorded: the same program, started at the same time.";
    case "unverifiable":
      return "Doesn't match what AI Juggler recorded, so it may be another program now. AI Juggler won't touch it.";
    default:
      return verification.detail.charAt(0).toUpperCase() + verification.detail.slice(1) + ".";
  }
}

export function ProcessRow(props: {
  row: CleanupRowInfo;
  /** The machine's name, as its own quiet chip — passed only when more
   * than one Mac is in view (one Mac needs no label). */
  machineLabel?: string;
  /** Opens the AJ-E507 sheet; the row's element rides along for
   * focus-return (WCAG 2.4.3). Absent = read-only surface. */
  onKill?: (row: CleanupRowInfo, trigger: HTMLElement) => void;
  /** Clock for the evidence's "seen" words (tests pin it). */
  now?: number;
}) {
  const row = props.row;
  const [expanded, setExpanded] = useState(false);
  const evidence = row.evidence;
  const killable = killableRow(row) && props.onKill !== undefined;
  const name = killDisplayName(row.cmdline, row.exe_path);
  const guess = probablyWords(row.role_guess);
  const now = props.now ?? Date.now();
  const isoFirst = new Date(evidence.first_seen).toISOString().slice(0, 19).replace("T", " ");
  const isoLast = new Date(evidence.last_seen).toISOString().slice(0, 19).replace("T", " ");
  return (
    <li className="border-t border-hairline first:border-t-0" data-testid={`cleanup-row-${row.pid}`}>
      <div className={`${COLUMNS} px-1 py-2 text-meta`}>
        <button
          type="button"
          onClick={() => setExpanded((open) => !open)}
          aria-expanded={expanded}
          className="flex min-w-0 items-start gap-2 rounded-md py-0.5 text-left outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          title="Why AI Juggler counts it as your session's"
        >
          <span aria-hidden="true" className="mt-0.5 w-3 shrink-0 text-center text-label text-muted">
            {expanded ? "▾" : "▸"}
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="flex min-w-0 items-center gap-2">
              <span className="max-w-[12rem] shrink-0 truncate font-medium text-ink" data-testid={`cleanup-name-${row.pid}`}>
                {name}
              </span>
              {guess !== null && (
                <span className="min-w-0 truncate text-muted" data-testid={`cleanup-role-${row.pid}`}>
                  {guess}
                </span>
              )}
              {row.hog && (
                <span
                  className="inline-flex shrink-0 items-center gap-1 rounded-full border border-warning/50 px-1.5 py-[1px] text-label text-warning"
                  title={row.hog_rules.join("; ")}
                >
                  <span aria-hidden="true">▲</span>heavy
                </span>
              )}
              {props.machineLabel !== undefined && (
                <span
                  className="shrink-0 rounded-full border border-hairline px-1.5 py-[1px] text-label text-muted"
                  data-testid={`cleanup-machine-${row.pid}`}
                >
                  {props.machineLabel}
                </span>
              )}
            </span>
            <span className="truncate font-mono text-label text-muted" title={row.cmdline}>
              {row.cmdline}
            </span>
          </span>
        </button>
        <span className="min-w-0">
          <span className="sr-only">State: </span>
          <span
            className={`inline-flex max-w-full items-center gap-1 rounded-full border px-1.5 py-[1px] text-label ${
              row.state === "orphaned"
                ? "border-accent/50 text-accent"
                : row.state === "unverifiable"
                  ? "border-danger/50 text-danger"
                  : "border-hairline text-muted"
            }`}
            title={row.state_label}
            data-testid={`cleanup-state-${row.pid}`}
          >
            <span aria-hidden="true">{stateIcon(row.state)}</span>
            <span className="truncate">{row.state_label}</span>
          </span>
        </span>
        <span className="text-right tabular-nums text-ink">
          <span className="sr-only">CPU </span>
          {row.cpu_pct !== null ? `${row.cpu_pct.toFixed(0)}%` : "—"}
        </span>
        <span className="text-right tabular-nums text-ink">
          <span className="sr-only">Memory </span>
          {rssText(row.rss_bytes)}
        </span>
        <span className="truncate tabular-nums">
          <span className="sr-only">Ports </span>
          {row.ports !== null ? (
            <span className="text-accent-hi">:{row.ports.split(",").join(" :")}</span>
          ) : (
            <span className="text-muted">—</span>
          )}
        </span>
        <span className="text-right tabular-nums text-muted">
          <span className="sr-only">Age </span>
          {ageText(row.age_ms)}
        </span>
        <span className="flex justify-end">
          {killable && (
            <Button
              size="sm"
              variant="secondary"
              className="shrink-0"
              onClick={(event: React.MouseEvent<HTMLButtonElement>) =>
                props.onKill?.(row, event.currentTarget)
              }
              data-testid={`cleanup-kill-${row.pid}`}
            >
              Terminate…
            </Button>
          )}
        </span>
      </div>
      {expanded && (
        <div
          className="mb-2 ml-6 flex flex-col gap-1.5 rounded-lg border border-hairline bg-surface/40 p-3 text-meta text-muted"
          data-testid={`cleanup-evidence-${row.pid}`}
        >
          <p className="font-medium text-ink">Why AI Juggler counts it as your session's</p>
          <dl className="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-1">
            <dt>Started by</dt>
            <dd
              className="text-ink"
              title={`Root PID ${evidence.root_pid ?? "—"}${evidence.parent_pid !== null ? ` · first-seen parent PID ${evidence.parent_pid}` : ""}`}
            >
              {evidence.root_pid !== null && evidence.parent_pid === evidence.root_pid
                ? "The session's main process"
                : "A process the session started"}
            </dd>
            <dt>Grouping</dt>
            <dd className="text-ink" title={`Process group (PGID) ${evidence.pgid ?? "—"}`}>
              {evidence.escaped_group
                ? "Left the session's process group, so stopping the group would miss it."
                : evidence.pgid !== null && evidence.pgid === evidence.root_pid
                  ? "Stays with the session's other processes."
                  : "In a process group of its own."}
            </dd>
            <dt>Identity check</dt>
            <dd className="text-ink" title={evidence.verification.detail} data-testid={`cleanup-verification-${row.pid}`}>
              {identityWords(evidence.verification)}
            </dd>
            <dt>What it is</dt>
            <dd className="text-ink">
              {guess !== null ? guess.charAt(0).toUpperCase() + guess.slice(1) : "Not known"}
              <span className="text-muted"> · process {row.pid}</span>
            </dd>
            <dt>Seen</dt>
            <dd className="text-ink" title={`First ${isoFirst} · last ${isoLast} (UTC)`}>
              First {agoWords(evidence.first_seen, now)} · last {agoWords(evidence.last_seen, now)}
            </dd>
          </dl>
          <p>Stopping a verified process always asks you to confirm first — never a one-click kill.</p>
        </div>
      )}
    </li>
  );
}

/** The empty success state — copy EXACT (Req.5), as a designed card
 * (the review of 0.1.4: one grey line on an empty page). */
export function CleanupEmpty(props: { cleanInstances: number; onHistory?: () => void }) {
  return (
    <div className="flex flex-col items-center gap-3 rounded-card border border-hairline bg-surface px-6 py-12 text-center shadow-card" data-testid="cleanup-empty-card">
      <span aria-hidden="true" className="flex size-12 items-center justify-center rounded-full bg-success-soft text-success">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M5 12.5l4.5 4.5L19 7.5" />
        </svg>
      </span>
      <p className="text-section font-semibold text-ink" data-testid="cleanup-empty">
        {props.cleanInstances === 0
          ? "Nothing left behind."
          : `Nothing left behind — ${props.cleanInstances} ${props.cleanInstances === 1 ? "session" : "sessions"} running clean.`}
      </p>
      <p className="max-w-[52ch] text-body text-muted">
        When a session closes and something it started keeps running, such as a dev server or a file watcher, it shows here for you to stop.
      </p>
      {props.onHistory && (
        <button type="button" onClick={props.onHistory} className="text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60">
          See what was cleaned up in the last 7 days
        </button>
      )}
    </div>
  );
}
