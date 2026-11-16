/**
 * The inspector on the pool itself (D-28..D-33): what its agents may do,
 * the rules you made (each taken back with a click), how messages go into
 * sessions and when a chain pauses, who tells you, your other Macs, and
 * the pools its agents may write to. Every change applies on the agents'
 * next request. AJ-E3011 round 3 (#2): after the summary, each of these is
 * a fold, closed at first, with one plain line on what it controls ("the
 * wall" the walk met), in plain words (no bare "Deliver" or "Pause after").
 * Round 4 (#6): the rarer ones (kept apart, your other Macs, other pools
 * its agents may write to) under one closed "More settings".
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";

import { errorText } from "../../lib/bridge";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { poolDelete, poolSetSpansMacs, type PoolInfo, type PoolsInfo } from "../../lib/ipc/pools";
import { notificationsMute, notificationsMutes, notificationsUnmute, onNoticesChanged } from "../../lib/ipc/notices";
import { PoolUsage } from "./PoolUsage";
import { SealSetting } from "./SealSetting";
import { setPools } from "../../lib/pools";
import { PoolAccessFolds } from "./PoolAccessFolds";
import { PoolLinks } from "../messages/PoolLinks";
import { ChoiceOption } from "../../components/controls";
import { FoldSection, Section } from "./Section";

/** Whether you muted this project's (or, D-62, this pool's)
 * notifications. */
function useMuted(scope: "project" | "pool", value: string | null): boolean | null {
  const [muted, setMuted] = useState<boolean | null>(null);
  useEffect(() => {
    if (!value) return;
    let alive = true;
    const read = () =>
      void notificationsMutes({ cursor: null, limit: 200 }).then(
        (page) => alive && setMuted(page.items.some((mute) => mute.scope === scope && mute.value === value)),
        () => undefined,
      );
    read();
    const off = onNoticesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [scope, value]);
  return muted;
}

export function InspectPool({
  pool,
  project,
  now,
  members = [],
  onDeleted,
}: {
  pool: PoolInfo;
  project: ProjectInfo | undefined;
  now: number;
  /** Its members by name (D-62: their usage this week, a view only). */
  members?: { id: string; name: string }[];
  onDeleted: () => void;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  // A project's own pool is its project; your pool is muted as itself.
  const own = pool.kind === "default";
  const muted = useMuted(own ? "project" : "pool", own ? pool.project_id : pool.id);
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
  return (
    <div className="flex flex-col" data-testid="inspect-pool">
      {/* What agents may do here, as Settings › Agent access shows it (one component, round 3). */}
      <PoolAccessFolds pool={pool} now={now} busy={busy} run={(work) => void run(work)} />

      <FoldSection title="Used this week" line="What its sessions used this week, from their tools' own counts.">
        <PoolUsage members={members} project={pool.scope_all || pool.scope.length !== 1 ? null : (pool.scope[0] ?? null)} />
      </FoldSection>

      <FoldSection title="Notifications" line="Whether what waits for you here also interrupts you." testId="pool-notify">
        {!own || project ? (
          <ChoiceOption
            type="checkbox"
            className="-mx-2"
            label={`Notify me about ${own && project ? project.name : pool.name}`}
            hint="Asks and paused chains always wait in Needs you; this is whether they interrupt."
            checked={muted === false}
            disabled={busy || muted === null}
            reason={muted === null ? "Reading your notification settings…" : undefined}
            onChange={(event) => {
              const on = event.target.checked;
              const [scope, value, label] = own && project ? (["project", project.id, project.name] as const) : (["pool", pool.id, pool.name] as const);
              void run(() => (on ? notificationsUnmute(scope, value) : notificationsMute(scope, value, label)));
            }}
            data-testid={own ? "pool-notify-project" : "pool-notify-pool"}
          />
        ) : (
          <p className="text-label text-muted">Asks and paused chains wait in Needs you and notify you as your notification settings say.</p>
        )}
      </FoldSection>

      {/* Round 4 (#6): the rarer settings, under one fold closed at first. */}
      <FoldSection title="More settings" line="Keep this pool apart, let its work reach your other Macs, or let its agents write to other pools." testId="pool-more">
        {pool.kind === "user" && (
          <MoreGroup title="Kept apart" testId="pool-seal-section">
            <SealSetting pool={pool} members={members} />
          </MoreGroup>
        )}
        <MoreGroup title="Your other Macs">
          <ChoiceOption
            type="checkbox"
            className="-mx-2"
            label="Include my paired Macs"
            hint="Message and task content will travel to your other Macs, directly and encrypted."
            checked={pool.spans_macs}
            disabled={busy}
            onChange={(event) => void run(() => poolSetSpansMacs(pool.id, event.target.checked))}
            data-testid="pool-spans-macs"
          />
        </MoreGroup>
        <MoreGroup title="Pools its agents may write to">
          <PoolLinks poolId={pool.id} />
        </MoreGroup>
      </FoldSection>

      {pool.kind === "user" && (
        <Section title="This pool">
          {confirmDelete ? (
            <div className="flex flex-col gap-2 rounded-lg border border-danger/40 bg-danger-soft p-2.5" role="group" aria-label="Delete this pool?">
              <span className="text-meta text-ink">Delete «{pool.name}»? Its sessions go back to their projects' own pools; its messages and tasks stay in its history.</span>
              <span className="flex gap-2">
                <button type="button" onClick={() => void run(async () => {
                  // The pools after it go to the list, so it leaves the screen (the P11 walk).
                  const after = await poolDelete(pool.id);
                  onDeleted();
                  return after;
                })} className="rounded-lg bg-danger px-2.5 py-1 text-meta font-medium text-white outline-none focus-visible:ring-2 focus-visible:ring-danger/60" data-testid="pool-delete-confirm">
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

/** One of More settings' groups: its title over its control. */
function MoreGroup({ title, children, testId }: { title: string; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" data-testid={testId}>
      <h5 className="text-meta font-semibold text-ink">{title}</h5>
      {children}
    </div>
  );
}
