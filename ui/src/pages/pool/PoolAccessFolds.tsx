/**
 * D-28, D-29 (AJ-E3011 round 3): what agents may do in one pool beyond
 * the level — a Custom level's actions, what you told it to always allow
 * (each taken back with a click), and how messages go into sessions —
 * one component for the pool's inspector and Settings › Agent access, so
 * the two can't say or set it differently. Each is a fold with one plain
 * line on what it controls.
 */
import type { JSX } from "react";

import { ChoiceOption, Picker } from "../../components/controls";
import { AGENT_ACTIONS, agentRuleRevoke, poolSetAccess, poolSetDelivery, type PoolInfo } from "../../lib/ipc/pools";
import { ACTION_WORDS } from "../../lib/pools";
import { agoWords } from "../../lib/time-words";
import { FoldSection, Section } from "./Section";

const HOPS = Array.from({ length: 20 }, (_, index) => ({ value: String(index + 1), label: String(index + 1) }));

/** The folds' titles and lines. */
export const ACCESS_FOLDS = {
  rules: { title: "Allowed without asking", line: "What you told this pool to always allow, so its agents stop asking you. Take one back any time." },
  delivery: { title: "Messages into sessions", line: "How a message reaches a session that is working, and when a back-and-forth between agents stops to ask you." },
};

/** How a message gets into a session here, in plain words (D-29; round 3
 * #2: "When you click Deliver" didn't say what was delivered, or where). */
export function deliveryWords(pool: PoolInfo): string {
  if (pool.access.delivery_stopped) return "Stopped: messages stay in the inboxes, and none goes into a session.";
  const own = pool.access.level === "full" || pool.rules.some((rule) => rule.action === "deliver_to_session");
  return own ? "They go into the session by themselves, between its turns." : "Each one waits for your OK in Needs you, then goes into the session.";
}

/** A Custom level's actions, each picked by hand: the one list, in the
 * pool's inspector, Settings and the diagram header's Custom (the Sam
 * walk: choosing Custom there changed only its label). */
export function CustomActions({ pool, busy, run, testId = "pool-custom" }: { pool: PoolInfo; busy: boolean; run: (work: () => Promise<unknown>) => void; testId?: string }): JSX.Element {
  const allowed = new Set(pool.access.actions);
  return (
    <div className="-mx-2 flex flex-col" data-testid={testId}>
      {AGENT_ACTIONS.filter((action) => action !== "deliver_to_session").map((action) => (
        <ChoiceOption
          key={action}
          type="checkbox"
          label={ACTION_WORDS[action] ?? action}
          checked={allowed.has(action)}
          disabled={busy}
          onChange={(event) => {
            const on = event.target.checked;
            run(() => poolSetAccess(pool.id, "custom", on ? [...allowed, action] : [...allowed].filter((word) => word !== action)));
          }}
        />
      ))}
    </div>
  );
}

export function PoolAccessFolds({ pool, now, busy, run }: { pool: PoolInfo; now: number; busy: boolean; run: (work: () => Promise<unknown>) => void }): JSX.Element {
  return (
    <>
      {pool.access.level === "custom" && (
        <Section title="Custom actions" testId="inspect-pool-level">
          <CustomActions pool={pool} busy={busy} run={run} />
        </Section>
      )}

      <FoldSection title={ACCESS_FOLDS.rules.title} line={ACCESS_FOLDS.rules.line} testId="pool-rules">
        {pool.rules.length === 0 ? (
          <p className="text-label text-muted">None yet. When you allow the same thing twice, the ask offers to remember it here.</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {pool.rules.map((rule) => (
              <li key={rule.id} className="flex items-center gap-2 rounded-lg border border-hairline/70 px-2.5 py-1.5" data-testid="pool-rule">
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-meta text-ink">{ACTION_WORDS[rule.action] ?? rule.action}</span>
                  <span className="text-label text-faint">You, {agoWords(rule.created_at, now)}</span>
                </span>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => run(() => agentRuleRevoke(rule.id))}
                  className="shrink-0 rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                  data-testid="pool-rule-revoke"
                >
                  Take back<span className="sr-only">: {ACTION_WORDS[rule.action] ?? rule.action}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </FoldSection>

      <FoldSection title={ACCESS_FOLDS.delivery.title} line={ACCESS_FOLDS.delivery.line} testId="pool-delivery">
        <p className="text-meta text-ink" data-testid="pool-delivery-words">
          {deliveryWords(pool)}
        </p>
        <div className="flex items-center justify-between gap-3">
          <span className="flex flex-col">
            <span className="text-meta text-ink">Stop and ask you after</span>
            <span className="text-label text-muted">messages in a row between agents, without you</span>
          </span>
          <Picker
            choice
            ariaLabel="Messages in a row between agents before it stops and asks you"
            heading="Stop and ask you after"
            size="sm"
            value={String(pool.access.hop_limit)}
            options={HOPS}
            disabled={busy}
            onChange={(hops) => run(() => poolSetDelivery(pool.id, Number(hops), pool.access.delivery_stopped))}
            testId="pool-hop-limit"
          />
        </div>
        <ChoiceOption
          type="checkbox"
          className="-mx-2"
          label="Keep every message out of sessions"
          hint="They still reach the inboxes; none goes into a session until you turn this off."
          checked={pool.access.delivery_stopped}
          disabled={busy}
          onChange={(event) => run(() => poolSetDelivery(pool.id, pool.access.hop_limit, event.target.checked))}
          data-testid="pool-stop-all"
        />
      </FoldSection>
    </>
  );
}
