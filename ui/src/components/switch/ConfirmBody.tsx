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
 * - 0.5.0 (D-142): rich mode. What stops it, or needs a look, as Alerts
 *   at their level; "What happens" as fact rows on the tinted panel; then
 *   only what needs a decision: the keys question, the original's stop
 *   choice while it runs, what can be set up, each with its explanation
 *   folded;
 * - everything else behind "Details", closed every time the dialog opens.
 */
import { useEffect, useState, type JSX, type ReactNode } from "react";
import { poolsSwitchLines, type SwitchPoolLineInfo } from "../../lib/ipc/pool-model";
import { CheckRow, RowList } from "../controls";
import { setSealOwed } from "./seal-confirm";

export { useSealOwed } from "./seal-confirm";
import type { PreflightInfoData } from "../../lib/ipc/switch-preflight";
import type { SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { moveBadge } from "../../lib/switch-words";
import { toolSpec } from "../../lib/tools";
import { originalWords, SwitchFreezeChoice, type FreezeChoice } from "../SwitchFreezeChoice";
import { SwitchSetupList } from "../SwitchSetupList";
import { finishedByYou } from "../../lib/useSwitchSetup";
import { CarrySummary } from "./CarrySummary";
import { DiskSpace } from "./DiskSpace";
import type { SwitchSpaceInfo } from "../../lib/ipc/switchSpace";
import { Disclosure } from "../controls";
import { risky, RoomNote } from "./TargetRoom";
import { Alert } from "../Alert";
import { Section } from "../Section";
import { IconGaugeLow, IconShield } from "../icons";
import { IconBell, IconCopy, IconPool } from "../icons-actions";
import { IconMessage } from "../icons-canvas";
import { IconHistory } from "../icons-places";
import { Fact, FactPanel } from "./Facts";
import { BadgeTag } from "./TargetCard";
import { WhereChoice, whereWords, type Destination } from "./WhereChoice";
import { creditsConsequence } from "../../lib/credits";
import { AutomationBefore } from "./AutomationLine";
import type { TargetRoom } from "../../lib/room-targets";

/** What needs the user among the setup offers this conversation used (or
 * nothing says): something only they can do, or turn on in the account. */
export function needsYouCount(setup: SetupCheckInfo | null): number {
  return (setup?.offers ?? []).filter((offer) => finishedByYou(offer) && offer.used !== false).length;
}

export function ConfirmBody({
  preflight,
  target,
  targetFolder,
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
  /** The target's folder (its desktop app is offered AI Juggler's
   * connector, AJ-E3013). */
  targetFolder?: string;
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
  const before = stoppable || offers > 0 || extra !== undefined;
  const tool = toolSpec(preflight.target_tool).display_name;
  const short = space?.kind === "short";
  return (
    <div className="flex flex-col gap-6" data-testid="switch-step-confirm">
      <div className="flex flex-col gap-1">
        <h3 className="type-section">Ready to switch to {target}</h3>
        <p className="type-help">Pick where it continues and check what happens. Nothing changes until you switch.</p>
      </div>

      <WhereChoice preflight={preflight} value={destination} onChange={onDestination} />

      {/* What stops it, or needs a look first, at its level (ALR-1). */}
      {tight && <RoomNote room={room} target={target} />}
      {short && <DiskSpace space={space} onCheckAgain={onSpaceAgain} />}
      {preflight.warnings.length > 0 && (
        <Alert level="info" live={null} title="Worth knowing" stack testId="switch-warnings">
          <ul className="mt-1 flex list-disc flex-col gap-0.5 pl-4">
            {preflight.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </Alert>
      )}

      <Section title="What happens" as="h4">
        <FactPanel testId="switch-summary" label="What happens">
          {/* 0.4: what goes, named before anything does (0.3.9 moved a
              conversation the user didn't pick). 0.4.1: a copy, never
              "moves" (deep/11 §4.9; Move to account is its own choice). */}
          <Fact icon={IconCopy} label="Copies">
            <span data-testid="switch-source">
              <span className="font-semibold">{preflight.source_title ? `“${preflight.source_title}”` : "This conversation"}</span>
              {preflight.source_account ? ` from ${preflight.source_account}` : null}
              {preflight.source_family ? `, with the ${preflight.source_family === 1 ? "session" : `${preflight.source_family} sessions`} started from it` : null}
            </span>
            <span className="contents" data-testid="switch-cost">
              <BadgeTag badge={moveBadge(preflight)} validated={preflight.validated === true} />
            </span>
          </Fact>
          {(preflight.carry_items ?? []).length > 0 && (
            <Fact icon={IconMessage} label="Carries">
              <CarrySummary items={preflight.carry_items ?? []} target={target} tool={tool} setup={setup} picked={picked} turnsLeftOut={turnsLeftOut} />
            </Fact>
          )}
          <Fact icon={IconHistory} label="Original">
            <span data-testid={outside ? "switch-outside" : preflight.source_keeps_running ? "switch-keeps-running" : "switch-original"}>
              {originalWords({ live: preflight.source_live, keepsRunning: preflight.source_keeps_running, outside, choice: freeze })}
            </span>
          </Fact>
          {!short && <DiskSpace space={space} onCheckAgain={onSpaceAgain} />}
          <AutomationBefore preflight={preflight} destination={destination} app={toolSpec(preflight.target_tool).desktop_app ?? "the desktop app"} />
          {room && (tight || room.kind === "credits") && (
            <Fact icon={IconGaugeLow} label="Room" tone={tight ? "warn" : "plain"}>
              <span data-testid="switch-summary-room" className={tight ? "font-semibold" : undefined}>
                {tight
                  ? `${room.account ?? target} ${room.kind === "out" ? "is out of room" : "has little room left"}: see above before you switch.`
                  : `Room on ${room.account ?? target}: ${room.words}. ${creditsConsequence(room.resetsAtMs, Date.now(), "It goes on")}`}
              </span>
            </Fact>
          )}
          {needs > 0 && (
            <Fact icon={IconBell} label="Needs you" tone="warn">
              <span data-testid="switch-summary-needs">
                {needs === 1 ? "1 thing needs you below; it doesn't stop the switch." : `${needs} things need you below; none of them stops the switch.`}
              </span>
            </Fact>
          )}
        </FactPanel>
        {carry}
      </Section>

      {pools}

      {before && (
        <div className="flex flex-col gap-6" data-testid="switch-before">
          {extra}
          {stoppable && <SwitchFreezeChoice sourceLive value={freeze} onChange={onFreeze} />}
          {setup && offers > 0 && (
            <Section title="Set up first" help={`What ${target} doesn't have yet`} as="h4">
              <SwitchSetupList check={setup} picked={picked} onToggle={onToggle} onCheckAgain={onCheckAgain} titled={false} toFolder={targetFolder} />
            </Section>
          )}
        </div>
      )}

      {/* Closed every time (the maintainer opened it once and it stayed
          open, burying the screen): what's fine, for who wants it. */}
      <Disclosure tone="title" testId="switch-details" summary="Details">
        <div className="mt-3 flex flex-col gap-2 text-body text-ink-2">
          <p data-testid="switch-details-move">{preflight.move_line}</p>
          <p data-testid="switch-oneclick-carry">{preflight.experimental_copy}</p>
          {room && !tight && room.kind !== "credits" && room.kind !== "blocked" && (
            <p data-testid="switch-details-room">
              Room on {target}: {room.words}
              {room.resets ? ` · ${room.resets}` : ""}.
            </p>
          )}
          {setup && setup.already_there > 0 && (
            <p>
              {target} already has {setup.already_there} of the {setup.already_there + offers} things this app has set up
              {offers > 0 ? `; the other ${offers} are listed under Set up first.` : "."}
            </p>
          )}
        </div>
      </Disclosure>
    </div>
  );
}

/** D-62 (AJ-E3009): one line per pool: it stays with full tools or its
 * shell only, leaves (and why), or joins; a sealed pool entered or left
 * is said in amber. 0.4.1: continuing in Desktop, the copy stays in each,
 * watched. Nothing when it's in no pool. */
export function SwitchPoolLines({ sessionId, targetProfileId, destination = "cli" }: { sessionId: string; targetProfileId: string; destination?: Destination }): JSX.Element | null {
  const [lines, setLines] = useState<SwitchPoolLineInfo[]>([]);
  const [ticked, setTicked] = useState(false);
  useEffect(() => {
    let alive = true;
    setTicked(false);
    void poolsSwitchLines(sessionId, targetProfileId, destination).then(
      (next) => alive && setLines(next),
      () => alive && setLines([]),
    );
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId, destination]);
  const sealed = lines.some((line) => line.confirm);
  // The switch button waits for the tick while a sealed pool is at stake.
  useEffect(() => {
    setSealOwed(sessionId, targetProfileId, sealed && !ticked);
    return () => setSealOwed(sessionId, targetProfileId, false);
  }, [sessionId, targetProfileId, sealed, ticked]);
  if (lines.length === 0) return null;
  return (
    <Section title="Its pools" as="h4" testId="switch-pool-lines">
      <ul className="flex flex-col gap-1.5">
        {lines.map((line) =>
          line.confirm ? (
            <li key={line.pool_id} data-kind={line.kind}>
              <Alert level="needs-you" live={null}>
                {line.words}
              </Alert>
            </li>
          ) : (
            <li key={line.pool_id} className="flex items-start gap-2.5 text-body text-ink-2" data-kind={line.kind}>
              <IconPool aria-hidden="true" width={16} height={16} className="mt-0.5 shrink-0 text-accent-hi" />
              {line.words}
            </li>
          ),
        )}
      </ul>
      {sealed && (
        <RowList>
          <CheckRow
            icon={IconShield}
            title="Switch anyway"
            help="A sealed pool keeps its sessions out of every other pool. The switch waits for this tick."
            checked={ticked}
            onChange={setTicked}
            testId="switch-seal-confirm"
          />
        </RowList>
      )}
    </Section>
  );
}
