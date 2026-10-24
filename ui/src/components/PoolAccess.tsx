import { useId, useState, type JSX } from "react";

import { errorText } from "../lib/bridge";
import { AGENT_ACTIONS, agentRuleRevoke, poolSetAccess, poolSetDelivery, type PoolInfo, type PoolsInfo } from "../lib/ipc/pools";
import { ACTION_WORDS, LEVELS, levelLabel, setPools, usePools } from "../lib/pools";
import { agoWords } from "../lib/time-words";
import { Card } from "./primitives";

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
            <input
              type="radio"
              name={name}
              checked={level === entry.id}
              onChange={() => void run(() => poolSetAccess(pool.id, entry.id, entry.id === "custom" ? pool.access.actions : []))}
              className="mt-0.5 size-3.5 accent-accent"
            />
            <span className="flex flex-col">
              <span className="font-medium text-ink">{entry.label}</span>
              <span className="text-[11px] text-muted">{entry.words}</span>
            </span>
          </label>
        ))}
      </fieldset>

      {level === "custom" && (
        <fieldset className="ml-6 grid grid-cols-1 gap-0.5 sm:grid-cols-2" disabled={busy} data-testid="pool-custom">
          <legend className="sr-only">Actions</legend>
          {AGENT_ACTIONS.filter((action) => action !== "deliver_to_session").map((action) => (
            <label key={action} className="flex items-center gap-2 text-[11.5px] text-ink">
              <input
                type="checkbox"
                checked={allowed.has(action)}
                onChange={(event) => {
                  const next = event.target.checked ? [...allowed, action] : [...allowed].filter((word) => word !== action);
                  void run(() => poolSetAccess(pool.id, "custom", next));
                }}
                className="size-3.5 accent-accent"
              />
              {ACTION_WORDS[action]}
            </label>
          ))}
        </fieldset>
      )}

      <details className="rounded-lg border border-hairline/60 px-2.5 py-1.5" data-testid="pool-delivery">
        <summary className="cursor-pointer select-none text-[12px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
          Messages put into sessions
        </summary>
        <div className="mt-2 flex flex-col gap-2 text-xs">
          <label className="flex items-center gap-2 text-ink">
            Pause and ask after
            <select
              value={pool.access.hop_limit}
              disabled={busy}
              onChange={(event) => void run(() => poolSetDelivery(pool.id, Number(event.target.value), pool.access.delivery_stopped))}
              className="h-7 rounded-lg border border-hairline bg-surface-2 px-1.5 text-[11.5px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
              data-testid="pool-hop-limit"
            >
              {Array.from({ length: 20 }, (_, index) => index + 1).map((hops) => (
                <option key={hops} value={hops}>
                  {hops}
                </option>
              ))}
            </select>
            replies in a row
          </label>
          <label className="flex items-center gap-2 text-ink">
            <input
              type="checkbox"
              checked={pool.access.delivery_stopped}
              disabled={busy}
              onChange={(event) => void run(() => poolSetDelivery(pool.id, pool.access.hop_limit, event.target.checked))}
              className="size-3.5 accent-accent"
              data-testid="pool-stop-all"
            />
            Stop all of them in this pool
          </label>
        </div>
      </details>

      {pool.rules.length > 0 && (
        <section aria-label="Always allowed" className="flex flex-col gap-1" data-testid="pool-rules">
          <p className="text-[11px] uppercase tracking-[0.14em] text-faint">Always allowed here</p>
          <ul className="flex flex-col gap-1">
            {pool.rules.map((rule) => (
              <li key={rule.id} className="flex items-center gap-2 rounded-lg border border-hairline/60 px-2.5 py-1.5 text-xs" data-testid="pool-rule">
                <span className="min-w-0 flex-1 truncate text-ink">{ACTION_WORDS[rule.action] ?? rule.action}</span>
                <span className="shrink-0 text-[11px] text-faint">you, {agoWords(rule.created_at, now)}</span>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void run(() => agentRuleRevoke(rule.id))}
                  aria-label={`Take back: ${ACTION_WORDS[rule.action] ?? rule.action}`}
                  className="shrink-0 rounded-md px-1.5 py-0.5 text-[11px] text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
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

/**
 * Settings › Agent access: every pool's level and every rule, on one
 * page. A pool's row opens its details.
 */
export function AgentAccessCard(): JSX.Element {
  const pools = usePools();
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-agent-access">
      <div>
        <h3 className="text-sm font-semibold">Agent access</h3>
        <p className="mt-1 text-xs text-muted">
          What agents in each pool may do. Only you change it, here or on the pool's page, and a change counts from the next request. Anything more becomes a question for you in Needs you.
        </p>
      </div>
      {pools === null ? (
        <p className="text-xs text-muted">Loading…</p>
      ) : (
        <ul className="flex flex-col gap-1.5" data-testid="agent-access-pools">
          {pools.pools.map((pool) => (
            <li key={pool.id}>
              <details className="group rounded-lg border border-hairline/60 px-3 py-2" data-testid="agent-access-pool">
                <summary className="flex cursor-pointer select-none items-center gap-2 text-xs outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
                  <span aria-hidden="true" className="text-faint transition-transform group-open:rotate-90">›</span>
                  <span className="min-w-0 flex-1 truncate font-medium text-ink">{pool.name}</span>
                  <span className="shrink-0 text-muted">{levelLabel(pool.access.level)}</span>
                  {pool.rules.length > 0 && (
                    <span className="shrink-0 text-faint">
                      · {pool.rules.length} {pool.rules.length === 1 ? "rule" : "rules"}
                    </span>
                  )}
                </summary>
                <div className="mt-2.5">
                  <PoolAccessPanel pool={pool} />
                </div>
              </details>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
