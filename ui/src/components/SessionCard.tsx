/**
 * The two-line session card (AJ-E402 Req.4), moved out of WorkflowRail.tsx
 * (AJ-E2104).
 */
import { useId } from "react";
import { AccountAvatar } from "./Avatar";
import type { InstanceInfo } from "../lib/bridge";
import {
  ChildBadges,
  GitBadge,
  attentionBadge,
  attentionSuffix,
  attentionTooltip,
  providerMark,
  rememberResourceSample,
  stateChip,
  tokenSummary,
  ageSummary,
  formatBytes,
} from "./indicator";
import { QuotaBar, Sparkline } from "./primitives";
import { RowDetails } from "./row-details";
import { chipToneClasses } from "./SessionCards";

/**
 * The two-line card variant (AJ-E402 Req.4): identical I-01..I-11 fields
 * to the rail row, sparkline widened to the 5-min window. Used by the
 * Overview live list — no number exists here that the rail cannot
 * explain.
 */
export function SessionCard({ session, onSelect }: { session: InstanceInfo; onSelect?: (id: string) => void }) {
  const detailsId = useId();
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
  return (
    <button
      type="button"
      onClick={onSelect ? () => onSelect(session.id) : undefined}
      aria-describedby={detailsId}
      className="flex w-full min-w-0 flex-col gap-1.5 rounded-xl px-3 py-2.5 text-left outline-none transition-colors hover:bg-surface-2/60 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <RowDetails id={detailsId} session={session} />
      <span className="flex min-w-0 items-center gap-2">
        <span className="inline-flex shrink-0 items-center gap-1.5 text-[11.5px] font-medium text-ink">
          <mark.Icon width={12} height={12} className="text-accent-hi" />
          {mark.abbrev}
        </span>
        <AccountAvatar accountId={session.account_id} label={session.account_label} size={20} />
        <span className="min-w-0 truncate text-[12px] text-ink">{session.account_label}</span>
        <span className="min-w-0 shrink-0 truncate text-[11px] text-muted">
          {session.model ?? <span className="text-faint">default model</span>}
        </span>
        <span className="ml-auto inline-flex shrink-0 items-center gap-1">
          {badge ? (
            <span
              title={attentionTooltip(session)}
              className={
                "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium " +
                chipToneClasses[badge.tone]
              }
            >
              {badge.tone === "warning" || badge.tone === "danger" ? <span aria-hidden="true">!</span> : null}
              {badge.text}
            </span>
          ) : (
            <span
              className={
                "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium " +
                chipToneClasses[chip.tone]
              }
            >
              <chip.Icon width={11} height={11} />
              {chip.label}
            </span>
          )}
        </span>
      </span>
      <span className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted">
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
        <QuotaBar quota={session.quota} />
        {tokens ? <span className="tabular-nums">{tokens}</span> : null}
        <ChildBadges session={session} />
        <GitBadge session={session} />
        <span className="ml-auto inline-flex shrink-0 items-center gap-2">
          <Sparkline
            series={samples}
            width={64}
            title={resources ? `ram/cpu ${resources}` : "resources unavailable"}
          />
          {resources ? <span className="tabular-nums">{resources}</span> : null}
        </span>
        <span className="shrink-0 whitespace-nowrap tabular-nums text-faint">
          {ageSummary(session.started_at_ms, session.last_activity_ms)}
        </span>
      </span>
    </button>
  );
}
