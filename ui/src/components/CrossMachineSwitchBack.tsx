/**
 * AJ-E706 Req.4 / AJ-E708 Req.4–5 — switching an away Instance back to
 * the machine its work came from, and the sheet when both sides moved on.
 *
 * D-05 across machines: the original native session resumes there with
 * exactly ONE "While you were away" turn — previewed here before the
 * click — and is never overwritten. When the original's machine reports
 * that it moved on too, nothing runs: the sheet shows both sides (counts
 * and times, plain words) and offers Keep mine / Keep theirs / Keep both,
 * each described. Default focus is "Review later" (non-destructive); no
 * auto-dismiss; the choices are a radio group, keyboard-completable.
 *
 * 0.5.1 F71: on the one switch frame (D-142, DESIGN-SYSTEM §8):
 * `SwitchDialogFrame` with `SwitchHeader`'s icon tile and from → to strip,
 * the update read back as "You" and "Reply" turns on a tinted inset (never
 * raw "[user]" lines), what each side did as a `FactPanel`, the choices as
 * `RadioCards`, times in words, and one primary in the fixed footer.
 */
import { useRef, useState, type JSX } from "react";
import { errorText } from "../lib/bridge";
import { readDelta } from "../lib/delta-words";
import { fleetDivergenceKeepTheirs, fleetSwitchBackExecute } from "../lib/ipc/fleet-switch";
import { agoWords } from "../lib/time-words";
import type { DivergenceInfo } from "../types/generated/DivergenceInfo";
import type { FleetSwitchBackPlanInfo } from "../types/generated/FleetSwitchBackPlanInfo";
import { Alert } from "./Alert";
import { CheckRow, RadioCard, RadioCards, RowList } from "./controls";
import { IconComputer, IconCopy } from "./icons-actions";
import { IconPlay } from "./icons-controls";
import { IconFleet, IconStateHandover } from "./icons";
import { IconHistory } from "./icons-places";
import { IconWarn } from "./icons-menu";
import { Button } from "./primitives";
import { Section } from "./Section";
import { LogoTile } from "./ToolLogo";
import { Fact, FactPanel } from "./switch/Facts";
import { DoneHead } from "./switch/StepParts";
import { CancelButton, SwitchDialogFrame, useDialogKeys } from "./switch/SwitchDialogFrame";
import { SwitchHeader } from "./switch/SwitchHeader";

const CHOICES = ["keep_mine", "keep_theirs", "keep_both"] as const;
type Choice = (typeof CHOICES)[number];

type Phase =
  | { kind: "plan" }
  | { kind: "running" }
  | { kind: "diverged"; divergence: DivergenceInfo }
  | { kind: "done"; message: string }
  | { kind: "failed"; message: string };

const turns = (n: number) => `${n} new ${n === 1 ? "turn" : "turns"}`;

/** When, in words ("30 min ago"), the day and time on hover (CNT-10). */
function When({ ms }: { ms: number | null }): JSX.Element {
  // Read once, when the sheet shows (render stays pure).
  const [now] = useState(() => Date.now());
  if (ms === null) return <>at a time it didn’t say</>;
  const at = new Date(ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  return <time dateTime={new Date(ms).toISOString()} title={at}>{agoWords(ms, now) || at}</time>;
}

/** The one update the original receives, read back as turns. */
function Update({ text, count }: { text: string; count: number }): JSX.Element {
  const read = readDelta(text);
  return (
    <Section title="What the original receives" help={`One update: ${turns(count)} from here`} testId="cross-back-update">
      <div className="flex max-h-56 flex-col gap-3 overflow-y-auto rounded-card bg-surface-2 px-4 py-3" data-testid="cross-back-delta" tabIndex={0} aria-label="The update the original receives">
        <p className="text-meta text-ink-2">{read.lead}</p>
        {read.groups.map((group, index) => (
          <div key={`${group.where ?? ""}-${index}`} className="flex flex-col gap-1.5">
            {group.where && <p className="text-meta font-medium text-ink-2">{group.where}</p>}
            <ul className="flex flex-col gap-1.5">
              {group.turns.map((turn, at) => (
                <li key={at} className="grid grid-cols-[3.5rem_minmax(0,1fr)] items-baseline gap-x-3 text-body" data-testid="cross-back-turn" data-who={turn.who}>
                  <span className="text-meta text-muted">{turn.who}</span>
                  <span className="text-ink [overflow-wrap:anywhere]">{turn.text}</span>
                </li>
              ))}
            </ul>
          </div>
        ))}
        {read.notes.map((note) => (
          <p key={note} className="text-meta text-ink-2">
            {note}
          </p>
        ))}
      </div>
    </Section>
  );
}

export function CrossMachineSwitchBack({
  sessionId,
  plan,
  divergence,
  onClose,
  onResolved,
}: {
  sessionId: string;
  plan: FleetSwitchBackPlanInfo;
  /** Open straight on the sheet (a divergence already known). */
  divergence?: DivergenceInfo | null;
  onClose: () => void;
  onResolved?: () => void;
}): JSX.Element {
  const [phase, setPhase] = useState<Phase>(divergence ? { kind: "diverged", divergence } : { kind: "plan" });
  const [keepAway, setKeepAway] = useState(false);
  const [choice, setChoice] = useState<Choice | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const machine = plan.original_machine_label;
  const running = phase.kind === "running";
  const close = () => !running && onClose();
  useDialogKeys(dialogRef, phase.kind, close);

  const run = async (pick: "" | "keep_mine" | "keep_both") => {
    setPhase({ kind: "running" });
    try {
      const outcome = await fleetSwitchBackExecute(sessionId, pick, keepAway);
      if (outcome.accepted) {
        setPhase({ kind: "done", message: outcome.message });
        onResolved?.();
      } else if (outcome.reason_code === "diverged" && outcome.divergence) {
        setChoice(null);
        setPhase({ kind: "diverged", divergence: outcome.divergence });
      } else {
        setPhase({ kind: "failed", message: outcome.message });
      }
    } catch (cause) {
      setPhase({ kind: "failed", message: errorText(cause) });
    }
  };

  const apply = async () => {
    if (choice === "keep_theirs") {
      try {
        await fleetDivergenceKeepTheirs(sessionId);
        setPhase({ kind: "done", message: `Kept as they are: the work stays here, and ${machine}'s original continues there. Nothing was sent.` });
        onResolved?.();
      } catch (cause) {
        setPhase({ kind: "failed", message: errorText(cause) });
      }
      return;
    }
    if (choice) void run(choice);
  };

  const diverged = phase.kind === "diverged";
  const header = (
    <SwitchHeader
      icon={diverged ? IconWarn : IconStateHandover}
      title={diverged ? "Both sides moved on" : `Switch back to ${machine}`}
      sub={diverged ? "Nothing was merged and nothing ran. Choose what to keep." : `The original on ${plan.original_account_label} resumes there, never overwritten.`}
      from={{ name: `This Mac · ${plan.away_account_label}`, logo: <LogoTile icon={IconComputer} size={24} /> }}
      to={{ name: `${machine} · ${plan.original_account_label}`, logo: <LogoTile icon={IconFleet} size={24} on /> }}
      testId="cross-back-header"
      titleTestId="cross-back-title"
      stripTestId="cross-back-strip"
    />
  );

  let body: JSX.Element | null = null;
  let footer: JSX.Element | null = null;
  if (phase.kind === "diverged") {
    const options: Array<{ id: Choice; title: string; body: string; icon: typeof IconCopy }> = [
      {
        id: "keep_mine",
        title: "Keep mine",
        icon: IconStateHandover,
        body: `The original on ${machine} resumes with this work's update; its ${turns(phase.divergence.turns_since)} stay before it.`,
      },
      {
        id: "keep_theirs",
        title: "Keep theirs",
        icon: IconHistory,
        body: `Both stay as they are: ${machine}'s original continues there, this work stays here. Nothing is sent.`,
      },
      {
        id: "keep_both",
        title: "Keep both",
        icon: IconCopy,
        body: `A copy of the original on ${machine} takes this work's update; the original itself stays untouched.`,
      },
    ];
    body = (
      <div className="flex flex-col gap-5">
        <FactPanel label="Since the work left" testId="conflict-sides">
          <Fact icon={IconComputer} label="Here">
            {turns(plan.new_turns)} on {plan.away_account_label}
          </Fact>
          <Fact icon={IconFleet} label={`On ${machine}`} testId="conflict-theirs">
            <span>
              {turns(phase.divergence.turns_since)} in the original, last active <When ms={phase.divergence.last_activity_ms} />
            </span>
          </Fact>
        </FactPanel>
        <Section title="What to keep" testId="conflict-keep">
          <RadioCards label="What to keep" value={choice} onChange={(value) => setChoice(CHOICES.find((one) => one === value) ?? null)} testId="conflict-choices">
            {options.map((option) => (
              <RadioCard key={option.id} value={option.id} align="start" logo={<LogoTile icon={option.icon} size={32} on={choice === option.id} />} title={option.title} sub={option.body} testId={`conflict-${option.id}`} />
            ))}
          </RadioCards>
        </Section>
      </div>
    );
    footer = (
      <>
        {choice === null && <p className="mr-auto text-meta text-ink-2" data-testid="conflict-why">Choose what to keep first.</p>}
        <Button variant="ghost" type="button" data-autofocus onClick={onClose} data-testid="conflict-later">
          Review later
        </Button>
        <Button variant="primary" type="button" disabled={choice === null} onClick={() => void apply()} data-testid="conflict-apply">
          {choice === "keep_theirs" ? "Keep both as they are" : choice === "keep_both" ? `Keep both on ${machine}` : `Keep mine on ${machine}`}
        </Button>
      </>
    );
  } else if (phase.kind === "plan") {
    const blocked = plan.blockers.length > 0;
    body = (
      <div className="flex flex-col gap-5">
        {blocked && (
          <Alert level="needs-you" title="Can’t switch back yet" testId="cross-back-blockers">
            <ul className="mt-1 flex flex-col gap-1">
              {plan.blockers.map((reason) => (
                <li key={reason}>{reason}</li>
              ))}
            </ul>
          </Alert>
        )}
        <Update text={plan.delta_preview} count={plan.new_turns} />
        <RowList>
          <CheckRow icon={IconPlay} title="Keep this session running too" help={`Otherwise it stops once the original resumes on ${machine}.`} checked={keepAway} onChange={setKeepAway} testId="cross-back-keep" />
        </RowList>
      </div>
    );
    footer = (
      <>
        {blocked && <p className="mr-auto text-meta text-ink-2">Clear the points above to switch back.</p>}
        <CancelButton onClick={onClose} testId="cross-back-cancel" />
        <Button variant="primary" type="button" disabled={blocked} onClick={() => void run("")} data-testid="cross-back-go">
          Switch back to {machine}
        </Button>
      </>
    );
  } else if (phase.kind === "running") {
    body = (
      <p className="text-body text-ink" aria-live="polite" data-testid="cross-back-running">
        Asking {machine} to resume the original…
      </p>
    );
  } else if (phase.kind === "done") {
    body = (
      <div aria-live="polite" data-testid="cross-back-done">
        <DoneHead title={<>Back on {machine}</>} help={phase.message} />
      </div>
    );
    footer = (
      <Button variant="primary" type="button" data-autofocus onClick={onClose} data-testid="cross-back-close">
        Done
      </Button>
    );
  } else {
    body = (
      <Alert level="broken" title="Didn’t switch back" testId="cross-back-failed">
        {phase.message}
      </Alert>
    );
    footer = (
      <Button variant="secondary" type="button" data-autofocus onClick={onClose} data-testid="cross-back-close">
        Close
      </Button>
    );
  }

  return (
    <SwitchDialogFrame dialogRef={dialogRef} label={diverged ? "Both sides moved on" : `Switch back to ${machine}`} testId="cross-switch-back" header={header} footer={footer} onClose={running ? null : onClose}>
      {body}
    </SwitchDialogFrame>
  );
}
