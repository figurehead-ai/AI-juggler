import { toolMarkIcon } from "./ToolMark";
import { toolSpec } from "../lib/tools";
import type { ReactNode } from "react";
import type { AccountInfo, InstanceInfo } from "../lib/bridge";
import { renewalLabel, renewalRowVisible } from "../lib/renewal";
import {
  IconFork,
  IconGitBranch,
  IconStateFailed,
  IconStateFinished,
  IconStateHandover,
  IconStateQuota,
  IconStateRunning,
  IconStateStarting,
  IconStateStopped,
  IconStateStuck,
  IconStateUnknown,
  IconStateWaiting,
  IconSubtasks,
} from "./icons";

/**
 * I-01..I-11 display mapping for the Instance indicator row and card
 * (AJ-E402). One source for both variants — no number may exist on a
 * card the rail cannot explain. Absence renders exactly: `Quota: no
 * data`, `default model`, `Unknown`, hidden children at 0, omitted
 * branch for non-git cwds, `—` for unsampled resources. Never a
 * fabricated `0`, `$0.00`, or guessed model.
 */

// --- I-01 Provider marks ---------------------------------------------------

/** AJ-E1526: a tool's mark and short name, from its spec in the catalog
 * (the letter mark until its owner allows the real one). An unknown tool
 * gets the neutral spec: its id and its letters, never blank. */
export function providerMark(provider: string) {
  return { abbrev: toolSpec(provider).short_name, Icon: toolMarkIcon(provider) };
}

// --- I-04 state chips (icon + text + colour, never colour-only) ------------

export type StateChipTone = "neutral" | "success" | "warning" | "danger" | "live" | "accent";

export function stateChip(status: string): {
  label: string;
  Icon: typeof IconStateRunning;
  tone: StateChipTone;
} {
  switch (status) {
    case "starting":
      return { label: "Starting", Icon: IconStateStarting, tone: "accent" };
    case "running":
      return { label: "Running", Icon: IconStateRunning, tone: "live" };
    case "waiting_user":
      return { label: "Needs you", Icon: IconStateWaiting, tone: "warning" };
    case "stuck":
      return { label: "Stuck?", Icon: IconStateStuck, tone: "warning" };
    case "quota_blocked":
      return { label: "Quota blocked", Icon: IconStateQuota, tone: "danger" };
    case "finished":
      return { label: "Finished", Icon: IconStateFinished, tone: "success" };
    case "stopped":
      return { label: "Stopped", Icon: IconStateStopped, tone: "neutral" };
    case "failed":
      return { label: "Failed", Icon: IconStateFailed, tone: "danger" };
    case "handed_over":
      return { label: "Handed over", Icon: IconStateHandover, tone: "accent" };
    // Unknown wire value → `Unknown` chip, never a neutral dot.
    default:
      return { label: "Unknown", Icon: IconStateUnknown, tone: "neutral" };
  }
}

// --- I-02 account identity ---------------------------------------------------

/** Initials for the account disc: first letters of the first and last
 * words, where "-", "_" and "." part words too — labels like
 * "claude-work" and "claude-personal" differ in their last word (AJ-E1529). */
// --- I-06 tokens (no pricing source exists: tokens only, never `$0.00`) ----

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

/** `18.2k tokens` from input+output; `null` when usage was never observed. */
export function tokenSummary(
  tokens: InstanceInfo["tokens"],
): string | null {
  if (!tokens) return null;
  return `${formatTokens(tokens.input + tokens.output)} tokens`;
}

/**
 * AJ-E405: the structured reason as tooltip/announcement text —
 * `stuck_no_output · no output 6m · cpu 0.3% · transcript stale 6m`.
 * The UI renders the measured inputs verbatim; it never re-derives
 * them from the status.
 */
export function attentionTooltip(session: InstanceInfo): string | undefined {
  const reason = session.attention;
  if (!reason) return undefined;
  return [reason.rule, ...reason.inputs].join(" · ");
}

// --- I-10 / I-11 time formatting ---------------------------------------------

/** Compact duration: `10s`, `4m`, `2h05m`, `1d3h`. */
export function formatDuration(ms: number): string {
  if (ms < 0) return "0s";
  const seconds = Math.round(ms / 1_000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h${String(minutes % 60).padStart(2, "0")}m`;
  return `${Math.floor(hours / 24)}d${hours % 24}h`;
}

/** `17:00`-style wall clock for reset countdowns (local time). */
export function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

/**
 * I-11 age + last-activity line: `42m · active 10s ago`; just-started →
 * `starting…`; never-active → `no output yet`.
 */
export function ageSummary(
  startedAtMs: number,
  lastActivityMs: number | null,
  nowMs: number = Date.now(),
): string {
  if (nowMs - startedAtMs < 5_000) return "starting…";
  const age = formatDuration(nowMs - startedAtMs);
  if (lastActivityMs === null) return `${age} · no output yet`;
  return `${age} · active ${formatDuration(Math.max(0, nowMs - lastActivityMs))} ago`;
}

/** I-10 waiting duration from last activity (`wait 4m`). */
export function waitingFor(
  lastActivityMs: number | null,
  nowMs: number = Date.now(),
): string | null {
  if (lastActivityMs === null) return null;
  return `wait ${formatDuration(Math.max(0, nowMs - lastActivityMs))}`;
}

/**
 * I-10 needs-attention badge. For the attention states the badge IS the
 * line-1 state indicator (icon + text + colour — the chip contract still
 * holds); `null` renders the plain I-04 chip instead. Exact copy:
 * `Needs you`, `Stuck?`, `Quota blocked`.
 */
export function attentionBadge(
  session: InstanceInfo,
): { text: string; tone: StateChipTone } | null {
  switch (session.needs_attention) {
    case "waiting_user":
      return { text: "Needs you", tone: "warning" };
    case "stuck":
      return { text: "Stuck?", tone: "warning" };
    case "quota_blocked":
      return { text: "Quota blocked", tone: "danger" };
    case "failed":
      return { text: "Failed", tone: "danger" };
    default:
      return null;
  }
}

/** I-10 badge suffix: waiting duration or reset countdown, when known. */
export function attentionSuffix(
  session: InstanceInfo,
  nowMs: number = Date.now(),
): string | null {
  if (session.needs_attention === "waiting_user") {
    return waitingFor(session.last_activity_ms, nowMs);
  }
  if (session.needs_attention === "quota_blocked" && session.quota?.resets_at_ms != null) {
    return `resets ${formatClock(session.quota.resets_at_ms)}`;
  }
  return null;
}

// --- I-07 sparkline history (honest: measured samples only) ------------------

const HISTORY_MAX = 60; // 5 min at the 5 s resource tick

type ResourceWindow = { cpu: number[]; ram: number[] };
const resourceHistory = new Map<string, ResourceWindow>();
const lastAppendAt = new Map<string, number>();

/**
 * Remembers one measured resource sample (CPU% + RAM bytes) and returns
 * the trailing windows (oldest → newest). Only real samples enter the
 * history; absence renders the placeholder dash, never a flat zero.
 * Rapid identical re-appends (React StrictMode double-render) collapse
 * into one sample.
 */
export function rememberResourceSample(
  id: string,
  cpu: number | null,
  ram: number | null,
  nowMs: number = Date.now(),
): ResourceWindow {
  if (cpu !== null && ram !== null) {
    const window = resourceHistory.get(id) ?? { cpu: [], ram: [] };
    const lastAt = lastAppendAt.get(id) ?? 0;
    const duplicate =
      window.cpu.length > 0 &&
      window.cpu[window.cpu.length - 1] === cpu &&
      window.ram[window.ram.length - 1] === ram &&
      nowMs - lastAt < 2_500;
    if (!duplicate) {
      window.cpu.push(cpu);
      window.ram.push(ram);
      if (window.cpu.length > HISTORY_MAX) {
        window.cpu.shift();
        window.ram.shift();
      }
      resourceHistory.set(id, window);
      lastAppendAt.set(id, nowMs);
    }
  }
  return resourceHistory.get(id) ?? { cpu: [], ram: [] };
}

/** `1.1 GB` / `890 MB` byte formatting. */
export function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${Math.round(bytes / 1024 ** 2)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

// --- small shared renderers ---------------------------------------------------

/** I-02 initials disc with the per-Account hue. */
/** I-08 children and running-subagent badges — hidden at 0/`null` (no
 * `⑂ 0` noise). AJ-E1520: subagents still running, from the transcript. */
/** `1 subagent running`, `3 subagents running`. */
export function subagentsRunning(count: number): string {
  return `${count} ${count === 1 ? "subagent" : "subagents"} running`;
}

export function ChildBadges({ session }: { session: InstanceInfo }): ReactNode {
  const children = session.child_count ?? 0;
  const subtasks = session.subtask_count ?? 0;
  if (children === 0 && subtasks === 0) return null;
  return (
    <span className="inline-flex shrink-0 items-center gap-1.5 text-[10px] text-muted">
      {children > 0 && (
        <span className="inline-flex items-center gap-0.5" title={`${children} child processes`}>
          <IconFork width={10} height={10} /> {children}
        </span>
      )}
      {subtasks > 0 && (
        <span className="inline-flex items-center gap-0.5" title={subagentsRunning(subtasks)}>
          <IconSubtasks width={10} height={10} /> {subtasks}
        </span>
      )}
    </span>
  );
}

/** AJ-E1002: branch, uncommitted files, ahead/behind and worktree in
 * words (the badge's title and the row's accessible summary). */
export function gitWords(session: InstanceInfo): string {
  const parts = [`branch ${session.git_branch ?? "detached"}`];
  const dirty = session.git_dirty_count;
  if (dirty) parts.push(`${dirty} uncommitted ${dirty === 1 ? "file" : "files"}`);
  if (session.git_ahead) parts.push(`${session.git_ahead} ahead`);
  if (session.git_behind) parts.push(`${session.git_behind} behind`);
  if (session.git_worktree) parts.push("own worktree");
  return parts.join(" · ");
}

/** I-09 branch + dirty — omitted entirely for non-git cwds. AJ-E1002:
 * the dirty count is a number (never a colour alone) and ahead/behind
 * and the worktree are named in the title. */
export function GitBadge({ session }: { session: InstanceInfo }): ReactNode {
  if (!session.git_branch) return null;
  const dirty = session.git_dirty_count;
  return (
    <span
      className="inline-flex min-w-0 items-center gap-0.5 text-[10px] text-muted"
      title={gitWords(session)}
      data-testid="git-badge"
    >
      <IconGitBranch width={10} height={10} />
      {session.git_worktree ? <span aria-hidden="true">⧉</span> : null}
      <span className="truncate">{session.git_branch}</span>
      {dirty ? <span className="shrink-0 text-warning">· {dirty}●</span> : null}
      {session.git_ahead ? <span className="shrink-0">↑{session.git_ahead}</span> : null}
      {session.git_behind ? <span className="shrink-0">↓{session.git_behind}</span> : null}
    </span>
  );
}

// --- AJ-E403: the stale-data rule ---------------------------------------------

/**
 * `stale - Nm` caption chip (Req.5). Contrast-safe by construction: the
 * value keeps its full text colour (never dimmed below 4.5:1) and the
 * caption carries the meaning — the manual-contrast fallback the task
 * prescribes when dimming would break contrast.
 */
export function StaleChip({ caption }: { caption: string | null }) {
  if (caption === null) return null;
  return (
    <span className="inline-flex shrink-0 items-center rounded border border-warning/40 bg-warning-soft px-1 text-[9px] font-medium whitespace-nowrap text-warning" title={`Value is ${caption} — older than 2× its refresh interval`}>
      {caption}
    </span>
  );
}

// --- AJ-E403: the shared legend (identical in every surface) -------------------

/** D-25: the rail's own words for each state — one list, shared by the
 * rail rows and the legend so the two can never drift apart. */
export const STATE_WORDS: Record<string, string> = {
  starting: "Starting",
  running: "Running",
  waiting_user: "Waiting for you",
  stuck: "Looks stuck",
  quota_blocked: "Hit its limit",
  finished: "Done",
  stopped: "Stopped",
  failed: "Failed",
  handed_over: "Handed over",
};

const LEGEND_STATES: { status: string; Icon: typeof IconStateRunning; tone: StateChipTone; meaning: string }[] = [
  { status: "starting", Icon: IconStateStarting, tone: "accent", meaning: "opening up" },
  { status: "running", Icon: IconStateRunning, tone: "live", meaning: "working" },
  { status: "waiting_user", Icon: IconStateWaiting, tone: "warning", meaning: "asked you something" },
  { status: "stuck", Icon: IconStateStuck, tone: "warning", meaning: "no output for a while" },
  { status: "quota_blocked", Icon: IconStateQuota, tone: "danger", meaning: "the account ran out" },
  { status: "finished", Icon: IconStateFinished, tone: "success", meaning: "ended normally" },
  { status: "stopped", Icon: IconStateStopped, tone: "neutral", meaning: "you stopped it" },
  { status: "failed", Icon: IconStateFailed, tone: "danger", meaning: "ended with an error" },
  { status: "handed_over", Icon: IconStateHandover, tone: "accent", meaning: "moved to another account" },
];

/** The notes a rail row adds only when they matter (D-25). */
const LEGEND_NOTES: { sample: string; tone: string; meaning: string }[] = [
  { sample: "18% left", tone: "text-warning", meaning: "its account is running low" },
  { sample: "Parked 2h 10m", tone: "text-ink", meaning: "waiting for the limit to reset" },
  { sample: "3 changed", tone: "text-muted", meaning: "files not committed yet" },
  { sample: "not signed in", tone: "text-muted", meaning: "the account needs its sign-in" },
  { sample: "APP", tone: "text-muted", meaning: "runs in the Claude desktop app" },
];

/**
 * The one legend (Req.6): what the rail's icons and words mean, and the
 * notes a row adds when something needs a glance. Opened from the `?`
 * in the rail's footer.
 */
export function IndicatorLegend({ compact = false }: { compact?: boolean }) {
  return (
    <div className={compact ? "flex flex-col gap-1.5 text-[10px]" : "flex flex-col gap-2 text-[11px]"}>
      <span className="text-[11px] font-semibold text-ink">What the session list shows</span>
      <dl className="grid grid-cols-[auto_1fr] items-center gap-x-2 gap-y-1" aria-label="States">
        {LEGEND_STATES.map((state) => (
          <div key={state.status} className="contents">
            <dt className="flex items-center gap-2 whitespace-nowrap font-medium text-ink">
              <state.Icon width={12} height={12} aria-hidden="true" className={legendTone(state.tone)} />
              {STATE_WORDS[state.status]}
            </dt>
            <dd className="text-muted">{state.meaning}</dd>
          </div>
        ))}
      </dl>
      <dl className="grid grid-cols-[auto_1fr] items-center gap-x-2 gap-y-1 border-t border-hairline pt-1.5" aria-label="Notes on a row">
        {LEGEND_NOTES.map((note) => (
          <div key={note.sample} className="contents">
            <dt className={"whitespace-nowrap font-medium " + note.tone}>{note.sample}</dt>
            <dd className="text-muted">{note.meaning}</dd>
          </div>
        ))}
        <div className="contents">
          <dt>
            <span className="inline-flex items-center whitespace-nowrap rounded border border-warning/40 bg-warning-soft px-1 text-[9px] font-medium text-warning">stale - 3m</span>
          </dt>
          <dd className="text-muted">a reading stopped updating</dd>
        </div>
      </dl>
      <span className="border-t border-hairline pt-1.5 text-muted">Each row's dot is its account's colour.</span>
    </div>
  );
}

function legendTone(tone: StateChipTone): string {
  switch (tone) {
    case "live": return "text-live";
    case "success": return "text-success";
    case "warning": return "text-warning";
    case "danger": return "text-danger";
    case "accent": return "text-accent-hi";
    default: return "text-muted";
  }
}

/**
 * Active statuses (mirrors the backend read model): non-terminal, so the
 * minimal rail keeps waiting/stuck/quota-blocked Instances visible — the
 * collapsed rail must still scream "needs you" (AJ-E403 user story).
 */
export function isActiveStatus(status: string): boolean {
  return status === "starting" || status === "running" || status === "waiting_user" || status === "stuck" || status === "quota_blocked";
}

/** "18% left" once a live session's account runs low (25 % or less);
 * a finished session says nothing about its account's quota (AJ-E1535). */
export function lowLeftWords(session: InstanceInfo): string | null {
  if (!session.quota || !isActiveStatus(session.status)) return null;
  const left = Math.max(0, Math.round(100 - session.quota.used_percentage));
  return left <= 25 ? `${left}% left` : null;
}

// --- WCAG 4.1.3: attention changes announced politely (AJ-E405) ---------

import { useEffect, useRef, useState } from "react";

/**
 * Announces attention transitions on a polite live region — state
 * changes are never conveyed by colour alone. The message is the badge
 * text plus the rule (`Needs you — waiting_prompt`).
 */
import { attentionWords } from "../lib/attention";
export { attentionWords };

export function AttentionAnnouncer({ sessions }: { sessions: InstanceInfo[] }) {
  const [message, setMessage] = useState("");
  const previous = useRef<Set<string>>(new Set());

  useEffect(() => {
    const current = new Set(sessions.filter((row) => row.attention !== null).map((row) => row.id));
    const labelOf = (id: string) =>
      sessions.find((row) => row.id === id)?.account_label ?? "A session";
    const fresh = [...current].filter((id) => !previous.current.has(id));
    const recovered = [...previous.current].filter((id) => !current.has(id));
    if (fresh.length > 0) {
      const parts = fresh.map((id) => {
        const row = sessions.find((entry) => entry.id === id);
        return `${labelOf(id)} needs you — ${attentionWords(row?.attention?.rule ?? row?.needs_attention)}`;
      });
      setMessage(parts.join(". "));
    } else if (recovered.length > 0) {
      setMessage(recovered.map((id) => `${labelOf(id)} recovered`).join(". "));
    }
    previous.current = current;
  }, [sessions]);

  return (
    <div aria-live="polite" className="sr-only" data-testid="attention-announcer">
      {message}
    </div>
  );
}


/**
 * AJ-E411 (D-12): polite once-per-day announcement when an Account's
 * renewal date crosses into <= 7 days (warning) or is past (danger) —
 * never on every tick (UI/UX reasoning). The last-announced local date
 * is remembered per account in localStorage.
 */
export function RenewalAnnouncer({ accounts, now = Date.now() }: { accounts: AccountInfo[]; now?: number }) {
  const [message, setMessage] = useState("");

  useEffect(() => {
    const local = new Date(now);
    const pad = (value: number) => String(value).padStart(2, "0");
    const today = `${local.getFullYear()}-${pad(local.getMonth() + 1)}-${pad(local.getDate())}`;
    const parts: string[] = [];
    for (const account of accounts) {
      const label = renewalLabel(account.renewal_date, now);
      if (!renewalRowVisible(label)) continue;
      const key = `aij.renewal.announced.${account.id}`;
      if (localStorage.getItem(key) === today) continue;
      localStorage.setItem(key, today);
      parts.push(`${account.label} renewal: ${label?.text ?? "Renewal date passed"}`);
    }
    setMessage(parts.length > 0 ? parts.join(". ") : "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accounts, now]);

  return (
    <div aria-live="polite" className="sr-only" data-testid="renewal-announcer">
      {message}
    </div>
  );
}
