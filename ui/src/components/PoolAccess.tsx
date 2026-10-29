import { useId, useState, type JSX } from "react";

import { errorText } from "../lib/bridge";
import { AGENT_ACTIONS, agentRuleRevoke, poolSetAccess, poolSetDelivery, type PoolInfo, type PoolsInfo } from "../lib/ipc/pools";
import { ACTION_WORDS, LEVELS, setPools } from "../lib/pools";
import { agoWords } from "../lib/time-words";
import { Checkbox, Picker, Radio } from "./controls";

/**
 * D-28: what agents may do in one pool — its level, picked by you here
 * only; a Custom level's actions; how far a delivered message may go
 * (D-29); and the rules you made, each one taken back with a click (it
 * stops applying on the very next request). Above the level, an agent's
 * request becomes a question in Needs you, never a block.
 */
export function PoolAccessPanel({ pool, now = Date.now() }: { pool: PoolInfo; now?: number }): JSX.Element {
  const name = useId();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (work: () => Promise<PoolsInfo>) => {
    setBusy(true);
    setError(null);
    try {
      setPools(await work());
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const level = pool.access.level;
  const allowed = new Set(pool.access.actions);
  return (
    <div className="flex flex-col gap-2.5" data-testid="pool-access" data-pool={pool.id}>
      <fieldset className="flex flex-col gap-1" disabled={busy}>
        <legend className="sr-only">What agents may do</legend>
        {LEVELS.map((entry) => (
          <label key={entry.id} className="flex cursor-pointer items-start gap-2 rounded-lg px-1.5 py-1 text-xs hover:bg-surface-2" data-testid={`pool-level-${entry.id}`}>
            <Radio
              name={name}
              checked={level === entry.id}
              onChange={() => void run(() => poolSetAccess(pool.id, entry.id, entry.id === "custom" ? pool.access.actions : []))}
              className="mt-0.5"
            />
            <span className="flex flex-col">
              <span className="font-medium text-ink">{entry.label}</span>
              <span className="text-label text-muted">{entry.words}</span>
            </span>
          </label>
        ))}
      </fieldset>

      {level === "custom" && (
        <fieldset className="ml-6 grid grid-cols-1 gap-0.5 sm:grid-cols-2" disabled={busy} data-testid="pool-custom">
          <legend className="sr-only">Actions</legend>
          {AGENT_ACTIONS.filter((action) => action !== "deliver_to_session").map((action) => (
            <label key={action} className="flex items-center gap-2 text-label text-ink">
              <Checkbox
                checked={allowed.has(action)}
                onChange={(event) => {
                  const next = event.target.checked ? [...allowed, action] : [...allowed].filter((word) => word !== action);
                  void run(() => poolSetAccess(pool.id, "custom", next));
                }}
              />
              {ACTION_WORDS[action]}
            </label>
          ))}
        </fieldset>
      )}

      <details className="rounded-lg border border-hairline/60 px-2.5 py-1.5" data-testid="pool-delivery">
        <summary className="cursor-pointer select-none text-label text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
          Messages put into sessions
        </summary>
        <div className="mt-2 flex flex-col gap-2 text-xs">
          <div className="flex items-center gap-2 text-ink">
            Pause and ask after
            <Picker
              size="sm"
              ariaLabel="Replies in a row before it pauses and asks"
              value={String(pool.access.hop_limit)}
              disabled={busy}
              onChange={(value) => void run(() => poolSetDelivery(pool.id, Number(value), pool.access.delivery_stopped))}
              testId="pool-hop-limit"
              options={Array.from({ length: 20 }, (_, index) => String(index + 1)).map((hops) => ({ value: hops, label: hops }))}
            />
            replies in a row
          </div>
          <label className="flex items-center gap-2 text-ink">
            <Checkbox
              checked={pool.access.delivery_stopped}
              disabled={busy}
              onChange={(event) => void run(() => poolSetDelivery(pool.id, pool.access.hop_limit, event.target.checked))}
              data-testid="pool-stop-all"
            />
            Stop all of them in this pool
          </label>
        </div>
      </details>

      {pool.rules.length > 0 && (
        <section aria-label="Always allowed" className="flex flex-col gap-1" data-testid="pool-rules">
          <p className="text-label uppercase tracking-[0.14em] text-faint">Always allowed here</p>
          <ul className="flex flex-col gap-1">
            {pool.rules.map((rule) => (
              <li key={rule.id} className="flex items-center gap-2 rounded-lg border border-hairline/60 px-2.5 py-1.5 text-xs" data-testid="pool-rule">
                <span className="min-w-0 flex-1 truncate text-ink">{ACTION_WORDS[rule.action] ?? rule.action}</span>
                <span className="shrink-0 text-label text-faint">you, {agoWords(rule.created_at, now)}</span>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void run(() => agentRuleRevoke(rule.id))}
                  aria-label={`Take back: ${ACTION_WORDS[rule.action] ?? rule.action}`}
                  className="shrink-0 rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                  data-testid="pool-rule-revoke"
                >
                  Take back
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {error && (
        <p className="text-xs text-danger" role="alert" data-testid="pool-access-error">
          {error}
        </p>
      )}
    </div>
  );
}
