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
import { Radio } from "./controls";

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
  if (keepsRunning) return "AI Juggler can't stop this session: if it's still open, it keeps going there, and the switch starts a separate continuation.";
  if (!live) return "The original stays as it is. You can switch back to it any time.";
  return choice === "finish-turn"
    ? "The original finishes its current reply, then stops. You can switch back to it any time."
    : "The original stops now; an unfinished reply is marked as cut off. You can switch back to it any time.";
}

const radio = "shrink-0";

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
    <fieldset className="flex flex-col gap-1.5" data-testid="switch-freeze">
      <legend className="mb-1 text-meta font-semibold text-ink">When to stop the original</legend>
      <label className="flex items-center gap-2.5 text-body text-ink">
        <Radio name="freeze" className={radio} checked={value === "finish-turn"} onChange={() => onChange("finish-turn")} />
        <span>
          Let it finish its reply, then switch{" "}
          <span className="ml-1 rounded-full border border-success/30 bg-success-soft px-1.5 py-px text-label font-medium text-success">recommended</span>
        </span>
      </label>
      <label className="flex items-center gap-2.5 text-body text-ink">
        <Radio name="freeze" className={radio} checked={value === "stop-now"} onChange={() => onChange("stop-now")} />
        <span>Stop it now and switch</span>
      </label>
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
    </fieldset>
  );
}
