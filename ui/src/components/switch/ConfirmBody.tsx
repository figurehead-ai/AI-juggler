/**
 * The switch's last screen (the one-click door, and the wizard's
 * Confirm step), simple by default (the maintainer, 2026-11-02: "Don't
 * overload the user … you should have info there and all the details,
 * but it should not be shown by default"):
 *
 * - "Ready to switch to Default" with the move's badge;
 * - where it continues: Desktop or CLI, always both (`WhereChoice`);
 * - what goes along in one coloured line that opens to the list, and what
 *   happens to the original in one;
 * - "Before you switch" only when something does need a decision: real
 *   risks (no room on the target), the keys question, the original's stop
 *   choice while it runs, what can be set up, each one line with its
 *   explanation folded;
 * - everything else behind "Details", closed every time the dialog opens.
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";
import { poolsSwitchLines, type SwitchPoolLineInfo } from "../../lib/ipc/pool-model";
import { ChoiceOption } from "../controls";
import { setSealOwed } from "./seal-confirm";

export { useSealOwed } from "./seal-confirm";
import type { PreflightInfoData } from "../../lib/bridge";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { moveBadge } from "../../lib/switch-words";
import { toolSpec } from "../../lib/tools";
import { originalWords, SwitchFreezeChoice, type FreezeChoice } from "../SwitchFreezeChoice";
import { SwitchSetupList } from "../SwitchSetupList";
import { FidelityBadge } from "../SwitchTargetRows";
import { finishedByYou } from "../../lib/useSwitchSetup";
import { CarrySummary } from "./CarrySummary";
import { DiskSpace } from "./DiskSpace";
import type { SwitchSpaceInfo } from "../../lib/ipc/switchSpace";
import { Disclosure } from "../controls";
import { risky, RoomNote } from "./TargetRoom";
import { WhereChoice, whereWords, type Destination } from "./WhereChoice";
import { creditsConsequence } from "../../lib/credits";
import type { TargetRoom } from "../../lib/room-targets";

/** What needs the user among the setup offers this conversation used (or
 * nothing says): something only they can do, or turn on in the account. */
export function needsYouCount(setup: SetupCheckInfo | null): number {
  return (setup?.offers ?? []).filter((offer) => finishedByYou(offer) && offer.used !== false).length;
}

export function ConfirmBody({
  preflight,
  target,
  room = null,
  freeze,
  onFreeze,
  destination,
  onDestination,
  setup,
  picked,
  onToggle,
  onCheckAgain,
  extra,
  pools,
  space = null,
  onSpaceAgain,
  carry,
  turnsLeftOut,
}: {
  preflight: PreflightInfoData;
  target: string;
  /** The target's room (the one runway); a warning when out or low. */
  room?: TargetRoom | null;
  freeze: FreezeChoice;
  onFreeze: (choice: FreezeChoice) => void;
  /** Where it continues: Desktop or CLI. */
  destination: Destination;
  onDestination: (destination: Destination) => void;
  setup: SetupCheckInfo | null;
  picked: ReadonlySet<string>;
  onToggle: (id: string) => void;
  /** Reads the target again; resolves to what it found for that offer,
   * in words. */
  onCheckAgain: (offerId: string) => Promise<string>;
  /** Anything else that needs an answer first (the keys question, asked
   * here when it applies: one screen). */
  extra?: ReactNode;
  /** D-62: what the switch does to each of its pools. */
  pools?: ReactNode;
  /** 0.3.9: the disk space the copy uses. */
  space?: SwitchSpaceInfo | null;
  onSpaceAgain?: () => void;
  /** D-76: leaving out the start of a very long conversation. */
  carry?: ReactNode;
  /** Turns the carry choice leaves out (D-76), for the summary's words. */
  turnsLeftOut?: number;
}): JSX.Element {
  const where = whereWords(destination, preflight);
  const outside = preflight.source_outside ? { app: preflight.source_app ?? null, target, where } : null;
  const needs = needsYouCount(setup);
  const offers = setup?.offers.length ?? 0;
  const stoppable = preflight.source_live && !preflight.source_keeps_running && !outside;
  const tight = risky(room);
  const before = preflight.warnings.length > 0 || stoppable || offers > 0 || extra !== undefined || tight;
  const tool = toolSpec(preflight.target_tool).display_name;
  return (
    <div className="flex flex-col gap-5" data-testid="switch-step-confirm">
      <div className="flex flex-col gap-1.5">
        <h3 className="text-title text-ink">Ready to switch to {target}</h3>
        {/* 0.4: what moves, named before anything does (0.3.9 moved a
            conversation the user didn't pick). */}
        <p className="text-body text-ink" data-testid="switch-source">
          Moves <span className="font-semibold">{preflight.source_title ? `“${preflight.source_title}”` : "this conversation"}</span>
          {preflight.source_account ? <span className="text-muted"> from {preflight.source_account}</span> : null}
          {preflight.source_family ? <span>, with the {preflight.source_family === 1 ? "session" : `${preflight.source_family} sessions`} started from it</span> : null}
        </p>
        <p className="text-meta" data-testid="switch-cost">
          <FidelityBadge badge={moveBadge(preflight)} validated={preflight.validated === true} />
        </p>
      </div>

      <WhereChoice preflight={preflight} value={destination} onChange={onDestination} />

      <div className="flex flex-col gap-2" data-testid="switch-summary">
        <CarrySummary items={preflight.carry_items ?? []} target={target} tool={tool} setup={setup} picked={picked} turnsLeftOut={turnsLeftOut} />
        <DiskSpace space={space} onCheckAgain={onSpaceAgain} />
        {carry}
        <p className="text-body text-ink-2" data-testid={outside ? "switch-outside" : preflight.source_keeps_running ? "switch-keeps-running" : "switch-original"}>
          {originalWords({ live: preflight.source_live, keepsRunning: preflight.source_keeps_running, outside, choice: freeze })}
        </p>
        {room && (tight || room.kind === "credits") && (
          <p data-testid="switch-summary-room" className={"text-body " + (tight ? "font-semibold text-warning" : "text-ink-2")}>
            {tight
              ? `${room.account ?? target} ${room.kind === "out" ? "is out of room" : "has little room left"}: see below before you switch.`
              : `Room on ${room.account ?? target}: ${room.words}. ${creditsConsequence(room.resetsAtMs, Date.now(), "It goes on")}`}
          </p>
        )}
        {needs > 0 && (
          <p className="text-body text-ink-2" data-testid="switch-summary-needs">
            {needs === 1 ? "1 thing needs you below; it doesn't stop the switch." : `${needs} things need you below; none of them stops the switch.`}
          </p>
        )}
      </div>

      {pools}

      {before && (
        <section className="flex flex-col gap-4 border-t border-hairline pt-4" aria-labelledby="switch-before" data-testid="switch-before">
          <h4 id="switch-before" className="text-section text-ink">
            Before you switch
          </h4>
          {tight && <RoomNote room={room} target={target} />}
          {preflight.warnings.length > 0 && (
            <ul className="flex flex-col gap-1.5" data-testid="switch-warnings">
              {preflight.warnings.map((warning) => (
                <li key={warning} className="rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-body text-ink">
                  {warning}
                </li>
              ))}
            </ul>
          )}
          {extra}
          {stoppable && <SwitchFreezeChoice sourceLive value={freeze} onChange={onFreeze} />}
          {setup && offers > 0 && <SwitchSetupList check={setup} picked={picked} onToggle={onToggle} onCheckAgain={onCheckAgain} titled={false} />}
        </section>
      )}

      {/* Closed every time (the maintainer opened it once and it stayed
          open, burying the screen): what's fine, for who wants it. */}
      <Disclosure tone="title" className="border-t border-hairline pt-3" testId="switch-details" summary="Details">
        <div className="mt-3 flex flex-col gap-2 text-body text-ink-2">
          <p data-testid="switch-details-move">{preflight.move_line}</p>
          <p data-testid="switch-oneclick-carry">{preflight.experimental_copy}</p>
          {room && !tight && room.kind !== "credits" && (
            <p data-testid="switch-details-room">
              Room on {target}: {room.words}
              {room.resets ? ` · ${room.resets}` : ""}.
            </p>
          )}
          {setup && setup.already_there > 0 && (
            <p>
              {target} already has {setup.already_there} of the {setup.already_there + offers} things this app has set up
              {offers > 0 ? `; the other ${offers} are listed under Before you switch.` : "."}
            </p>
          )}
        </div>
      </Disclosure>
    </div>
  );
}

/** D-62 (AJ-E3009): one line per pool: it stays with full tools or its
 * shell only, leaves (and why), or joins; a sealed pool entered or left
 * is said in amber. Nothing when it's in no pool. */
export function SwitchPoolLines({ sessionId, targetProfileId }: { sessionId: string; targetProfileId: string }): JSX.Element | null {
  const [lines, setLines] = useState<SwitchPoolLineInfo[]>([]);
  const [ticked, setTicked] = useState(false);
  useEffect(() => {
    let alive = true;
    setTicked(false);
    void poolsSwitchLines(sessionId, targetProfileId).then(
      (next) => alive && setLines(next),
      () => alive && setLines([]),
    );
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId]);
  const sealed = lines.some((line) => line.confirm);
  // The switch button waits for the tick while a sealed pool is at stake.
  useEffect(() => {
    setSealOwed(sessionId, targetProfileId, sealed && !ticked);
    return () => setSealOwed(sessionId, targetProfileId, false);
  }, [sessionId, targetProfileId, sealed, ticked]);
  if (lines.length === 0) return null;
  return (
    <section className="flex flex-col gap-1.5" aria-label="Its pools" data-testid="switch-pool-lines">
      <h4 className="text-label font-semibold uppercase tracking-wide text-muted">Its pools</h4>
      <ul className="flex flex-col gap-1">
        {lines.map((line) => (
          <li key={line.pool_id} className={"text-body " + (line.confirm ? "rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-ink" : "text-ink-2")} data-kind={line.kind}>
            {line.words}
          </li>
        ))}
      </ul>
      {sealed && (
        <ChoiceOption
          type="checkbox"
          label="Switch anyway"
          hint="A sealed pool keeps its sessions out of every other pool. The switch waits for this tick."
          checked={ticked}
          onChange={(event) => setTicked(event.target.checked)}
          data-testid="switch-seal-confirm"
        />
      )}
    </section>
  );
}
