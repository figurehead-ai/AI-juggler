/**
 * The ONE frame every switch dialog uses (this Mac's switch, the switch
 * to a paired Mac), so the two can't drift (the maintainer's walk of
 * 0.1.8, 2026-10-27): a header with the step marks and the close ×, a body
 * that scrolls, and a footer with the buttons — Cancel · Back · the
 * primary — always on screen, at any window size, on every step and in
 * every phase. The keyboard rules are the same too: the phase's own
 * default (Retry, Go to it) takes focus first, Tab stays inside, and Esc
 * closes — except while the copy runs, when the flow says it's finishing
 * instead (§4.9).
 */
import { useEffect, type JSX, type ReactNode, type RefObject } from "react";

export const BUTTON = "min-h-8 rounded-lg border px-3.5 text-body";
export const SECONDARY = `${BUTTON} border-hairline text-ink hover:bg-surface-2`;
export const PRIMARY = `${BUTTON} border-accent/50 bg-accent-soft font-semibold text-accent-hi hover:border-accent/70`;
export const PRIMARY_WAITING = `${BUTTON} border-hairline text-muted`;

/** Where the dialog's controls are, for Tab. */
const FOCUSABLE = "button:not(:disabled), input:not(:disabled), summary";

/** The step marks: "Step 2 of 3: What's carried", then each step, the
 * ones done ticked. Only the steps that render are listed (§8). */
export function StepMarks({ steps, step, testId }: { steps: readonly string[]; step: number; testId: string }): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" data-testid={testId} aria-live="polite">
      <p className="text-meta text-ink">
        Step {step + 1} of {steps.length}: {steps[step]}
      </p>
      <ol className="flex flex-wrap gap-1">
        {steps.map((title, index) => (
          <li key={title} aria-current={index === step ? "step" : undefined} className={"rounded-full border px-2.5 py-0.5 text-label " + (index === step ? "border-accent/60 bg-accent-soft text-accent-hi" : "border-hairline text-ink")}>
            {index < step ? "✓ " : ""}
            {title}
          </li>
        ))}
      </ol>
    </div>
  );
}

/** Cancel, said the same everywhere (0.3: as every other dialog's —
 * no key in the label; Escape still cancels, and says so to screen
 * readers). */
export function CancelButton({ onClick, testId }: { onClick: () => void; testId: string }): JSX.Element {
  return (
    <button type="button" onClick={onClick} className={SECONDARY} data-testid={testId} aria-keyshortcuts="Escape">
      Cancel
    </button>
  );
}

/** Back, from the second step on. */
export function BackButton({ onClick, testId }: { onClick: () => void; testId: string }): JSX.Element {
  return (
    <button type="button" onClick={onClick} className={SECONDARY} data-testid={testId}>
      Back
    </button>
  );
}

/** Continue, to the next step; waits (disabled) until the step is
 * answered. A double-click never lands on the next step's button. */
export function ContinueButton({ onClick, ready, testId }: { onClick: () => void; ready: boolean; testId: string }): JSX.Element {
  return (
    <button key="continue" type="button" disabled={!ready} onClick={onClick} data-testid={testId} className={ready ? PRIMARY : PRIMARY_WAITING}>
      Continue
    </button>
  );
}

/**
 * The keyboard: the phase's default first (`data-autofocus`), else the
 * step's heading, else the first control in header, body, footer order; Tab wraps inside; Esc
 * calls `onEscape` (the flow decides whether it closes); ⏎ calls `onEnter`
 * when given (not on a button, which ⏎ already presses). Runs again when
 * `phase` changes.
 */
export function useDialogKeys(
  dialogRef: RefObject<HTMLDivElement | null>,
  phase: string,
  onEscape: () => void,
  onEnter?: () => void,
): void {
  useEffect(() => {
    const node = dialogRef.current;
    // deep/11 §9: a step change puts the keyboard on the step's heading
    // (the walk of fix-walk-a: it landed on ×, so ⏎ closed the switch).
    const heading = node?.querySelector<HTMLElement>("[data-testid=switch-body] h3");
    if (heading && !heading.hasAttribute("tabindex")) {
      // Read out, not operated: no ring (the scroll box would cut it).
      heading.tabIndex = -1;
      heading.classList.add("outline-none");
    }
    (node?.querySelector<HTMLElement>("[data-autofocus]") ?? heading ?? node?.querySelector<HTMLElement>("input:checked, button, input"))?.focus();
  }, [dialogRef, phase]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const node = dialogRef.current;
      if (event.key === "Escape") {
        event.preventDefault();
        onEscape();
        return;
      }
      if (event.key === "Enter" && onEnter && !(event.target instanceof HTMLButtonElement)) {
        event.preventDefault();
        onEnter();
        return;
      }
      if (event.key === "Tab" && node) {
        const focusables = Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE));
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dialogRef, onEscape, onEnter]);
}

export function SwitchDialogFrame({
  dialogRef,
  label,
  testId,
  header,
  footer,
  onClose,
  children,
}: {
  dialogRef: RefObject<HTMLDivElement | null>;
  /** The dialog's name ("Switch to Personal"). */
  label: string;
  testId: string;
  /** The title and step marks, when there are any. */
  header?: ReactNode;
  /** The buttons; none while the copy runs. */
  footer?: ReactNode;
  /** The close ×; none while the copy runs. */
  onClose: (() => void) | null;
  children: ReactNode;
}): JSX.Element {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        data-testid={testId}
        className="flex max-h-[min(92vh,760px)] w-full max-w-[680px] flex-col overflow-hidden rounded-xl border border-hairline bg-surface shadow-card"
      >
        <div className="flex shrink-0 items-start gap-3 px-6 pt-5" data-testid="switch-header">
          <div className="min-w-0 flex-1 pb-3">{header}</div>
          {onClose && (
            <button type="button" onClick={onClose} aria-label="Close" title="Close (Esc)" className="-mr-3 -mt-2 flex size-8 shrink-0 items-center justify-center rounded-lg text-ink hover:bg-surface-2" data-testid="switch-close">
              <span aria-hidden="true" className="text-lg leading-none">×</span>
            </button>
          )}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-5" data-testid="switch-body">
          {children}
        </div>
        {footer && (
          <div className="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-hairline px-6 py-3" data-testid="switch-footer">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
