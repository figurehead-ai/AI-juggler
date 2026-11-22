import { useEffect, useId, useRef, type ComponentProps, type KeyboardEvent, type ReactNode, type RefObject } from "react";
import { DialogFrame, DialogOverlay } from "./DialogFrame";

/**
 * Form primitives in the house design language: quiet surfaces, hairline
 * borders, focus rings, helper/error text.
 */

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-label font-semibold text-muted">
        {label}
      </span>
      {children}
      {hint && <span className="text-label text-muted">{hint}</span>}
    </label>
  );
}

/** A text field; `ref` reaches the input (a sheet that selects its value on open). */
export function Input({ className = "", ...props }: ComponentProps<"input">) {
  return (
    <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
      className={`h-9 rounded-xl border border-hairline bg-surface-2 px-3 text-sm text-ink
        placeholder:text-muted transition-colors focus:border-accent/50 ${className}`}
      {...props}
    />
  );
}

/** §2.5: Tab and Shift+Tab stay inside `container` (a dialog's trap). */
export function keepTabInside(container: HTMLElement | null, event: Pick<KeyboardEvent<HTMLElement>, "key" | "shiftKey" | "preventDefault">): void {
  if (event.key !== "Tab") return;
  const focusable = Array.from(
    container?.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex='0']") ?? [],
  );
  if (focusable.length === 0) return;
  const first = focusable[0]!;
  const last = focusable[focusable.length - 1]!;
  const at = document.activeElement;
  if (event.shiftKey && (at === first || at === container || !container?.contains(at))) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

/** Focus goes back to what had it when this mounted (if it is still
 * there) once this unmounts: a dialog returns focus to what opened it. */
export function useFocusReturn(): void {
  useEffect(() => {
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => {
      if (before && document.contains(before)) before.focus();
    };
  }, []);
}

/** A dialog that isn't the shared Modal (AJ-E710): focus goes into it when
 * it opens, and again once its choices arrive if focus isn't inside yet
 * (the first `[data-autofocus]`, else its first control); it goes back to
 * what opened it when it closes. Pair with `keepTabInside`. */
export function useDialogFocus(dialogRef: RefObject<HTMLElement | null>): void {
  useFocusReturn();
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog || dialog.contains(document.activeElement)) return;
    (dialog.querySelector<HTMLElement>("[data-autofocus]") ?? dialog.querySelector<HTMLElement>("button:not(:disabled), input:not(:disabled)"))?.focus();
  });
}

/**
 * The app's sheet: a titled dialog over a dimmed window. AJ-E1503: it is
 * a real modal dialog — named by its title, focus moves inside on open
 * (the first `[data-autofocus]`, else the first control), Tab stays
 * inside, Escape closes, and focus goes back where it was.
 * 0.5.0 (D-142, GAPS A1): a thin wrapper over DialogFrame. Never taller
 * than the window; the title and close stay, only the body scrolls, and
 * the actions (`footer`) stay at the foot. A form's submit sits in the
 * footer by its `form` attribute.
 */
export function Modal({
  title,
  subtitle,
  onClose,
  children,
  footer,
  wide = false,
  broad = false,
  table = false,
  testId,
}: {
  title: string;
  subtitle?: string;
  onClose: () => void;
  children: ReactNode;
  /** The buttons (Cancel … the one primary, last), fixed at the foot. */
  footer?: ReactNode;
  /** A wider sheet (lists): size M with cards, 680 px. */
  wide?: boolean;
  /** 0.4.1: wider still, for two columns of rows with keys or values on
   * their right (the Keyboard shortcuts sheet): size L, 768 px. */
  broad?: boolean;
  /** 0.1.8: a sheet for a table with many columns: size XL, up to 1024 px. */
  table?: boolean;
  testId?: string;
}) {
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    const dialog = dialogRef.current;
    const first =
      dialog?.querySelector<HTMLElement>("[data-autofocus]") ??
      dialog?.querySelector<HTMLElement>("[data-modal-body] button, [data-modal-body] input, [data-modal-body] select, [data-modal-body] textarea") ??
      // Then the footer's first button (Cancel), never the header's ×.
      dialog?.querySelector<HTMLElement>("[data-dialog-part='footer'] button") ??
      dialog?.querySelector<HTMLElement>("button");
    first?.focus();
    return () => {
      if (before && document.contains(before)) before.focus();
    };
  }, []);
  const trapTab = (event: KeyboardEvent<HTMLDivElement>) => keepTabInside(dialogRef.current, event);
  return (
    <DialogOverlay
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
        trapTab(event);
      }}
    >
      <DialogFrame
        frameRef={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        data-testid={testId}
        size={table ? "XL" : broad ? "L" : wide ? "M-cards" : "S"}
        className="sheen rounded-card border border-hairline bg-surface shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)]"
        headerClassName="flex items-start justify-between gap-4 px-5 pt-5 pb-3"
        header={
          <>
            <div className="min-w-0">
              <h2 id={titleId} className="type-section">{title}</h2>
              {subtitle && <p className="type-help mt-0.5">{subtitle}</p>}
            </div>
            <button
              onClick={onClose}
              className="rounded-lg p-1 text-muted transition-colors hover:bg-surface-2 hover:text-ink"
              aria-label="Close"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M6 6l12 12M18 6L6 18" />
              </svg>
            </button>
          </>
        }
        // Room for a focus ring the scroll box would otherwise cut.
        bodyClassName={`px-5 pt-1 ${footer ? "pb-4" : "pb-5"} ${table ? "flex flex-col" : ""}`}
        footer={footer}
        footerClassName="flex flex-wrap items-center justify-end gap-2 px-5 py-3"
      >
        <div data-modal-body className={table ? "flex min-h-0 flex-1 flex-col" : undefined}>
          {children}
        </div>
      </DialogFrame>
    </DialogOverlay>
  );
}
