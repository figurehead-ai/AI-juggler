/**
 * AJ-E1540: how the source stops before its conversation is copied. Both
 * switch doors (one-click and the wizard's Confirm step) show the same two
 * choices. A session AI Juggler can't stop (started outside it) gets no
 * choice: the screen says it keeps running and the copy is separate.
 * AJ-E2108: a session another app runs says where it stays, untouched,
 * and where the continued session runs.
 */
import type { JSX } from "react";

export type FreezeChoice = "finish-turn" | "stop-now";

/** Where a session another app runs stays, and where it continues, in
 * words: "The original stays open in Claude, untouched: …". */
export function outsideWords(app: string | null | undefined, live: boolean, target: string): string {
  const where = live ? `stays open in ${app ?? "its app"}, untouched` : "stays as it is, untouched";
  return `The original ${where}: AI Juggler only reads its conversation. ${target} continues it as a new session here in AI Juggler.`;
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
  /** A session another app runs: where it is open, and the target's name. */
  outside?: { app: string | null; target: string } | null;
  value: FreezeChoice;
  onChange: (choice: FreezeChoice) => void;
}): JSX.Element {
  if (outside) {
    return (
      <p className="text-label text-muted" data-testid="switch-outside">
        {outsideWords(outside.app, sourceLive, outside.target)}
      </p>
    );
  }
  if (keepsRunning) {
    return (
      <p className="text-label text-muted" data-testid="switch-keeps-running">
        AI Juggler can't stop this session: if it's still open, it keeps going there, and the
        switch starts a separate continuation.
      </p>
    );
  }
  return (
    <fieldset className="flex flex-col gap-1.5">
      <legend className="text-section font-semibold text-ink">
        {sourceLive ? "Freeze the source first" : "The source is at rest"}
      </legend>
      <label className="flex items-center gap-2 text-xs">
        <input
          type="radio"
          name="freeze"
          checked={value === "finish-turn"}
          onChange={() => onChange("finish-turn")}
        />
        Finish this turn, then switch <span className="text-muted">(default)</span>
      </label>
      <label className="flex items-center gap-2 text-xs">
        <input
          type="radio"
          name="freeze"
          checked={value === "stop-now"}
          onChange={() => onChange("stop-now")}
        />
        Stop now and switch <span className="text-muted">(an unfinished reply is marked as cut off)</span>
      </label>
    </fieldset>
  );
}
