/**
 * AJ-E1540: how the source stops before its conversation is copied. Both
 * switch doors (one-click and the wizard's Confirm step) show the same two
 * choices. A session AI Juggler can't stop (started outside it) gets no
 * choice: the screen says it keeps running and the copy is separate.
 */
import type { JSX } from "react";

export type FreezeChoice = "finish-turn" | "stop-now";

export function SwitchFreezeChoice({
  sourceLive,
  keepsRunning = false,
  value,
  onChange,
}: {
  sourceLive: boolean;
  keepsRunning?: boolean;
  value: FreezeChoice;
  onChange: (choice: FreezeChoice) => void;
}): JSX.Element {
  if (keepsRunning) {
    return (
      <p className="text-[11px] text-muted" data-testid="switch-keeps-running">
        AI Juggler can't stop this session: if it's still open, it keeps going there, and the
        switch starts a separate continuation.
      </p>
    );
  }
  return (
    <fieldset className="flex flex-col gap-1.5">
      <legend className="text-[11px] uppercase tracking-wider text-muted">
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
        Stop now and switch <span className="text-muted">(partial turn marked truncated)</span>
      </label>
    </fieldset>
  );
}
