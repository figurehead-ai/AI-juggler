import { useEffect, useRef } from "react";
import type { RecoverySummaryData } from "../lib/ipc/cleanup";
import { Button } from "./primitives";
import { DialogFrame, DialogOverlay } from "./DialogFrame";

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
    <DialogOverlay
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-label="Restart recovery summary"
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-4 backdrop-blur-sm outline-none"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onDismiss();
        }
      }}
      data-testid="recovery-summary"
    >
      <DialogFrame
        className="sheen w-full max-w-md rounded-card border border-hairline bg-surface shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)]"
        headerClassName="px-5 pt-5"
        bodyClassName="px-5 pb-1.5"
        footerClassName="px-5 pt-2.5 pb-5"
        header={
          <h2 className="text-sm font-semibold text-ink">AI Juggler restarted — here is what kept running</h2>
        }
        footer={
          <div className="flex justify-end gap-2">
            <Button onClick={onDismiss} data-testid="recovery-summary-dismiss">
              Got it
            </Button>
          </div>
        }
      >
        <p className="mt-2 text-label text-ink" data-testid="recovery-summary-text">
          {summary.summary}
        </p>
        <p className="mt-2 text-label text-muted">
          Sessions picked up again keep running, and Stop… still stops them safely (on the
          Processes page or a session's Processes tab). Automatic cleanup stays paused for 24 hours —
          turn it back on from the Processes page when you are ready.
        </p>
      </DialogFrame>
    </DialogOverlay>
  );
}
