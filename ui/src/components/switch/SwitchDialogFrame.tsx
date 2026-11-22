/**
 * The ONE frame every switch dialog uses (this Mac's switch, the switch
 * to a paired Mac), so the two can't drift (the maintainer's walk of
 * 0.1.8, 2026-10-27): a header with the step marks and the close ×, a body
 * that scrolls, and a footer with the buttons — Cancel · Back · the
 * primary — always on screen, at any window size, on every step and in
 * every phase. 0.5.0: it is the shared DialogFrame (DialogFrame.tsx), and a
 * choice that sets what the primary does (Move or Copy) sits in the
 * header with the step marks, never scrolled away. The keyboard rules are the same too: the phase's own
 * default (Retry, Go to it) takes focus first, Tab stays inside, and Esc
 * closes — except while the copy runs, when the flow says it's finishing
 * instead (§4.9).
 */
import { useEffect, type JSX, type ReactNode, type RefObject } from "react";
import { Button } from "../primitives";
import { IconCheck, IconClose } from "../icons-controls";
import { DialogFrame, DialogOverlay } from "../DialogFrame";


/** Where the dialog's controls are, for Tab. */
const FOCUSABLE = "button:not(:disabled), input:not(:disabled), summary";

/** The step marks (WIZ-1, DESIGN-SYSTEM §4.3): "Step 2 of 3: What's
 * carried" for screen readers, then each step as a numbered dot and its name,
 * joined: the ones done ticked on the accent's tint, the current one filled
 * in the accent and marked `aria-current`, the rest a quiet ring. Only the
 * steps that render are listed (§8). */
export function StepMarks({ steps, step, testId }: { steps: readonly string[]; step: number; testId: string }): JSX.Element {
  return (
    <div className="flex flex-col gap-2" data-testid={testId} aria-live="polite">
      {/* Said to screen readers only: the stepper below shows it (D-142 B3:
          capitals only over groups, never a second heading). */}
      <p className="sr-only">
        Step {step + 1} of {steps.length}: {steps[step]}
      </p>
      <ol className="flex flex-wrap items-center gap-x-2 gap-y-1.5">
        {steps.map((title, index) => {
          const done = index < step;
          const now = index === step;
          return (
            <li key={title} aria-current={now ? "step" : undefined} data-done={done ? "true" : undefined} className="flex items-center gap-2">
              {index > 0 && <span aria-hidden="true" className={"h-px w-5 " + (done || now ? "bg-accent/60" : "bg-hairline")} />}
              <span
                aria-hidden="true"
                className={
                  "flex size-5 shrink-0 items-center justify-center rounded-full text-label font-semibold tabular-nums " +
                  (now ? "bg-accent-fill text-white" : done ? "bg-accent-soft text-accent-hi" : "text-muted inset-ring-[1.5px] inset-ring-faint/70")
                }
              >
                {done ? <IconCheck width={12} height={12} strokeWidth={3} /> : index + 1}
              </span>
              {done && <span className="sr-only">Done: </span>}
              <span className={"text-meta " + (now ? "font-semibold text-ink" : done ? "text-ink-2" : "text-muted")}>{title}</span>
            </li>
          );
        })}
      </ol>
    </div>
  );
}

/** Cancel, said the same everywhere (0.3: as every other dialog's —
 * no key in the label; Escape still cancels, and says so to screen
 * readers). */
export function CancelButton({ onClick, testId }: { onClick: () => void; testId: string }): JSX.Element {
  return (
    <Button variant="ghost" type="button" onClick={onClick} data-testid={testId} aria-keyshortcuts="Escape">
      Cancel
    </Button>
  );
}

/** Back, from the second step on. */
export function BackButton({ onClick, testId }: { onClick: () => void; testId: string }): JSX.Element {
  return (
    <Button variant="ghost" type="button" onClick={onClick} data-testid={testId}>
      Back
    </Button>
  );
}

/** Next, to the next step, named by where it goes (WIZ-3: "Next: What's
 * carried"); waits (disabled) until the step is answered, the footer's
 * reason line saying why (BTN-5). A double-click never lands on the next
 * step's button. */
export function ContinueButton({ onClick, ready, testId, next }: { onClick: () => void; ready: boolean; testId: string; next?: string }): JSX.Element {
  return (
    <Button variant="primary" key="continue" type="button" disabled={!ready} onClick={onClick} data-testid={testId}>
      {next ? `Next: ${next}` : "Continue"}
    </Button>
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
        const first = focusables[0]!;
        const last = focusables[focusables.length - 1]!;
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
    <DialogOverlay className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <DialogFrame
        frameRef={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        data-testid={testId}
        size="M-cards"
        className="rounded-card border border-hairline bg-surface shadow-card"
        headerTestId="switch-header"
        headerClassName="flex items-start gap-3 px-6 pt-5 pb-1"
        header={
          <>
            <div className="min-w-0 flex-1 pb-4">{header}</div>
            {onClose && (
              <Button variant="ghost" size="sm" type="button" onClick={onClose} aria-label="Close" title="Close (Esc)" className="-mr-2 -mt-1 size-7 shrink-0 px-0 text-ink-2" data-testid="switch-close">
                {/* The set's × fills half its box: 24 draws a 12 px cross (never shrunk by the button's padding), a 28 px hit target. */}
                <IconClose aria-hidden="true" width={24} height={24} strokeWidth={2} className="shrink-0" />
              </Button>
            )}
          </>
        }
        bodyTestId="switch-body"
        bodyClassName="px-6 pt-1 pb-6"
        footer={footer}
        footerTestId="switch-footer"
        footerClassName="flex flex-wrap items-center justify-end gap-2 border-t border-hairline px-6 py-3"
      >
        {children}
      </DialogFrame>
    </DialogOverlay>
  );
}
