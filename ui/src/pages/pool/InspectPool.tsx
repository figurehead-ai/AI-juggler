/**
 * The inspector on the pool itself (D-28..D-33): what its agents may do,
 * the rules you made (each taken back with a click), how messages go into
 * sessions and when a chain pauses, who tells you, your other Macs, and
 * the pools its agents may write to. Every change applies on the agents'
 * next request.
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";

import { errorText, type ProjectInfo } from "../../lib/bridge";
import { AGENT_ACTIONS, agentRuleRevoke, poolDelete, poolSetAccess, poolSetDelivery, poolSetSpansMacs, type PoolInfo, type PoolsInfo } from "../../lib/ipc/pools";
import { notificationsMute, notificationsMutes, notificationsUnmute, onNoticesChanged } from "../../lib/ipc/notices";
import { ACTION_WORDS, LEVELS, setPools } from "../../lib/pools";
import { agoWords } from "../../lib/time-words";
import { PoolLinks } from "../messages/PoolLinks";
import { Picker, Stepper, Toggle } from "./controls";

export function Section({ title, children, testId }: { title: string; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <section className="flex flex-col gap-2.5 border-t border-hairline/70 px-4 py-3.5 first:border-t-0" data-testid={testId}>
      <h4 className="text-label font-semibold uppercase tracking-[0.08em] text-faint">{title}</h4>
      {children}
    </section>
  );
}

/** Whether you muted this project's notifications. */
function useProjectMuted(projectId: string | null): boolean | null {
  const [muted, setMuted] = useState<boolean | null>(null);
  useEffect(() => {
    if (!projectId) return;
    let alive = true;
    const read = () =>
      void notificationsMutes({ cursor: null, limit: 200 }).then(
        (page) => alive && setMuted(page.items.some((mute) => mute.scope === "project" && mute.value === projectId)),
        () => undefined,
      );
    read();
    const off = onNoticesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [projectId]);
  return muted;
}

/** How a message gets into a session here, in words (D-29). */
export function deliveryWords(pool: PoolInfo): string {
  if (pool.access.delivery_stopped) return "Stopped: nothing goes into a session";
  const own = pool.access.level === "full" || pool.rules.some((rule) => rule.action === "deliver_to_session");
  return own ? "By themselves, between turns" : "When you click Deliver";
}

export function InspectPool({ pool, project, now, onDeleted }: { pool: PoolInfo; project: ProjectInfo | undefined; now: number; onDeleted: () => void }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const muted = useProjectMuted(pool.project_id);
  const run = async (work: () => Promise<PoolsInfo | unknown>) => {
    setBusy(true);
    setError(null);
    try {
      const answer = await work();
      if (answer && typeof answer === "object" && "pools" in answer) setPools(answer as PoolsInfo);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const allowed = new Set(pool.access.actions);
  return (
    <div className="flex flex-col" data-testid="inspect-pool">
      <Section title="What agents may do" testId="inspect-pool-level">
        <Picker
          label="What agents may do"
          heading="Agents in this pool may"
          value={pool.access.level}
          choices={LEVELS.map((level) => ({ id: level.id, label: level.label, words: level.words }))}
          disabled={busy}
          onPick={(level) => void run(() => poolSetAccess(pool.id, level, level === "custom" ? pool.access.actions : []))}
          testId="pool-level"
        />
        <p className="text-label text-muted">{LEVELS.find((level) => level.id === pool.access.level)?.words}</p>
        {pool.access.level === "custom" && (
          <div className="flex flex-col gap-2" data-testid="pool-custom">
            {AGENT_ACTIONS.filter((action) => action !== "deliver_to_session").map((action) => (
              <Toggle
                key={action}
                label={ACTION_WORDS[action] ?? action}
                on={allowed.has(action)}
                disabled={busy}
                onChange={(on) => void run(() => poolSetAccess(pool.id, "custom", on ? [...allowed, action] : [...allowed].filter((word) => word !== action)))}
              />
            ))}
          </div>
        )}
      </Section>

      <Section title="Always allowed here" testId="pool-rules">
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
                  onClick={() => void run(() => agentRuleRevoke(rule.id))}
                  className="shrink-0 rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                  data-testid="pool-rule-revoke"
                >
                  Take back<span className="sr-only">: {ACTION_WORDS[rule.action] ?? rule.action}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </Section>

      <Section title="Messages into sessions" testId="pool-delivery">
        <div className="flex items-baseline justify-between gap-3">
          <span className="text-body text-muted">Delivered</span>
          <span className="text-right text-body text-ink" data-testid="pool-delivery-words">
            {deliveryWords(pool)}
          </span>
        </div>
        <div className="flex items-center justify-between gap-3">
          <span className="flex flex-col">
            <span className="text-body text-ink">Pause and ask after</span>
            <span className="text-label text-muted">agent messages in a row without you</span>
          </span>
          <Stepper label="Agent messages in a row before a chain pauses" value={pool.access.hop_limit} min={1} max={20} disabled={busy} onChange={(hops) => void run(() => poolSetDelivery(pool.id, hops, pool.access.delivery_stopped))} testId="pool-hop-limit" />
        </div>
        <Toggle label="Stop all of them" hint="Messages still reach the inboxes; none goes into a session." on={pool.access.delivery_stopped} disabled={busy} onChange={(on) => void run(() => poolSetDelivery(pool.id, pool.access.hop_limit, on))} testId="pool-stop-all" />
      </Section>

      <Section title="Notifications" testId="pool-notify">
        {project ? (
          <Toggle
            label={`Notify me about ${project.name}`}
            hint="Asks and paused chains always wait in Needs you; this is whether they interrupt."
            on={muted === false}
            disabled={busy || muted === null}
            onChange={(on) => void run(() => (on ? notificationsUnmute("project", project.id) : notificationsMute("project", project.id, project.name)))}
            testId="pool-notify-project"
          />
        ) : (
          <p className="text-label text-muted">Asks and paused chains wait in Needs you and notify you as your notification settings say.</p>
        )}
      </Section>

      <Section title="Your other Macs">
        <Toggle label="Include my paired Macs" hint="Message and task content will travel to your other Macs, directly and encrypted." on={pool.spans_macs} disabled={busy} onChange={(on) => void run(() => poolSetSpansMacs(pool.id, on))} testId="pool-spans-macs" />
      </Section>

      <Section title="Pools its agents may write to">
        <PoolLinks poolId={pool.id} />
      </Section>

      {pool.kind === "user" && (
        <Section title="This pool">
          {confirmDelete ? (
            <div className="flex flex-col gap-2 rounded-lg border border-danger/40 bg-danger-soft p-2.5" role="group" aria-label="Delete this pool?">
              <span className="text-meta text-ink">Delete «{pool.name}»? Its sessions go back to their projects' own pools; its messages and tasks stay in its history.</span>
              <span className="flex gap-2">
                <button type="button" onClick={() => void run(async () => (await poolDelete(pool.id), onDeleted()))} className="rounded-lg bg-danger px-2.5 py-1 text-meta font-medium text-white outline-none focus-visible:ring-2 focus-visible:ring-danger/60" data-testid="pool-delete-confirm">
                  Delete pool
                </button>
                <button type="button" onClick={() => setConfirmDelete(false)} className="rounded-lg px-2.5 py-1 text-meta text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
                  Keep it
                </button>
              </span>
            </div>
          ) : (
            <button type="button" onClick={() => setConfirmDelete(true)} className="self-start rounded-lg px-1 text-meta text-danger outline-none hover:underline focus-visible:ring-1 focus-visible:ring-danger/60" data-testid="pool-delete">
              Delete pool…
            </button>
          )}
        </Section>
      )}

      {error && (
        <p className="px-4 pb-3 text-label text-danger" role="alert" data-testid="pool-access-error">
          {error}
        </p>
      )}
    </div>
  );
}
