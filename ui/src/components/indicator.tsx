import type { ReactNode } from "react";
import type { AccountInfo, InstanceInfo } from "../lib/bridge";
import { renewalLabel, renewalRowVisible } from "../lib/renewal";
import {
  IconFork,
  IconGitBranch,
  IconProviderClaude,
  IconProviderCodex,
  IconProviderGeneric,
  IconProviderZCode,
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

export const PROVIDER_MARKS: Record<
  string,
  { abbrev: string; Icon: typeof IconProviderClaude }
> = {
  "claude-code": { abbrev: "Claude", Icon: IconProviderClaude },
  codex: { abbrev: "Codex", Icon: IconProviderCodex },
  zcode: { abbrev: "ZCode", Icon: IconProviderZCode },
};

/** Unknown Provider degrades to the generic `Tool` mark — never blank. */
export function providerMark(provider: string) {
  return PROVIDER_MARKS[provider] ?? { abbrev: "Tool", Icon: IconProviderGeneric };
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

/** Initials for the account disc: first letters of the first two words. */
export function initialsOf(label: string): string {
  const words = label.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length === 1) return words[0]!.slice(0, 2).toUpperCase();
  return (words[0]![0]! + words[1]![0]!).toUpperCase();
}

/** Stable per-Account hue (0–359) from a hash of the account id. */
export function hueOf(accountId: string | null): number {
  const key = accountId ?? "unknown";
  let hash = 0;
  for (let i = 0; i < key.length; i += 1) {
    hash = (hash * 31 + key.charCodeAt(i)) % 100_000;
  }
  return hash % 360;
}

// --- I-06 tokens (no pricing source exists: tokens only, never `$0.00`) ----

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

/** `18.2k tok` from input+output; `null` when usage was never observed. */
export function tokenSummary(
  tokens: InstanceInfo["tokens"],
): string | null {
  if (!tokens) return null;
  return `${formatTokens(tokens.input + tokens.output)} tok`;
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

// --- Screen-reader summary ----------------------------------------------------

/** Full field text for `aria-label`: every number reachable as text. */
export function ariaSummary(session: InstanceInfo, nowMs: number = Date.now()): string {
  const mark = providerMark(session.provider);
  const chip = stateChip(session.status);
  const badge = attentionBadge(session);
  const parts: string[] = [
    `${mark.abbrev} provider`,
    session.account_label,
    session.model ?? "default model",
    badge ? badge.text : chip.label,
  ];
  if (session.quota) {
    parts.push(
      `quota ${Math.round(100 - session.quota.used_percentage)}% left, ${session.quota.confidence}`,
    );
  } else {
    parts.push("Quota: no data");
  }
  const tokens = tokenSummary(session.tokens);
  if (tokens) parts.push(tokens);
  if (session.cpu_pct !== null && session.rss_bytes !== null) {
    parts.push(`cpu ${Math.round(session.cpu_pct)}%, ram ${formatBytes(session.rss_bytes)}`);
  }
  if (session.child_count) parts.push(`${session.child_count} children`);
  if (session.subtask_count) parts.push(`${session.subtask_count} subtasks`);
  if (session.git_branch) parts.push(`branch ${session.git_branch}`);
  parts.push(ageSummary(session.started_at_ms, session.last_activity_ms, nowMs));
  const suffix = attentionSuffix(session, nowMs);
  if (badge && suffix) parts.push(suffix);
  const reason = attentionTooltip(session);
  if (reason) parts.push(reason);
  return parts.join(", ");
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
export function AccountDisc({
  accountId,
  label,
  size = 18,
}: {
  accountId: string | null;
  label: string;
  size?: number;
}) {
  return (
    <span
      className="acct-disc inline-flex shrink-0 items-center justify-center rounded-full text-[9px] font-semibold"
      style={
        {
          "--acct-hue": hueOf(accountId),
          width: size,
          height: size,
        } as React.CSSProperties
      }
      title={label}
      aria-hidden="true"
    >
      {initialsOf(label)}
    </span>
  );
}

/** I-08 children/subtask badges — hidden at 0/`null` (no `⑂ 0` noise). */
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
        <span className="inline-flex items-center gap-0.5" title={`${subtasks} subtasks`}>
          <IconSubtasks width={10} height={10} /> {subtasks}
        </span>
      )}
    </span>
  );
}

/** I-09 branch + dirty — omitted entirely for non-git cwds. */
export function GitBadge({ session }: { session: InstanceInfo }): ReactNode {
  if (!session.git_branch) return null;
  const dirty = session.git_dirty_count;
  return (
    <span className="inline-flex min-w-0 items-center gap-0.5 text-[10px] text-muted" title={`branch ${session.git_branch}${dirty ? ` · ${dirty} dirty` : ""}`}>
      <IconGitBranch width={10} height={10} />
      <span className="truncate">{session.git_branch}</span>
      {dirty ? <span className="shrink-0 text-warning">· {dirty}●</span> : null}
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

const LEGEND_STATES: { label: string; Icon: typeof IconStateRunning; tone: StateChipTone; meaning: string }[] = [
  { label: "Starting", Icon: IconStateStarting, tone: "accent", meaning: "starting" },
  { label: "Running", Icon: IconStateRunning, tone: "live", meaning: "running" },
  { label: "Needs you", Icon: IconStateWaiting, tone: "warning", meaning: "waiting_user" },
  { label: "Stuck?", Icon: IconStateStuck, tone: "warning", meaning: "stuck (heuristic)" },
  { label: "Quota blocked", Icon: IconStateQuota, tone: "danger", meaning: "quota_blocked" },
  { label: "Finished", Icon: IconStateFinished, tone: "success", meaning: "finished" },
  { label: "Stopped", Icon: IconStateStopped, tone: "neutral", meaning: "stopped" },
  { label: "Failed", Icon: IconStateFailed, tone: "danger", meaning: "failed" },
  { label: "Handed over", Icon: IconStateHandover, tone: "accent", meaning: "handed_over" },
];

/**
 * The one legend (Req.6): 9 states as icon+text+colour, the quota bar
 * semantics (fill = used %, tier tag Meas/Comp/Est = confidence), and
 * the sparkline semantics (solid = RAM, outline = CPU%). Rendered
 * identically wherever it appears — rail footer `?` popover, Overview
 * header, and the all-clear empty state.
 */
export function IndicatorLegend({ compact = false }: { compact?: boolean }) {
  return (
    <div className={compact ? "flex flex-col gap-1.5 text-[10px]" : "flex flex-col gap-2 text-[11px]"} role="list" aria-label="Indicator legend">
      <div className="grid grid-cols-1 gap-1">
        {LEGEND_STATES.map((state) => (
          <span key={state.label} className="flex min-w-0 items-center gap-2" role="listitem">
            <state.Icon width={12} height={12} className={legendTone(state.tone)} />
            <span className="font-medium text-ink">{state.label}</span>
            <span className="truncate text-faint">· {state.meaning}</span>
          </span>
        ))}
      </div>
      <span className="flex min-w-0 flex-wrap items-center gap-1.5 border-t border-hairline pt-1.5 text-muted" role="listitem">
        <span className="relative inline-block h-1.5 w-10 overflow-hidden rounded-full bg-surface-3">
          <span className="absolute inset-y-0 left-0 w-2/3 rounded-full bg-accent" />
        </span>
        <span>Quota fill = used %</span>
        <span className="rounded border border-hairline px-1 text-[9px] uppercase text-faint">Meas/Comp/Est</span>
        <span>= confidence</span>
      </span>
      <span className="flex min-w-0 items-center gap-2 text-muted" role="listitem">
        <svg width={26} height={10} aria-hidden="true" focusable="false">
          <polyline points="1,5 8,2 15,7 22,4 25,5" fill="none" stroke="var(--color-accent-hi)" strokeWidth={1.6} />
        </svg>
        <span>sparkline solid = RAM</span>
        <svg width={26} height={10} aria-hidden="true" focusable="false">
          <polyline points="1,5 8,7 15,3 22,6 25,4" fill="none" stroke="var(--color-live)" strokeWidth={1.2} strokeDasharray="2 1.6" />
        </svg>
        <span>outline = CPU%</span>
      </span>
      <span className="flex items-center gap-1.5 border-t border-hairline pt-1.5 text-faint" role="listitem">
        <span className="inline-flex items-center rounded border border-warning/40 bg-warning-soft px-1 text-[9px] font-medium text-warning">stale - Nm</span>
        <span>older than 2× refresh — never shown as fresh</span>
      </span>
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

// --- WCAG 4.1.3: attention changes announced politely (AJ-E405) ---------

import { useEffect, useRef, useState } from "react";

/**
 * Announces attention transitions on a polite live region — state
 * changes are never conveyed by colour alone. The message is the badge
 * text plus the rule (`Needs you — waiting_prompt`).
 */
export function AttentionAnnouncer({ sessions }: { sessions: InstanceInfo[] }) {
  const [message, setMessage] = useState("");
  const previous = useRef<Set<string>>(new Set());

  useEffect(() => {
    const current = new Set(sessions.filter((row) => row.attention !== null).map((row) => row.id));
    const labelOf = (id: string) =>
      sessions.find((row) => row.id === id)?.account_label ?? "Instance";
    const fresh = [...current].filter((id) => !previous.current.has(id));
    const recovered = [...previous.current].filter((id) => !current.has(id));
    if (fresh.length > 0) {
      const parts = fresh.map((id) => {
        const row = sessions.find((entry) => entry.id === id);
        return `${labelOf(id)} needs you — ${row?.attention?.rule ?? row?.needs_attention ?? "attention"}`;
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
