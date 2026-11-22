/**
 * The small sheet that confirms a change that removes something (unlink
 * this Mac, take a Mac's access away, clean a process on another Mac): the
 * question, what happens, Cancel and the one danger action. Cancel has
 * focus (the non-destructive choice), Esc cancels, Tab stays inside, and
 * focus goes back to what asked once it closes (AJ-E710).
 */
import { useEffect, useRef, type JSX, type ReactNode } from "react";
import { DialogFrame, DialogOverlay } from "./DialogFrame";
import { keepTabInside, useFocusReturn } from "./form";
import { Button } from "./primitives";

export function ConfirmSheet({
  idPrefix,
  testId,
  title,
  children,
  action,
  actionTestId,
  busy,
  onCancel,
  onConfirm,
}: {
  /** The title's and the body's ids: `<idPrefix>-title`, `<idPrefix>-body`. */
  idPrefix: string;
  testId: string;
  title: ReactNode;
  /** What happens, in the body's one paragraph. */
  children: ReactNode;
  /** The danger button's words. */
  action: ReactNode;
  actionTestId: string;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}): JSX.Element {
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const boxRef = useRef<HTMLDivElement | null>(null);
  useFocusReturn();
  useEffect(() => {
    cancelRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);
  return (
    <DialogOverlay ref={boxRef} className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <DialogFrame
        className="sheen rounded-card border border-hairline bg-surface shadow-card w-full max-w-sm"
        onKeyDown={(event) => keepTabInside(boxRef.current, event)}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={`${idPrefix}-title`}
        aria-describedby={`${idPrefix}-body`}
        data-testid={testId}
        bodyClassName="px-5 pb-1.5"
        footerClassName="px-5 pt-2.5 pb-5"
        footer={
          <div className="flex justify-end gap-2">
            <Button ref={cancelRef} size="sm" variant="secondary" onClick={onCancel}>
              Cancel
            </Button>
            <Button size="sm" variant="danger" disabled={busy} onClick={onConfirm} data-testid={actionTestId}>
              {action}
            </Button>
          </div>
        }
        headerClassName="px-5 pt-5"
        header={
          <p id={`${idPrefix}-title`} className="text-sm font-semibold">
            {title}
          </p>
        }
      >
        <p id={`${idPrefix}-body`} className="mt-1.5 text-xs leading-relaxed text-ink-2">
          {children}
        </p>
      </DialogFrame>
    </DialogOverlay>
  );
}
