/**
 * AJ-E1702, D-28 — Settings › Agent access: what agents may do in each
 * pool (its level, picked by you here or on the pool's page, with its
 * details behind the row) and every rule you made, each taken back with
 * a click.
 *
 * Always in the sidebar on 0.2: every project has its default pool
 * (D-33), whose level applies to that project's sessions too, so what
 * agents may do is within reach whether or not you've made a pool.
 *
 * AJ-E1704, D-30: last, whether the sessions AI Juggler starts get its
 * agent tools at all (the one-time question until it's answered), with
 * the switches per project and profile behind a fold.
 */
import { useState, type JSX } from "react";
import { errorText } from "../../lib/bridge";
import { agentRuleRevoke, type PoolInfo, type PoolsInfo } from "../../lib/ipc/pools";
import { ACTION_WORDS, levelLabel, setPools, usePools } from "../../lib/pools";
import { agoWords } from "../../lib/time-words";
import { CardTitle } from "../../components/CardTitle";
import { PoolAccessPanel } from "../../components/PoolAccess";
import { AgentToolsSettings } from "../../components/AgentTools";
import { Button, Card } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { Pane, PaneSection } from "./parts";

export function AgentAccessPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="agent-access" title="Agent access" anchor={anchor}>
      <PaneSection
        id="agent-pools"
        title="Pools"
        footer="Only you change what agents may do, here or on a pool's page, and a change counts from the next request. Anything more becomes a question for you in Needs you."
        card
      >
        <AgentAccessCard />
      </PaneSection>
      <PaneSection
        id="agent-rules"
        title="Rules you made"
        footer="A rule lets the agents in one pool do one thing without asking. Taking it back counts from the very next request."
        card
      >
        <AgentRules />
      </PaneSection>
      <PaneSection
        id="agent-tools"
        title="Agent tools in sessions"
        footer="Added to each launch only, never written into your setup. Sessions started outside AI Juggler get them once you continue them here."
        card
      >
        <AgentToolsSettings />
      </PaneSection>
    </Pane>
  );
}

/** Pools shown before "Show more" (a project's own pool each: the list
 * grows with the projects). */
export const POOLS_SHOWN = 12;

/** Every pool's level, on one card (the 0.2 walk, F10): a row's name opens
 * the pool's page (its members, tasks, messages and access); its level
 * unfolds in place to change it. Your own pools first, then each
 * project's, a page at a time. */
export function AgentAccessCard(): JSX.Element {
  const pools = usePools();
  const host = useActionHost();
  const [shown, setShown] = useState(POOLS_SHOWN);
  const list = pools?.pools ?? [];
  const more = Math.max(0, list.length - shown);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-agent-access">
      <CardTitle>Agent access</CardTitle>
      {pools === null ? (
        <p className="text-meta text-muted">Loading…</p>
      ) : (
        <>
          <ul className="flex flex-col gap-1.5" data-testid="agent-access-pools">
            {list.slice(0, shown).map((pool) => (
              <PoolRow key={pool.id} pool={pool} onOpen={host.openPool ? () => host.openPool!(pool.id) : undefined} />
            ))}
          </ul>
          {more > 0 && (
            <Button size="sm" variant="secondary" className="w-fit" onClick={() => setShown((count) => count + POOLS_SHOWN * 2)} data-testid="agent-access-more">
              Show {Math.min(more, POOLS_SHOWN * 2)} more of {more}
            </Button>
          )}
        </>
      )}
    </Card>
  );
}

/** One pool: its name (to its page), its level and rules, and the level
 * unfolded in place. */
function PoolRow({ pool, onOpen }: { pool: PoolInfo; onOpen?: () => void }): JSX.Element {
  const [open, setOpen] = useState(false);
  const panel = `agent-access-panel-${pool.id}`;
  const rules = pool.rules.length > 0 ? ` · ${pool.rules.length} ${pool.rules.length === 1 ? "rule" : "rules"}` : "";
  return (
    <li className="rounded-lg border border-hairline/60 px-3 py-2" data-testid="agent-access-pool">
      <div className="flex items-center gap-2 text-body">
        {onOpen ? (
          <button
            type="button"
            onClick={onOpen}
            className="min-w-0 flex-1 truncate rounded-md text-left font-medium text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
            title="Open its page: members, tasks, messages and access"
            data-testid="agent-access-open"
          >
            {pool.name}
            <span className="sr-only">: open its page</span>
          </button>
        ) : (
          <span className="min-w-0 flex-1 truncate font-medium text-ink">{pool.name}</span>
        )}
        <button
          type="button"
          aria-expanded={open}
          aria-controls={panel}
          onClick={() => setOpen((now) => !now)}
          className="flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-0.5 text-meta text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="agent-access-level"
        >
          <span>
            {levelLabel(pool.access.level)}
            {rules}
          </span>
          <span className="sr-only">: {open ? "hide" : "change"} what agents may do in {pool.name}</span>
          <span aria-hidden="true" className={`transition-transform ${open ? "rotate-90" : ""}`}>
            ›
          </span>
        </button>
      </div>
      {open && (
        <div id={panel} className="mt-2.5">
          <PoolAccessPanel pool={pool} />
        </div>
      )}
    </li>
  );
}

/** Every rule you made, in every pool, newest first. */
function AgentRules({ now = Date.now() }: { now?: number }): JSX.Element {
  const pools = usePools();
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
  const rules = (pools?.pools ?? []).flatMap((pool) => pool.rules.map((rule) => ({ rule, pool: pool.name }))).sort((a, b) => b.rule.created_at - a.rule.created_at);
  return (
    <Card className="flex flex-col gap-2 p-5" data-testid="settings-agent-rules">
      <CardTitle>Rules you made</CardTitle>
      {pools === null ? (
        <p className="text-xs text-muted">Loading…</p>
      ) : rules.length === 0 ? (
        <p className="text-xs text-muted">None yet. When you always allow something an agent asked, the rule shows here.</p>
      ) : (
        <ul className="flex flex-col gap-1">
          {rules.map(({ rule, pool }) => (
            <li key={rule.id} className="flex items-center gap-2 rounded-lg border border-hairline/60 px-2.5 py-1.5 text-xs" data-testid="agent-access-rule">
              <span className="min-w-0 flex-1 truncate text-ink">{ACTION_WORDS[rule.action] ?? rule.action}</span>
              <span className="shrink-0 truncate text-muted">{pool}</span>
              <span className="shrink-0 text-label text-faint">you, {agoWords(rule.created_at, now)}</span>
              <button
                type="button"
                disabled={busy}
                onClick={() => void run(() => agentRuleRevoke(rule.id))}
                aria-label={`Take back: ${ACTION_WORDS[rule.action] ?? rule.action} in ${pool}`}
                className="shrink-0 rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="agent-access-rule-revoke"
              >
                Take back
              </button>
            </li>
          ))}
        </ul>
      )}
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </Card>
  );
}
