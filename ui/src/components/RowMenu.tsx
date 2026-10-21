import { useEffect, useRef, type MouseEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * The row menu (D-25: one pattern everywhere): the ⋯ button or a
 * right-click on a rail row, an account card or a project opens the same
 * small menu — a heading, then plain-word actions. It takes focus, moves
 * with the arrow keys, closes on Escape, Tab or a click outside, and
 * gives focus back to what opened it.
 */
export function RowMenu({
  heading,
  ariaLabel,
  at,
  onClose,
  children,
}: {
  /** The small uppercase line on top (what the menu is about). */
  heading: string;
  ariaLabel: string;
  /** Where it opens (viewport px), from {@link menuPoint}. */
  at: { x: number; y: number };
  onClose: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  // The parent re-renders on every refresh: the latest close, without
  // re-running the focus setup below.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    const items = () => [...(ref.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [])];
    items()[0]?.focus();
    const close = (restore: boolean) => {
      closeRef.current();
      if (restore) opener?.focus?.();
    };
    const onDown = (event: globalThis.MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) close(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close(true);
      } else if (event.key === "Tab") {
        close(true);
      } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        const list = items();
        if (list.length === 0) return;
        event.preventDefault();
        const index = list.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === "ArrowDown" ? (index + 1) % list.length : (index - 1 + list.length) % list.length;
        list[next]?.focus();
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    const menu = ref.current;
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
      // An action ran (or the menu went): focus goes back where it came
      // from, unless the action moved it on (a sheet that opened).
      const active = document.activeElement;
      if (active === null || active === document.body || menu?.contains(active)) opener?.focus?.();
    };
  }, []);
  const style = {
    left: Math.max(4, Math.min(at.x, window.innerWidth - 190)),
    top: Math.max(4, Math.min(at.y, window.innerHeight - 190)),
  };
  // On the page itself: a blurred rail is the containing block of
  // anything fixed inside it, which would clip the menu to the rail.
  return createPortal(
    <div
      ref={ref}
      role="menu"
      aria-label={ariaLabel}
      style={style}
      className="fixed z-50 flex w-44 flex-col gap-0.5 rounded-xl border border-hairline bg-surface p-1.5 shadow-card"
    >
      <span className="truncate px-2 pb-1 pt-0.5 text-[9.5px] font-medium uppercase tracking-wide text-faint">{heading}</span>
      {children}
    </div>,
    document.body,
  );
}

/** One action in a {@link RowMenu}. */
export function MenuItem({
  onClick,
  disabled,
  title,
  danger,
  testId,
  children,
}: {
  onClick: () => void;
  disabled?: boolean;
  title?: string;
  danger?: boolean;
  testId?: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      disabled={disabled}
      title={title}
      data-testid={testId}
      className={
        "rounded-lg px-2 py-1 text-left text-[11.5px] outline-none transition-colors hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:text-faint disabled:hover:bg-transparent " +
        (danger ? "text-danger" : "text-ink")
      }
    >
      {children}
    </button>
  );
}

/** Where a menu opens: at the pointer, or under its button when opened
 * from the keyboard (a keyboard click has no pointer position). */
export function menuPoint(event: MouseEvent<HTMLElement>): { x: number; y: number } {
  if (event.detail === 0 || (event.clientX === 0 && event.clientY === 0)) {
    const box = event.currentTarget.getBoundingClientRect();
    return { x: box.left, y: box.bottom + 4 };
  }
  return { x: event.clientX, y: event.clientY };
}
