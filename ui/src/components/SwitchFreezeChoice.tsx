/**
 * AJ-E1540: how the original stops before its conversation is copied.
 * Both switch doors show the same choice, with what each one means, an
 * example, "What does this mean?" and the guide (the switch dialog's
 * rule, 2026-10-27). A session AI Juggler can't stop (started outside
 * it) gets no choice: the screen says it keeps running and the copy is
 * separate. AJ-E2108: a session another app runs says where it stays,
 * untouched, and where the continued session runs (0.3.5: Desktop or
 * CLI, as chosen).
 */
import type { JSX } from "react";
import { Explain } from "./switch/Explain";
import { GUIDE } from "../lib/switch-words";
import { RadioCard, RadioCards } from "./controls";
import { IconStop } from "./icons-actions";
import { IconClock } from "./icons-menu";
import { Pill } from "./Pill";
import { Section } from "./Section";
import { LogoTile } from "./ToolLogo";
import type { PreflightInfoData } from "../lib/ipc/switch-preflight";

export type FreezeChoice = "finish-turn" | "stop-now";

/** Where a session another app runs stays, and where it continues, in
 * words: "The original stays open in Claude app, untouched. It continues in
 * Claude app · Personal." `where` is the choice in words
 * (`whereWords`); without it, where it continues is chosen later. */
export function outsideWords(app: string | null | undefined, live: boolean, target: string, where?: string | null): string {
  const stays = outsideStays(app, live);
  // 0.3.2 (the maintainer's walk: it moved to a terminal unannounced) and
  // 0.3.5 (always two choices, named Desktop and CLI): where the work goes
  // on is said before the switch, plainly.
  return where ? `${stays} It continues ${where}.` : `${stays} You choose where ${target} continues it, Desktop or CLI, on the last step.`;
}

/** "The original stays open in Claude, untouched." */
export function outsideStays(app: string | null | undefined, live: boolean): string {
  return `The original ${live ? `stays open in ${app ?? "its app"}, untouched` : "stays as it is, untouched"}.`;
}

/** 0.4.1 (the rc5 walk: step 1 said the original stays, step 3 and the
 * done screen that it stops): what a copy does to the original, one true
 * line for every screen. A running original stops first (its reply
 * finished or cut off, as chosen on the confirm step), then stays as it
 * was; one at rest only stays. `before` is the Target step's, `after` the
 * done screen's; the confirm summary says it with the stop chosen
 * ([`originalWords`]). */
export function originalFate(
  phase: "before" | "after",
  { live, keepsRunning, outside }: { live: boolean; keepsRunning: boolean; outside: { app: string | null } | null },
): string {
  if (outside) return outsideStays(outside.app, live);
  if (keepsRunning) return phase === "before" ? CANT_STOP : "The original keeps going in its app, beside the copy.";
  if (!live) return BACK_ANY_TIME;
  return phase === "before"
    ? "The original stops first, then stays as it was. You can switch back to it any time."
    : "The original has stopped and stays as it was. You can switch back to it any time.";
}

/** [`originalFate`]'s facts, from the preflight. */
export function fateOf(preflight: Pick<PreflightInfoData, "source_live" | "source_keeps_running" | "source_outside" | "source_app">) {
  return {
    live: preflight.source_live,
    keepsRunning: preflight.source_keeps_running,
    outside: preflight.source_outside ? { app: preflight.source_app ?? null } : null,
  };
}

const CANT_STOP = "AI Juggler can't stop this session: if it's still open, it keeps going there, and the switch starts a separate continuation.";

const BACK_ANY_TIME = "The original stays as it was. You can switch back to it any time.";

/** What happens to the original, in one line (the confirm summary). */
export function originalWords({
  live,
  keepsRunning,
  outside,
  choice,
}: {
  live: boolean;
  keepsRunning: boolean;
  outside: { app: string | null; target: string; where?: string | null } | null;
  choice: FreezeChoice;
}): string {
  if (outside) return outsideWords(outside.app, live, outside.target, outside.where);
  if (keepsRunning) return CANT_STOP;
  if (!live) return BACK_ANY_TIME;
  return choice === "finish-turn"
    ? "The original finishes its current reply, then stops and stays as it was. You can switch back to it any time."
    : "The original stops now (an unfinished reply is marked as cut off), then stays as it was. You can switch back to it any time.";
}

export function SwitchFreezeChoice({
  sourceLive,
  keepsRunning = false,
  outside,
  value,
  onChange,
}: {
  sourceLive: boolean;
  keepsRunning?: boolean;
  /** A session another app runs: where it is open, the target's name,
   * and where it continues (in words). */
  outside?: { app: string | null; target: string; where?: string | null } | null;
  value: FreezeChoice;
  onChange: (choice: FreezeChoice) => void;
}): JSX.Element | null {
  if (outside) {
    return (
      <p className="text-body text-ink" data-testid="switch-outside">
        {outsideWords(outside.app, sourceLive, outside.target, outside.where)}
      </p>
    );
  }
  if (keepsRunning) {
    return (
      <p className="text-body text-ink" data-testid="switch-keeps-running">
        AI Juggler can't stop this session: if it's still open, it keeps going there, and the
        switch starts a separate continuation.
      </p>
    );
  }
  // At rest, there is nothing to choose (the summary says it stays).
  // 0.3.5: one line each; the examples are under "What does this mean?".
  if (!sourceLive) return null;
  return (
    <Section title="When to stop the original" testId="switch-freeze">
      <RadioCards label="When to stop the original" value={value} onChange={(next) => onChange(next === "stop-now" ? "stop-now" : "finish-turn")} columns={2}>
        <RadioCard
          value="finish-turn"
          logo={<LogoTile icon={IconClock} size={32} on={value === "finish-turn"} />}
          title={
            <span className="inline-flex flex-wrap items-center gap-x-2 gap-y-1">
              Let it finish its reply, then switch
              <Pill tone="accent">Recommended</Pill>
            </span>
          }
          sub="Nothing it was writing is cut off."
        />
        <RadioCard value="stop-now" logo={<LogoTile icon={IconStop} size={32} on={value === "stop-now"} />} title="Stop it now and switch" sub="An unfinished reply is marked as cut off." />
      </RadioCards>
      <Explain
        guide={GUIDE.original}
        testId="switch-freeze-explain"
        tips={[
          "A conversation can't be copied while it's still being written, so the original stops first.",
          "Let it finish: if it's writing a test, the test is finished first.",
          "Stop it now: an unfinished reply is marked as cut off, so the new session knows to pick it up again.",
          "Either way the original stays: you can switch back to it any time.",
          "AI Juggler can stop only sessions it started; others keep going in their app.",
        ]}
      />
    </Section>
  );
}
