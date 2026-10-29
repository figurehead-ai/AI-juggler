/**
 * Continue on another account, for a session AI Juggler runs in its
 * terminal: a card in its page's side column. D-25: the ranked switch list
 * is one step away: folded, with one line of what it holds, opened by
 * "Switch account…" (the header) or ⌘⇧H, and open by itself when the
 * session hit its limit or a way back is on offer. AJ-E209: the ranked,
 * badged destinations; AJ-E706: paired Macs as destinations and, for a
 * session that came from another Mac, the way back (AJ-E708: with any
 * divergence that waits for you). Suggest-only: every move is a click.
 * 0.3: Save handover… under them, ending with Reveal in Finder.
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { createPortal } from "react-dom";
import { Card } from "./primitives";
import { SwitchTargetRows } from "./SwitchTargetRows";
import { RemoteSwitchTargets } from "./RemoteSwitchTargets";
import { CrossMachineSwitchFlow } from "./CrossMachineSwitchFlow";
import { CrossMachineSwitchBack } from "./CrossMachineSwitchBack";
import { SaveHandover } from "./HandoverView";
import {
  fleetDivergences,
  fleetSwitchBackPlan,
  fleetSwitchTargets,
  switchTargets,
  type DivergenceRecordInfo,
  type FleetSwitchBackPlanInfo,
  type InstanceInfo,
  type RemoteSwitchTargetInfo,
  type SwitchBackSuggestionData,
  type SwitchTargetData,
} from "../lib/bridge";
import { rankTargets, type TargetRoom } from "../lib/room-targets";
import { useHandoverFocus } from "../lib/handover-focus";
import { useSubscriptions } from "../lib/subscriptions";
import { useRunways } from "../pages/session/data";

/** What the folded list holds, in one line: the Suggested one (most
 * room, `lib/room`) with its room, or that none has much room. */
export function handoverTeaser(targets: Array<SwitchTargetData & { room?: TargetRoom }> | null): string {
  if (targets === null) return "Ranking the other accounts…";
  const open = targets.filter((target) => target.filtered_reason == null);
  if (open.length === 0) return targets.length === 0 ? "Add another account to juggle between them." : "No other account can take it now.";
  const count = open.length === 1 ? "1 account can take it over" : `${open.length} accounts can take it over`;
  const best = open.find((target) => target.suggested);
  if (!best) return `${count}, but none has much room left now.`;
  const why = best.room?.left != null ? `${best.room.left}% left` : best.top_factors[0];
  return `${count}. Suggested: ${best.account_label}${why ? `, ${why}` : ""}.`;
}

export function HandoverSection({
  session,
  switchBackSuggestions = [],
  onSwitchBack,
  onHandover,
}: {
  session: InstanceInfo;
  switchBackSuggestions?: SwitchBackSuggestionData[];
  onSwitchBack?: (sessionId: string) => void;
  onHandover: (sessionId: string, targetProfileId: string, targetLabel: string) => void;
}): JSX.Element {
  const handoverRef = useRef<HTMLDetailsElement | null>(null);
  const [switchOpen, setSwitchOpen] = useState(() => session.status === "quota_blocked");
  useHandoverFocus(() => {
    setSwitchOpen(true);
    // jsdom has no scrollIntoView; the guard keeps tests honest.
    requestAnimationFrame(() => {
      handoverRef.current?.scrollIntoView?.({ block: "center" });
      // ⌘⇧H "focuses the hand-over section": its summary takes focus.
      handoverRef.current?.querySelector<HTMLElement>("summary")?.focus();
    });
  });
  // Refetched when the session or its live state changes: the ranker's
  // inputs (quota, sessions) move with them.
  const [rows, setRows] = useState<SwitchTargetData[] | null>(null);
  const instanceId = session.id;
  // The one order by room, as Home's Runway reads it (the 0.2 walk, F1).
  const accounts = useSubscriptions();
  const displays = useRunways();
  const targets = useMemo(
    () => (rows === null ? null : rankTargets(rows, accounts, displays, session.account_id)),
    [rows, accounts, displays, session.account_id],
  );
  useEffect(() => {
    let alive = true;
    switchTargets(instanceId)
      .then((next) => alive && setRows(next))
      .catch(() => alive && setRows([]));
    return () => {
      alive = false;
    };
  }, [instanceId, session.status]);
  const [remoteTargets, setRemoteTargets] = useState<RemoteSwitchTargetInfo[]>([]);
  const [backPlan, setBackPlan] = useState<FleetSwitchBackPlanInfo | null>(null);
  const [divergence, setDivergence] = useState<DivergenceRecordInfo | null>(null);
  const [crossTarget, setCrossTarget] = useState<RemoteSwitchTargetInfo | null>(null);
  const [backOpen, setBackOpen] = useState(false);
  const [crossTick, setCrossTick] = useState(0);
  useEffect(() => {
    let alive = true;
    fleetSwitchTargets(instanceId)
      .then((rows) => alive && setRemoteTargets(rows))
      .catch(() => alive && setRemoteTargets([]));
    fleetSwitchBackPlan(instanceId)
      .then((plan) => alive && setBackPlan(plan))
      .catch(() => alive && setBackPlan(null));
    fleetDivergences()
      .then((rows) => alive && setDivergence(rows.find((row) => row.session_id === instanceId) ?? null))
      .catch(() => alive && setDivergence(null));
    return () => {
      alive = false;
    };
  }, [instanceId, session.status, crossTick]);
  const suggestions = switchBackSuggestions.filter((suggestion) => suggestion.session_id === session.id);
  const open = switchOpen || Boolean(backPlan) || suggestions.length > 0;
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="handover-card">
      <details
        ref={handoverRef}
        open={open}
        onToggle={(event) => setSwitchOpen((event.currentTarget as HTMLDetailsElement).open)}
        className="group"
        data-testid="handover-section"
      >
        <summary className="-m-1 flex cursor-pointer list-none items-center justify-between gap-2 rounded p-1 text-section font-semibold text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden">
          Continue on another account
          <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" className="shrink-0 text-muted transition-transform group-open:rotate-90 motion-reduce:transition-none">
            <path d="M9 6l6 6-6 6" />
          </svg>
        </summary>
        <div className="mt-3 flex flex-col gap-2">
          {/* AJ-E207 Req.3: the reset-trigger suggestion — suggested,
              never executed; the user clicks. */}
          {suggestions.map((suggestion) => (
            <button
              key={suggestion.session_id}
              type="button"
              onClick={() => onSwitchBack?.(suggestion.session_id)}
              data-testid="switch-back-card"
              className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-2 text-left text-meta text-accent-hi transition-colors hover:border-accent/60"
            >
              <span className="block" data-testid="switch-back-card-copy">
                {suggestion.copy}
              </span>
              <span className="text-muted">Resume the original conversation ▸</span>
            </button>
          ))}
          {backPlan && (
            <button
              type="button"
              onClick={() => setBackOpen(true)}
              data-testid={divergence ? "cross-back-diverged-card" : "cross-back-card"}
              className={
                "rounded-lg border px-3 py-2 text-left text-meta transition-colors " +
                (divergence ? "border-warning/50 text-ink hover:border-warning" : "border-accent/40 bg-accent-soft text-accent-hi hover:border-accent/60")
              }
            >
              <span className="block">
                {divergence ? (
                  <>
                    <span aria-hidden="true">⚠ </span>Diverged — {divergence.turns_since} new {divergence.turns_since === 1 ? "turn" : "turns"} on{" "}
                    {divergence.machine_label} since the work left. Choose what to keep.
                  </>
                ) : (
                  <>
                    Came from {backPlan.original_machine_label} ({backPlan.original_account_label}) — {backPlan.new_turns} new{" "}
                    {backPlan.new_turns === 1 ? "turn" : "turns"} here.
                  </>
                )}
              </span>
              <span className="text-muted">{divergence ? "Review the conflict ▸" : `Switch back to ${backPlan.original_machine_label} ▸`}</span>
            </button>
          )}
          {targets === null ? (
            <p className="text-meta text-muted" data-testid="handover-targets-loading">
              Ranking destinations…
            </p>
          ) : targets.length === 0 ? (
            <p className="text-meta text-muted" data-testid="handover-targets-empty">
              Add another account to juggle between them.
            </p>
          ) : (
            <SwitchTargetRows targets={targets} onPick={(target) => onHandover(session.id, target.recipe_id, target.account_label)} />
          )}
          {remoteTargets.length > 0 && (
            <>
              <p className="mt-1 text-section font-semibold text-ink">Continue on another machine</p>
              <RemoteSwitchTargets targets={remoteTargets} onPick={setCrossTarget} />
            </>
          )}
        </div>
      </details>
      {!open && (
        <p className="text-meta leading-[1.5] text-muted" data-testid="handover-teaser">
          {handoverTeaser(targets)}
        </p>
      )}
      {/* 0.3: save its handover, beside the switch actions. */}
      <SaveHandover sessionId={session.id} />
      {/* Out to the body: the page's column is a size container, which
          would hold a fixed sheet inside it. */}
      {crossTarget &&
        createPortal(
          <CrossMachineSwitchFlow
            sessionId={session.id}
            target={crossTarget}
            onClose={() => {
              setCrossTarget(null);
              setCrossTick((tick) => tick + 1);
            }}
          />,
          document.body,
        )}
      {backOpen &&
        backPlan &&
        createPortal(
          <CrossMachineSwitchBack
            sessionId={session.id}
            plan={backPlan}
            divergence={divergence ? { turns_since: divergence.turns_since, last_activity_ms: divergence.last_activity_ms, summary: divergence.summary } : null}
            onClose={() => {
              setBackOpen(false);
              setCrossTick((tick) => tick + 1);
            }}
            onResolved={() => setCrossTick((tick) => tick + 1)}
          />,
          document.body,
        )}
    </Card>
  );
}
