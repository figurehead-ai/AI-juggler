import { useEffect, useRef } from "react";
import type { RecoverySummaryData } from "../lib/bridge";
import { Button } from "./primitives";

/**
 * AJ-E510 Req.3: the ONE-time restart summary. `App` fetches it via
 * the bridge (shown only while a pending, unconsumed summary
 * exists); this component owns the presentation contract — exact
 * template text, modal semantics, focus-on-open, Esc + "Got it"
 * dismiss with focus returning to the board (WCAG 2.4.3) — so the
 * crash narrative is testable without booting the whole app.
 */
export function RecoverySummaryDialog({
  summary,
  onDismiss,
}: {
  summary: RecoverySummaryData;
  onDismiss: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ref.current?.focus();
    // On dismissal focus returns to the board (the app's <main>), not
    // to the discarded overlay.
    const board = document.querySelector("main");
    return () => {
      (board as HTMLElement | null)?.focus?.();
    };
  }, []);

  return (
    <div
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-label="Restart recovery summary"
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-6 backdrop-blur-sm outline-none"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onDismiss();
        }
      }}
      data-testid="recovery-summary"
    >
      <div className="sheen w-full max-w-md rounded-card border border-hairline bg-surface p-5 shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)]">
        <h2 className="text-sm font-semibold text-ink">Juggler restarted — here is what survived</h2>
        <p className="mt-2 text-[12px] text-ink" data-testid="recovery-summary-text">
          {summary.summary}
        </p>
        <p className="mt-2 text-[10.5px] text-muted">
          Re-adopted Instances run on with a working verified stop (Terminate… in the Cleanup
          Centre or their process tab). Automatic cleanup stays suspended for 24 hours —
          re-confirm it in the Cleanup Centre when you are ready.
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <Button onClick={onDismiss} data-testid="recovery-summary-dismiss">
            Got it
          </Button>
        </div>
      </div>
    </div>
  );
}
