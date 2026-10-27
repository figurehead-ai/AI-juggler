/**
 * AJ-E1540: how the original stops before its conversation is copied.
 * Both switch doors show the same choice, with what each one means, an
 * example, "What does this mean?" and the guide (the switch dialog's
 * rule, 2026-10-27). A session AI Juggler can't stop (started outside
 * it) gets no choice: the screen says it keeps running and the copy is
 * separate. AJ-E2108: a session another app runs says where it stays,
 * untouched, and where the continued session runs.
 */
import type { JSX } from "react";
import { Explain } from "./switch/Explain";
import { GUIDE } from "../lib/switch-words";

export type FreezeChoice = "finish-turn" | "stop-now";

/** Where a session another app runs stays, and where it continues, in
 * words: "The original stays open in Claude, untouched: …". */
export function outsideWords(app: string | null | undefined, live: boolean, target: string): string {
  const where = live ? `stays open in ${app ?? "its app"}, untouched` : "stays as it is, untouched";
  return `The original ${where}: AI Juggler only reads its conversation. ${target} continues it as a new session here in AI Juggler.`;
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
  outside: { app: string | null; target: string } | null;
  choice: FreezeChoice;
}): string {
  if (outside) return outsideWords(outside.app, live, outside.target);
  if (keepsRunning) return "AI Juggler can't stop this session: if it's still open, it keeps going there, and the switch starts a separate continuation.";
  if (!live) return "The original stays as it is. You can switch back to it any time.";
  return choice === "finish-turn"
    ? "The original finishes its current reply, then stops. You can switch back to it any time."
    : "The original stops now; an unfinished reply is marked as cut off. You can switch back to it any time.";
}

const radio = "mt-1 size-4 shrink-0 accent-[var(--color-accent)]";

export function SwitchFreezeChoice({
  sourceLive,
  keepsRunning = false,
  outside,
  value,
  onChange,
}: {
  sourceLive: boolean;
  keepsRunning?: boolean;
  /** A session another app runs: where it is open, and the target's name. */
  outside?: { app: string | null; target: string } | null;
  value: FreezeChoice;
  onChange: (choice: FreezeChoice) => void;
}): JSX.Element | null {
  if (outside) {
    return (
      <p className="text-body text-ink" data-testid="switch-outside">
        {outsideWords(outside.app, sourceLive, outside.target)}
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
  if (!sourceLive) return null;
  return (
    <fieldset className="flex flex-col gap-2.5" data-testid="switch-freeze">
      <legend className="mb-1 text-body font-semibold text-ink">When to stop the original</legend>
      <label className="flex items-start gap-2.5 text-body text-ink">
        <input type="radio" name="freeze" className={radio} checked={value === "finish-turn"} onChange={() => onChange("finish-turn")} />
        <span>
          Let it finish its reply, then switch <span className="text-label text-success">recommended</span>
          <span className="block text-meta">For example, if it's writing a test, the test is finished first.</span>
        </span>
      </label>
      <label className="flex items-start gap-2.5 text-body text-ink">
        <input type="radio" name="freeze" className={radio} checked={value === "stop-now"} onChange={() => onChange("stop-now")} />
        <span>
          Stop it now and switch
          <span className="block text-meta">An unfinished reply is marked as cut off, so the new session knows to pick it up again.</span>
        </span>
      </label>
      <Explain
        guide={GUIDE.original}
        testId="switch-freeze-explain"
        tips={[
          "A conversation can't be copied while it's still being written, so the original stops first.",
          "Either way the original stays: you can switch back to it any time.",
          "AI Juggler can stop only sessions it started; others keep going in their app.",
        ]}
      />
    </fieldset>
  );
}
