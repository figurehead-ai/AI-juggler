/**
 * AJ-E409: a session's "How it's going" (D-07: signals, never a score),
 * on its Usage tab under its usage (AJ-E1533). Moved out of
 * RightInspector.tsx (its size budget).
 */
import { useState, type ReactNode } from "react";
import { Card } from "./primitives";
import { useQualitySignals } from "../lib/kept-reads";
import { resourceLine } from "../lib/machine";
import { modelName } from "./charts/colors";
import { providerDisplayName } from "../lib/providers";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** SPEC §6's honesty tiers, each with a plain line beside it (0.0.11
 * walk, Q5): the tag stays, and says what it means. */
const TIER_WORDS: Record<"Measured" | "Computed" | "Estimated", string> = {
  Measured: "read from the session",
  Computed: "worked out from what's measured",
  Estimated: "a best guess",
};

function TierTag({ tier }: { tier: "Measured" | "Computed" | "Estimated" }) {
  return (
    <span className="mt-auto pt-1 text-label text-muted" data-testid="tier-words" data-tier={tier}>
      {TIER_WORDS[tier]}
    </span>
  );
}

/** A stuck rule's id in words (the rail's own wording). */
function stuckRuleWords(rule: string): string {
  switch (rule) {
    case "stuck_no_output":
      return "No output for a while";
    case "waiting_prompt":
      return "Waiting for your reply";
    case "quota_exit": case "limit_hit": // ended, or open at its limit (0.3)
      return "Hit its limit";
    default:
      return "Looks stuck";
  }
}

export function QualityTab({ instance, visible }: { instance: InstanceInfo; visible: boolean }) {
  const { quality, loadError } = useQualitySignals(instance.id, visible);
  const [asTable, setAsTable] = useState(false);

  const signals: { testId: string; label: string; body: ReactNode; tier: "Measured" | "Computed" | "Estimated"; reason?: string }[] = [];
  if (loadError) {
    return (
      <Card className="flex flex-col gap-1 p-4" data-testid="quality-tab">
        <p className="text-meta text-ink-2">Signals unavailable right now.</p>
        <p className="text-label text-muted">{loadError}</p>
      </Card>
    );
  }
  if (quality) {
    signals.push({
      testId: "quality-tokens",
      label: "Tokens / min",
      tier: "Measured",
      body: quality.tokens_bars ? (
        <div className="flex flex-col gap-1">
          <span className="tabular-nums text-ink">
            {Math.round(quality.avg_tokens_per_min)} a minute on average over the last 5 minutes
          </span>
          {asTable ? (
            <table className="text-label">
              <tbody>
                <tr>
                  <td className="pr-2 text-muted">minutes ago</td>
                  {[5, 4, 3, 2, 1].map((m) => (
                    <td key={m} className="pr-2 text-muted">{m}</td>
                  ))}
                </tr>
                <tr>
                  <td className="pr-2 text-muted">tokens</td>
                  {quality.tokens_bars.map((bar, i) => (
                    <td key={i} className="pr-2 tabular-nums text-ink">{bar}</td>
                  ))}
                </tr>
              </tbody>
            </table>
          ) : (
            <div className="flex items-end gap-1" aria-hidden="true">
              {quality.tokens_bars.map((bar, i) => (
                <span key={i} className="w-3 rounded-sm bg-accent/70" style={{ height: `${Math.max(3, (bar / Math.max(...quality.tokens_bars!)) * 24)}px` }} />
              ))}
            </div>
          )}
        </div>
      ) : (
        <span className="text-muted">Nothing measured yet</span>
      ),
    });
    // AJ-E1533: one source per fact — the same live reading the
    // session's header shows, kept current by the live updates.
    const measured = instance.cpu_pct !== null || instance.rss_bytes !== null;
    signals.push({
      testId: "quality-cpu",
      label: "CPU and memory now",
      tier: "Measured",
      body: measured ? (
        <span className="tabular-nums text-ink">{resourceLine(instance.cpu_pct, instance.rss_bytes, instance.child_count)}</span>
      ) : (
        <span className="text-muted">Collecting first samples…</span>
      ),
    });
    signals.push({
      testId: "quality-errors",
      label: "Errors",
      tier: "Measured",
      body: quality.errors_not_measurable ? (
        <span className="text-muted">Can't be counted in an interactive session</span>
      ) : (
        <span className="tabular-nums text-ink">{quality.error_label}</span>
      ),
    });
    signals.push({
      testId: "quality-context",
      label: "Context",
      tier: "Measured",
      body:
        quality.context_used !== null && quality.context_window ? (
          <span className="tabular-nums text-ink">
            {Math.round((quality.context_used / quality.context_window) * 100)}% ({quality.context_used.toLocaleString()} / {quality.context_window.toLocaleString()})
          </span>
        ) : (
          <span className="text-muted">
            {providerDisplayName(instance.provider)} doesn't say how full its context is ·{" "}
            <span className="tabular-nums text-ink">
              {(quality.cumulative.input + quality.cumulative.output).toLocaleString()}
            </span>{" "}
            tokens in and out in this session so far
          </span>
        ),
    });
    // AJ-E2903: an ended session's context at its end and how often it got stuck, as kept when it ended.
    if (["finished", "failed", "stopped", "handed_over", "quota_blocked"].includes(instance.status)) {
      const [used, window] = quality.end_context ?? [0, 0];
      signals.push({
        testId: "quality-context-end",
        label: "Context at finish",
        tier: "Measured",
        body: window > 0 ? <span className="tabular-nums text-ink">{Math.round((used / window) * 100)}% ({used.toLocaleString()} / {window.toLocaleString()})</span> : <span className="text-muted">Not reported by {providerDisplayName(instance.provider)}</span>,
      });
      signals.push({
        testId: "quality-stuck-episodes",
        label: "Times probably stuck",
        tier: "Computed",
        body: <span className="text-ink">{quality.stuck_episodes === 0 ? "Not once" : `${quality.stuck_episodes} ${quality.stuck_episodes === 1 ? "time" : "times"}, from the states AI Juggler recorded`}</span>,
      });
    }
    signals.push({
      testId: "quality-cost",
      label: "Session cost",
      tier: "Estimated",
      body: quality.session_cost_usd !== null ? (
        <span className="tabular-nums text-ink">
          ~${quality.session_cost_usd.toFixed(2)}{quality.priced_model ? ` for ${modelName(quality.priced_model)}` : ""} · what the API would charge, not your bill
        </span>
      ) : (
        <span className="text-muted">
          No price known for this model, so no cost is shown
        </span>
      ),
    });
    signals.push({
      testId: "quality-forecast",
      label: "How long its limit lasts",
      tier: "Estimated",
      body: <span className="text-ink">{quality.quota_forecast}</span>,
    });
    if (quality.stuck) {
      signals.push({
        testId: "quality-stuck",
        label: "Probably stuck",
        tier: "Computed",
        body: (
          <span className="text-ink">
            {stuckRuleWords(quality.stuck.rule)} · {quality.stuck.inputs.join(" · ")}
          </span>
        ),
      });
    }
  }

  return (
    <section className="flex flex-col gap-3" data-testid="quality-tab" aria-labelledby="quality-title">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h3 id="quality-title" className="text-section font-semibold text-ink">
          How it's going
        </h3>
        {signals.length > 0 && (
          <span className="flex items-center gap-3">
            <span className="text-meta text-muted">every 2 seconds</span>
            <button
              type="button"
              onClick={() => setAsTable((v) => !v)}
              aria-pressed={asTable}
              className="rounded-lg border border-hairline px-2 py-0.5 text-label text-muted outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            >
              {asTable ? "View as chart" : "View as table"}
            </button>
          </span>
        )}
      </div>
      {signals.length === 0 ? (
        <Card className="flex flex-col gap-1 p-5">
          <p className="text-body text-ink-2">Nothing measured yet — this fills in as the session works.</p>
        </Card>
      ) : (
        // Cards sized to what they say, as many to a row as fit.
        <div className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,300px),1fr))] gap-grid">
          {signals.map((signal) => (
            <Card key={signal.testId} className="flex flex-col gap-1.5 p-5 text-body" data-testid={signal.testId}>
              <p className="text-meta font-medium text-ink-2">{signal.label}</p>
              {signal.body}
              <TierTag tier={signal.tier} />
            </Card>
          ))}
        </div>
      )}
    </section>
  );
}
