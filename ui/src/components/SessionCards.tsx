/**
 * The session row and card with every indicator (AJ-E402/E403: I-01 to
 * I-11), moved out of WorkflowRail.tsx (AJ-E2104) so the rail stays
 * within its size. The rail's own rows are RailRow and the place rows.
 */
import { useId } from "react";
import { AccountAvatar } from "./Avatar";
import { identityWords } from "../lib/identity";
import { parkedWords } from "../lib/parked";
import { providerDisplayName } from "../lib/providers";
import type { InstanceInfo } from "../lib/bridge";
import type { Freshness } from "../lib/live-deltas";
import { REFRESH_INTERVALS, staleCaption } from "../lib/density";
import {
  ChildBadges,
  GitBadge,
  StaleChip,
  attentionBadge,
  attentionSuffix,
  attentionTooltip,
  isActiveStatus,
  providerMark,
  rememberResourceSample,
  stateChip,
  tokenSummary,
  ageSummary,
  formatBytes,
  type StateChipTone,
} from "./indicator";
import { QuotaBar, Sparkline } from "./primitives";
import { RowDetails } from "./row-details";

/** Tone classes for the compact state/attention chip (token-driven). */
export const chipToneClasses: Record<StateChipTone, string> = {
  neutral: "bg-surface-2 text-muted border-hairline",
  success: "bg-success-soft text-success border-success/25",
  warning: "bg-warning-soft text-warning border-warning/25",
  danger: "bg-danger-soft text-danger border-danger/25",
  live: "bg-live-soft text-live border-live/25",
  accent: "bg-accent-soft text-accent-hi border-accent/25",
};

/**
 * The indicator row (AJ-E402/E403): I-01..I-11 in pass2/04 §3 render
 * order — provider mark, account disc, model chip, state chip, quota bar
 * with tier tag, tokens, children, git, attention, age, resource
 * sparkline. Absent data renders the exact honest copy (`Quota: no
 * data`, `default model`, `Unknown`, `—`), never fabricated numbers.
 *
 * Densities: Comfortable shows all I-01..I-11; Compact (~36 px, 2
 * lines) shows I-01, I-02, I-04, I-05 bar-only, I-09, I-10 plus CPU/RAM
 * as text. I-04 text and I-10 are NEVER hidden in either. Values older
 * than 2× their refresh interval carry the `stale - Nm` chip (AJ-E403
 * Req.5) — the value keeps its colour (contrast-safe), the caption
 * carries the meaning.
 */
/**
 * I-03 drift (AJ-E412 Req.3): true when a running model was observed
 * and differs from the launch overlay. The marker's text is the full
 * `Launched with X - now running Y` (WCAG 1.3.1: text, not just a dot).
 */
export function modelDrift(session: InstanceInfo): string | null {
  if (
    session.running_model !== null &&
    session.launch_model !== null &&
    session.running_model !== session.launch_model
  ) {
    return `Launched with ${session.launch_model} - now running ${session.running_model}`;
  }
  return null;
}

export function SessionRow({
  session,
  selected,
  onSelect,
  density = "comfortable",
  freshness = {},
  now = Date.now(),
}: {
  session: InstanceInfo;
  selected: boolean;
  onSelect: (id: string) => void;
  density?: "comfortable" | "compact";
  freshness?: Freshness;
  now?: number;
}) {
  const mark = providerMark(session.provider);
  const badge = attentionBadge(session);
  const chip = stateChip(session.status);
  const suffix = attentionSuffix(session);
  const tokens = tokenSummary(session.tokens);
  const samples = rememberResourceSample(session.id, session.cpu_pct, session.rss_bytes);
  const resources =
    session.cpu_pct !== null && session.rss_bytes !== null
      ? `${Math.round(session.cpu_pct)}% · ${formatBytes(session.rss_bytes)}`
      : null;
  const sampled = freshness[session.id];
  const quotaStale =
    session.quota !== null && sampled
      ? staleCaption(sampled.quota, REFRESH_INTERVALS.quota, now)
      : null;
  const resourceStale =
    resources !== null && sampled
      ? staleCaption(sampled.resources, REFRESH_INTERVALS.resources, now)
      : null;
  // Terminal rows never show resource slots: a finished Instance is not
  // "unavailable" — it is done. Active-but-unsampled keeps the honest `—`.
  const showResources = isActiveStatus(session.status);
  // WCAG 2.5.3 (AJ-E804): the row's name is its visible content; the
  // rest is its description.
  const detailsId = useId();

  if (density === "compact") {
    return (
      <button
        type="button"
        onClick={() => onSelect(session.id)}
        aria-describedby={detailsId}
        data-instance-id={session.id}
        className={
          "relative flex min-w-0 w-full flex-col gap-0.5 rounded-lg px-2.5 py-1.5 pl-3 text-left outline-none transition-colors " +
          "focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface" : "hover:bg-surface/60")
        }
      >
        <RowDetails id={detailsId} session={session} />
        {selected && (
          <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
        )}
        {/* L1: I-01 I-02 … I-04/I-10 (text never hidden in Compact) */}
        <span className="flex min-w-0 items-center gap-1.5 overflow-hidden">
          <span
            className="inline-flex min-w-0 shrink items-center gap-1 text-[10.5px] font-medium text-ink"
            title={providerDisplayName(session.provider)}
          >
            <mark.Icon width={11} height={11} className="shrink-0 text-accent-hi" />
            <span className="truncate">{mark.abbrev}</span>
          </span>
          <AccountAvatar accountId={session.account_id} label={session.account_label} />
          <span className="sr-only">{session.account_label}</span>
          <span className="ml-auto inline-flex shrink-0 items-center gap-1">
            {badge ? (
              <span
                className={
                  "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                  chipToneClasses[badge.tone]
                }
              >
                {badge.tone === "warning" || badge.tone === "danger" ? <span aria-hidden="true">!</span> : null}
                {badge.text}
              </span>
            ) : (
              <span
                className={
                  "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                  chipToneClasses[chip.tone]
                }
              >
                <chip.Icon width={10} height={10} />
                {chip.label}
              </span>
            )}
          </span>
        </span>
        {/* L2: I-05 bar-only + I-09 + CPU/RAM text (sparkline → text) */}
        <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-muted">
          <span
            className="relative inline-block h-1.5 w-12 shrink-0 overflow-hidden rounded-full bg-surface-3"
            title={session.quota ? `${Math.round(100 - session.quota.used_percentage)}% left · ${session.quota.confidence}` : "Quota: not measured yet"}
          >
            {session.quota ? (
              <span
                className={
                  "absolute inset-y-0 left-0 rounded-full " +
                  (100 - session.quota.used_percentage <= 10
                    ? "bg-danger"
                    : 100 - session.quota.used_percentage <= 30
                      ? "bg-warning"
                      : "bg-accent")
                }
                style={{ width: `${Math.max(2, session.quota.used_percentage)}%` }}
              />
            ) : (
              <span className="absolute inset-0 rounded-full border border-dashed border-faint/70" />
            )}
          </span>
          <StaleChip caption={quotaStale} />
          <GitBadge session={session} />
          {showResources && (
            <span className="ml-auto inline-flex min-w-0 shrink items-center gap-1">
              {resources ? (
                <span className="min-w-0 truncate tabular-nums">{resources}</span>
              ) : (
                <span className="text-faint" title="resources unavailable">
                  <span aria-hidden="true">—</span>
                  <span className="sr-only">resources unavailable</span>
                </span>
              )}
              <StaleChip caption={resourceStale} />
            </span>
          )}
        </span>
      </button>
    );
  }

  return (
    <button
      type="button"
      onClick={() => onSelect(session.id)}
      aria-describedby={detailsId}
      data-instance-id={session.id}
      className={
        "relative flex min-w-0 w-full flex-col gap-1 rounded-lg px-2.5 py-2 pl-3 text-left outline-none transition-colors " +
        "focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (selected ? "bg-surface" : "hover:bg-surface/60")
      }
    >
      <RowDetails id={detailsId} session={session} />
      {selected && (
        <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
      )}
      {/* I-01 I-02 I-03 … I-04/I-10. Narrow rails make the provider
          abbrev and model yield first (truncate); the disc, chip text and
          glyphs never clip (WCAG 1.4.1). */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden">
        <span
          className="inline-flex min-w-0 shrink items-center gap-1 text-[10.5px] font-medium text-ink"
          title={providerDisplayName(session.provider)}
        >
          <mark.Icon width={11} height={11} className="shrink-0 text-accent-hi" />
          <span className="truncate">{mark.abbrev}</span>
        </span>
        <AccountAvatar accountId={session.account_id} label={session.account_label} />
          <span className="sr-only">{session.account_label}</span>
        {identityWords(session) && (
          <span
            title={identityWords(session)!.text}
            aria-label={identityWords(session)!.text}
            tabIndex={0}
            data-testid="rail-identity"
            data-state={identityWords(session)!.state}
            className="shrink-0 rounded-full border border-hairline px-1 text-[9.5px] font-medium text-muted outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            {identityWords(session)!.glyph}
          </span>
        )}
        {session.kind === "desktop" && (
          <span
            title="desktop app session"
            className="shrink-0 rounded-full border border-accent/25 bg-accent-soft px-1 text-[9.5px] font-medium text-accent-hi"
            data-testid="rail-app-chip"
          >
            app
          </span>
        )}
        <span className="min-w-0 shrink truncate text-[10.5px] text-muted" title={session.model ?? "default model"}>
          {session.model ?? <span className="text-faint">default model</span>}
        </span>
        {modelDrift(session) && (
          <span
            title={modelDrift(session)!}
            aria-label={modelDrift(session)!}
            tabIndex={0}
            data-testid="model-drift"
            className="shrink-0 rounded-full border border-warning/25 bg-warning-soft px-1 text-[10px] font-medium text-warning outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            ⇄
          </span>
        )}
        <span className="ml-auto inline-flex shrink-0 items-center gap-1">
          {parkedWords(session, now) ? (
            <span
              title={parkedWords(session, now)!.long}
              aria-label={parkedWords(session, now)!.long}
              data-testid="rail-parked"
              className="inline-flex items-center gap-1 rounded-full border border-hairline px-1.5 py-px text-[10px] font-medium text-ink"
            >
              <span aria-hidden="true">⏸</span>
              {parkedWords(session, now)!.chip}
            </span>
          ) : badge ? (
            <span
              title={attentionTooltip(session)}
              className={
                "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                chipToneClasses[badge.tone]
              }
            >
              {badge.tone === "warning" || badge.tone === "danger" ? <span aria-hidden="true">!</span> : null}
              {badge.text}
            </span>
          ) : (
            <span
              className={
                "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[10px] font-medium " +
                chipToneClasses[chip.tone]
              }
            >
              <chip.Icon width={10} height={10} />
              {chip.label}
            </span>
          )}
        </span>
      </span>
      {/* I-10 suffix (wait duration / reset countdown) + I-09 I-11. The
          age text yields (truncate) before anything else clips. */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-muted">
        {badge && suffix ? (
          <span
            className={
              "shrink-0 whitespace-nowrap font-medium " +
              (badge.tone === "danger" ? "text-danger" : "text-warning")
            }
          >
            {suffix}
          </span>
        ) : null}
        <GitBadge session={session} />
        <span className="ml-auto min-w-0 shrink truncate whitespace-nowrap tabular-nums">
          {ageSummary(session.started_at_ms, session.last_activity_ms)}
        </span>
      </span>
      {/* I-05 (+ stale chip when the sample expired) */}
      <span className="flex min-w-0 items-center gap-1.5">
        <QuotaBar quota={session.quota} />
        <StaleChip caption={quotaStale} />
      </span>
      {/* I-06 I-08 I-07 — right-hand resource text yields before anything
          clips. */}
      <span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[10px] text-muted">
        {tokens ? <span className="shrink-0 tabular-nums">{tokens}</span> : null}
        <ChildBadges session={session} />
        {showResources && (
          <span className="ml-auto inline-flex min-w-0 shrink items-center gap-1.5">
            <Sparkline
              series={samples}
              title={resources ? `ram/cpu ${resources}` : "resources unavailable"}
            />
            {resources ? <span className="min-w-0 truncate tabular-nums">{resources}</span> : null}
            <StaleChip caption={resourceStale} />
          </span>
        )}
      </span>
      {session.kind === "desktop" && (
        <span className="self-start text-[9px] font-medium uppercase tracking-wide text-accent-hi">
          app
        </span>
      )}
    </button>
  );
}
