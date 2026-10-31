import { useEffect, useId, useRef, type InputHTMLAttributes, type KeyboardEvent, type ReactNode } from "react";

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

export function Input({ className = "", ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={`h-9 rounded-xl border border-hairline bg-surface-2 px-3 text-sm text-ink
        placeholder:text-muted transition-colors focus:border-accent/50 ${className}`}
      {...props}
    />
  );
}

/** §2.5: Tab and Shift+Tab stay inside `container` (a dialog's trap). */
export function keepTabInside(container: HTMLElement | null, event: KeyboardEvent<HTMLElement>): void {
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

/**
 * The app's sheet: a titled dialog over a dimmed window. AJ-E1503: it is
 * a real modal dialog — named by its title, focus moves inside on open
 * (the first `[data-autofocus]`, else the first control), Tab stays
 * inside, Escape closes, and focus goes back where it was.
 */
export function Modal({
  title,
  subtitle,
  onClose,
  children,
  wide = false,
  table = false,
  testId,
}: {
  title: string;
  subtitle?: string;
  onClose: () => void;
  children: ReactNode;
  /** A wider sheet (lists). */
  wide?: boolean;
  /** 0.1.8: a sheet for a table with many columns: as wide as the window
   * allows up to 64rem, never taller than the window. */
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
      dialog?.querySelector<HTMLElement>("button");
    first?.focus();
    return () => {
      if (before && document.contains(before)) before.focus();
    };
  }, []);
  const trapTab = (event: KeyboardEvent<HTMLDivElement>) => keepTabInside(dialogRef.current, event);
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-6 backdrop-blur-sm"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
        trapTab(event);
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        data-testid={testId}
        className={`sheen w-full ${table ? "flex max-h-full max-w-5xl flex-col" : wide ? "max-w-2xl" : "max-w-md"} rounded-card border border-hairline bg-surface
          shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)] p-5`}
      >
        <div className="mb-4 flex items-start justify-between gap-4">
          <div>
            <h2 id={titleId} className="text-sm font-semibold">{title}</h2>
            {subtitle && <p className="mt-0.5 text-xs text-ink-2">{subtitle}</p>}
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
        </div>
        <div data-modal-body className={table ? "flex min-h-0 flex-1 flex-col" : undefined}>
          {children}
        </div>
      </div>
    </div>
  );
}
