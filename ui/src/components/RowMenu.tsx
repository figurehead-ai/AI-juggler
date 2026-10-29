import { useEffect, useLayoutEffect, useRef, useState, type MouseEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * The row menu (D-25: one pattern everywhere): the ⋯ button or a
 * right-click on a rail row, an account card or a project opens the same
 * small menu — a heading, then plain-word actions. It takes focus, moves
 * with the arrow keys (Home and End too), closes on Escape, Tab or a
 * click outside, and gives focus back to what opened it. AJ-E2106: one
 * level of submenus — → opens one, ← or Escape closes it.
 */
export function RowMenu({
  heading,
  ariaLabel,
  at,
  onClose,
  wide = false,
  minWidth,
  children,
}: {
  /** The small uppercase line on top (what the menu is about); none when
   * the menu's own first lines say it (the user menu). */
  heading: string;
  ariaLabel: string;
  /** Where it opens (viewport px), from {@link menuPoint}. */
  at: { x: number; y: number };
  onClose: () => void;
  /** AJ-E2507: a little wider (the user menu's inline choices). */
  wide?: boolean;
  /** 0.3: at least this wide (a Picker's menu, as wide as its button). */
  minWidth?: number;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  // The parent re-renders on every refresh: the latest close, without
  // re-running the focus setup below.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    // The arrows move within one level: the menu, or the open submenu.
    const levelOf = (element: Element | null): Element | null =>
      (element && ref.current?.contains(element) ? element.closest('[role="menu"]') : null) ?? ref.current;
    const items = (level: Element | null = ref.current) =>
      [...(level?.querySelectorAll<HTMLButtonElement>(MENU_ITEMS) ?? [])].filter(
        (item) => item.closest('[role="menu"]') === level,
      );
    // A choice opens on the chosen one (a Picker, the order menu).
    (items().find((item) => item.getAttribute("aria-checked") === "true" && item.getAttribute("role") === "menuitemradio") ?? items()[0])?.focus();
    const close = (restore: boolean) => {
      closeRef.current();
      if (restore) opener?.focus?.();
    };
    const onDown = (event: globalThis.MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) close(false);
    };
    const onKey = (event: KeyboardEvent) => {
      const active = document.activeElement as HTMLElement | null;
      const level = levelOf(active);
      const inSubmenu = level !== null && level !== ref.current;
      if ((event.key === "Escape" || event.key === "ArrowLeft") && inSubmenu) {
        // Back to the line that opened it, which closes it.
        event.preventDefault();
        const trigger = level.parentElement?.querySelector<HTMLButtonElement>(':scope > [aria-haspopup="menu"]');
        trigger?.focus();
        trigger?.click();
        if (event.key === "Escape") event.stopPropagation();
      } else if (event.key === "Escape") {
        event.preventDefault();
        // 0.3: only the menu closes, never the sheet it opened from (a
        // Picker in a dialog): this listener runs first (capture).
        event.stopPropagation();
        close(true);
      } else if (event.key === "Tab") {
        close(true);
      } else if (event.key === "ArrowRight" && active?.getAttribute("aria-haspopup") === "menu") {
        event.preventDefault();
        if (active.getAttribute("aria-expanded") !== "true") active.click();
        else items(active.parentElement?.querySelector('[role="menu"]') ?? null)[0]?.focus();
      } else if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
        const list = items(level);
        if (list.length === 0) return;
        event.preventDefault();
        const index = list.indexOf(active as HTMLButtonElement);
        const next =
          event.key === "Home"
            ? 0
            : event.key === "End"
              ? list.length - 1
              : event.key === "ArrowDown"
                ? (index + 1) % list.length
                : (index - 1 + list.length) % list.length;
        list[next]?.focus();
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey, true);
    const menu = ref.current;
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey, true);
      // An action ran (or the menu went): focus goes back where it came
      // from, unless the action moved it on (a sheet that opened).
      const active = document.activeElement;
      if (active === null || active === document.body || menu?.contains(active)) opener?.focus?.();
    };
  }, []);
  // Where it fits, measured once it's drawn. Like the Mac's own menus it
  // keeps a corner at the pointer: below and to the right when there's
  // room, else above it (or to its left). Pushed up against the window's
  // edge instead, a menu opened low put the pointer over its middle lines.
  const [style, setStyle] = useState({ left: Math.max(4, Math.min(at.x, window.innerWidth - 260)), top: Math.max(4, at.y) });
  useLayoutEffect(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    setStyle({ left: menuEdge(at.x, box.width, window.innerWidth), top: menuEdge(at.y, box.height, window.innerHeight) });
  }, [at.x, at.y]);
  // On the page itself: a blurred rail is the containing block of
  // anything fixed inside it, which would clip the menu to the rail.
  return createPortal(
    <div
      ref={ref}
      role="menu"
      aria-label={ariaLabel}
      style={minWidth ? { ...style, minWidth } : style}
      className={"fixed z-50 flex max-h-[calc(100vh-8px)] flex-col gap-0.5 overflow-y-auto rounded-xl border border-hairline bg-surface p-1.5 shadow-card " + (wide ? "w-80" : "w-64")}
    >
      {heading && <span className="truncate px-2 pb-1 pt-0.5 text-label font-semibold text-muted">{heading}</span>}
      {children}
    </div>,
    document.body,
  );
}

/** Where a menu of `size` opened at `at` starts along one axis of a
 * window `room` long: at the point when it fits after it, else ending at
 * the point when it fits before it, else as far in as the window allows. */
export function menuEdge(at: number, size: number, room: number): number {
  const start = at + size <= room - 4 ? at : at - size >= 4 ? at - size : room - size - 4;
  // Never off the window, even for a point outside it.
  return Math.max(4, Math.min(start, room - size - 4));
}

/** What the arrows move through: actions, and a choice's options. */
export const MENU_ITEMS = '[role="menuitem"]:not(:disabled), [role="menuitemradio"]:not(:disabled), [role="menuitemcheckbox"]:not(:disabled)';

/** One action in a {@link RowMenu}; with `checked`, one option of a
 * choice (a `menuitemradio`, D-47's order menu), or with `multi` too, a
 * line that turns on and off (a `menuitemcheckbox`, the View's Show). */
export function MenuItem({
  onClick,
  disabled,
  title,
  danger,
  testId,
  describedBy,
  submenu,
  checked,
  multi,
  children,
}: {
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  disabled?: boolean;
  title?: string;
  danger?: boolean;
  testId?: string;
  /** The id of the line that says why it's greyed. */
  describedBy?: string;
  /** It opens a submenu: whether that is open now. */
  submenu?: { open: boolean };
  /** An option of a choice: whether it is the one chosen. */
  checked?: boolean;
  /** With `checked`: one of several that turn on and off. */
  multi?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      role={checked === undefined ? "menuitem" : multi ? "menuitemcheckbox" : "menuitemradio"}
      aria-checked={checked}
      onClick={onClick}
      disabled={disabled}
      title={title}
      data-testid={testId}
      aria-describedby={describedBy}
      aria-haspopup={submenu ? "menu" : undefined}
      aria-expanded={submenu ? submenu.open : undefined}
      className={
        "rounded-lg px-2 py-1 text-left text-meta outline-none transition-colors hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:text-muted disabled:hover:bg-transparent " +
        (danger ? "text-danger" : "text-ink")
      }
    >
      {children}
    </button>
  );
}

/**
 * AJ-E2106 — a submenu (one level, never nested): a line that opens its
 * own small menu beside it, on hover, a click or →; ← or Escape closes
 * it. Drawn fixed beside its line, so the menu's scroll never clips it.
 */
export function MenuSubmenu({ label, line, testId, children }: { label: string; line: ReactNode; testId?: string; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [fromKeys, setFromKeys] = useState(false);
  const wrap = useRef<HTMLDivElement | null>(null);
  const panel = useRef<HTMLDivElement | null>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  useLayoutEffect(() => {
    if (!open) return;
    const row = wrap.current?.getBoundingClientRect();
    if (!row) return;
    const box = panel.current?.getBoundingClientRect();
    const width = box?.width ?? 208;
    const height = box?.height ?? 0;
    const fitsRight = row.right + width - 2 <= window.innerWidth - 4;
    setPlace({
      left: fitsRight ? row.right - 2 : Math.max(4, row.left - width + 2),
      top: Math.max(4, Math.min(row.top - 6, window.innerHeight - height - 4)),
    });
  }, [open]);
  useEffect(() => {
    if (open && fromKeys && place)
      (panel.current?.querySelector<HTMLButtonElement>('[role="menuitemradio"][aria-checked="true"]:not(:disabled)') ??
        panel.current?.querySelector<HTMLButtonElement>(MENU_ITEMS)
      )?.focus();
  }, [open, fromKeys, place]);
  return (
    <div ref={wrap} className="relative flex flex-col" onMouseEnter={() => setOpen(true)} onMouseLeave={() => setOpen(false)}>
      <MenuItem
        testId={testId}
        submenu={{ open }}
        onClick={(event) => {
          setFromKeys(event.detail === 0);
          setOpen((was) => !was);
        }}
      >
        {line}
      </MenuItem>
      {open && (
        <div
          ref={panel}
          role="menu"
          aria-label={label}
          style={place ?? { left: -9999, top: 0 }}
          className="fixed z-50 flex w-max min-w-52 max-w-80 flex-col gap-0.5 rounded-xl border border-hairline bg-surface p-1.5 shadow-card"
        >
          {children}
        </div>
      )}
    </div>
  );
}

/** A line between a menu's groups (AJ-E1537: destructive ones last). */
export function MenuSeparator() {
  return <div role="separator" className="mx-1 my-1 border-t border-hairline" />;
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
