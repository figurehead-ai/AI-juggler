import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";

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
      <span className="text-[11px] uppercase tracking-[0.14em] text-faint">
        {label}
      </span>
      {children}
      {hint && <span className="text-[11px] text-muted">{hint}</span>}
    </label>
  );
}

export function Input({ className = "", ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={`h-9 rounded-xl border border-hairline bg-surface-2 px-3 text-sm text-ink
        placeholder:text-faint transition-colors focus:border-accent/50 ${className}`}
      {...props}
    />
  );
}

export function Select({
  className = "",
  children,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={`h-9 rounded-xl border border-hairline bg-surface-2 px-3 text-sm text-ink
        transition-colors focus:border-accent/50 ${className}`}
      {...props}
    >
      {children}
    </select>
  );
}

export function Modal({
  title,
  subtitle,
  onClose,
  children,
}: {
  title: string;
  subtitle?: string;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-6 backdrop-blur-sm"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          onClose();
        }
      }}
    >
      <div
        className="sheen w-full max-w-md rounded-card border border-hairline bg-surface
          shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)] p-5"
      >
        <div className="mb-4 flex items-start justify-between gap-4">
          <div>
            <h2 className="text-sm font-semibold">{title}</h2>
            {subtitle && <p className="mt-0.5 text-xs text-muted">{subtitle}</p>}
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-1 text-faint transition-colors hover:bg-surface-2 hover:text-ink"
            aria-label="Close"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
