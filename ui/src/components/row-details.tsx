/**
 * A session row's screen-reader text (WCAG 2.5.3): the full field text
 * for `aria-label`, and the details a row doesn't show as text for its
 * `aria-describedby` — in the rows' own words.
 */
import type { ReactNode } from "react";
import type { InstanceInfo } from "../lib/bridge";
import {
  ageSummary,
  attentionBadge,
  attentionSuffix,
  attentionTooltip,
  gitWords,
  isActiveStatus,
  providerMark,
  tokenSummary,
} from "./indicator";
import { quotaTierTag } from "./primitives";
import { sessionState } from "../lib/rail-items";
import { subagentsRunning } from "../lib/subagents";
import { memorySize } from "../lib/compact";

/** Full field text for `aria-label`: every number reachable as text. */
export function ariaSummary(session: InstanceInfo, nowMs: number = Date.now()): string {
  const mark = providerMark(session.provider);
  const badge = attentionBadge(session);
  const parts: string[] = [
    `${mark.abbrev} provider`,
    session.account_label,
    session.model ?? "default model",
    // AJ-E2104 Req.3: the rail's words for its state.
    sessionState(session).word,
  ];
  const quota = quotaWords(session);
  if (quota) parts.push(quota);
  const tokens = tokenSummary(session.tokens);
  if (tokens) parts.push(tokens);
  if (session.cpu_pct !== null && session.rss_bytes !== null) {
    parts.push(`cpu ${Math.round(session.cpu_pct)}%, ram ${memorySize(session.rss_bytes)}`);
  }
  if (session.child_count) parts.push(`${session.child_count} children`);
  if (session.subtask_count) parts.push(subagentsRunning(session.subtask_count));
  if (session.git_branch) parts.push(gitWords(session));
  parts.push(ageSummary(session.started_at_ms, session.last_activity_ms, nowMs));
  const suffix = attentionSuffix(session, nowMs);
  if (badge && suffix) parts.push(suffix);
  const reason = attentionTooltip(session);
  if (reason) parts.push(reason);
  return parts.join(", ");
}

/** A row's quota in words: a live session's account left, with its
 * tier; a finished session says nothing about it (AJ-E1535). */
function quotaWords(session: InstanceInfo): string | null {
  if (!isActiveStatus(session.status)) return null;
  if (!session.quota) return "Quota: not measured yet";
  return `quota ${Math.round(100 - session.quota.used_percentage)}% left, ${quotaTierTag(session.quota.confidence)}`;
}

/**
 * The details a row doesn't show as text, for its `aria-describedby`
 * (WCAG 2.5.3 — the row's name is its visible content; everything else
 * a screen reader needs is here).
 */
export function ariaDetails(session: InstanceInfo, nowMs: number = Date.now()): string {
  const parts: string[] = [];
  const quota = quotaWords(session);
  if (quota) parts.push(quota);
  const tokens = tokenSummary(session.tokens);
  if (tokens) parts.push(tokens);
  if (session.cpu_pct !== null && session.rss_bytes !== null) {
    parts.push(`cpu ${Math.round(session.cpu_pct)}%, ram ${memorySize(session.rss_bytes)}`);
  }
  if (session.child_count) parts.push(`${session.child_count} children`);
  if (session.subtask_count) parts.push(subagentsRunning(session.subtask_count));
  parts.push(ageSummary(session.started_at_ms, session.last_activity_ms, nowMs));
  const reason = attentionTooltip(session);
  if (reason) parts.push(reason);
  return parts.join(", ");
}

/**
 * A row's described-by text: `hidden`, so it never joins the row's name
 * from content, but still read as its description.
 */
export function RowDetails({ id, session }: { id: string; session: InstanceInfo }): ReactNode {
  return (
    <span id={id} hidden>
      {ariaDetails(session)}
    </span>
  );
}
