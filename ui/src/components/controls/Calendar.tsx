/**
 * A8 — the shared DateField's month: a day picked from it is saved at
 * once. Keyboard as the APG date picker dialog's: the arrows move a day or
 * a week, Page Up/Down a month (with Shift a year), Home/End the week's
 * ends, Enter or Space picks, Escape closes it and gives focus back.
 */
import { useEffect, useId, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { useAnchoredPlace } from "../RowMenu";
import { PRIMARY_FILL } from "../primitives";
import { IconChevronLeft, IconChevronRight } from "../icons-controls";
import { isoDay } from "../../lib/date-text";

const pad = (n: number) => String(n).padStart(2, "0");
const isoOf = (date: Date) => `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
const dateOf = (iso: string) => {
  const [y = 1970, m = 1, d = 1] = iso.split("-").map(Number);
  return new Date(y, m - 1, d);
};
const plusDays = (iso: string, days: number) => {
  const date = dateOf(iso);
  date.setDate(date.getDate() + days);
  return isoOf(date);
};
const plusMonths = (iso: string, months: number) => {
  const date = dateOf(iso);
  const day = date.getDate();
  date.setDate(1);
  date.setMonth(date.getMonth() + months);
  const last = new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
  date.setDate(Math.min(day, last));
  return isoOf(date);
};

/** The user's first day of the week (0 Sunday … 6 Saturday). */
function firstWeekday(): number {
  try {
    const locale = new Intl.Locale(navigator.language) as Intl.Locale & { getWeekInfo?: () => { firstDay: number }; weekInfo?: { firstDay: number } };
    const day = locale.getWeekInfo?.().firstDay ?? locale.weekInfo?.firstDay;
    return day === undefined ? 0 : day % 7;
  } catch {
    return 0;
  }
}

/** A month to pick a day from, below the field (above when there's no
 * room), on the page itself (a portal, as a menu is). */
export function Calendar({
  at,
  value,
  min,
  max,
  testId,
  onPick,
  onClose,
}: {
  at: { x: number; y: number; above: number; right: number };
  value: string | null;
  min?: string;
  max?: string;
  testId?: string;
  onPick: (iso: string) => void;
  onClose: (restore: boolean) => void;
}): JSX.Element {
  const ref = useRef<HTMLDialogElement | null>(null);
  const [today] = useState(() => isoOf(new Date()));
  const [focus, setFocus] = useState(() => value ?? (min && today < min ? min : max && today > max ? max : today));
  const style = useAnchoredPlace(ref, at);
  const titleId = `${useId().replace(/:/g, "")}-month`;
  const closeRef = useRef(onClose);
  useEffect(() => {
    closeRef.current = onClose;
  });
  const outside = (iso: string) => (min !== undefined && iso < min) || (max !== undefined && iso > max);
  // The focused day takes focus as it moves (a roving tab stop).
  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>(`[data-day="${focus}"]`)?.focus();
  }, [focus]);
  useEffect(() => {
    const panel = ref.current;
    const onDown = (event: MouseEvent) => {
      if (panel && event.target instanceof Node && !panel.contains(event.target)) closeRef.current(false);
    };
    // A press inside is never "outside" to the sheet it opened from (the
    // switch's dialog): a menu's rule (RowMenu).
    const inside = (event: MouseEvent) => event.stopPropagation();
    // Leaving it by Tab closes it, as a menu does.
    const onLeave = (event: FocusEvent) => {
      if (event.relatedTarget instanceof Node && !panel?.contains(event.relatedTarget)) closeRef.current(false);
    };
    window.addEventListener("mousedown", onDown);
    panel?.addEventListener("mousedown", inside);
    panel?.addEventListener("focusout", onLeave);
    return () => {
      window.removeEventListener("mousedown", onDown);
      panel?.removeEventListener("mousedown", inside);
      panel?.removeEventListener("focusout", onLeave);
    };
  }, []);
  const first = firstWeekday();
  const shown = dateOf(focus);
  const year = shown.getFullYear();
  const month = shown.getMonth();
  const lead = (new Date(year, month, 1).getDay() - first + 7) % 7;
  const days = new Date(year, month + 1, 0).getDate();
  const cells: (string | null)[] = [...Array.from({ length: lead }, () => null), ...Array.from({ length: days }, (_, i) => isoDay(year, month + 1, i + 1))];
  while (cells.length % 7 !== 0) cells.push(null);
  const weeks = Array.from({ length: cells.length / 7 }, (_, i) => cells.slice(i * 7, i * 7 + 7));
  const weekdays = Array.from({ length: 7 }, (_, i) => {
    const day = new Date(2026, 10, 1 + ((first + i) % 7)); // 1 Nov 2026 is a Sunday
    return { short: day.toLocaleDateString([], { weekday: "narrow" }), long: day.toLocaleDateString([], { weekday: "long" }) };
  });
  const title = shown.toLocaleDateString([], { month: "long", year: "numeric" });
  const onKey = (event: KeyboardEvent<HTMLButtonElement>) => {
    const weekday = (dateOf(focus).getDay() - first + 7) % 7;
    const moves: Partial<Record<string, () => string>> = {
      ArrowLeft: () => plusDays(focus, -1),
      ArrowRight: () => plusDays(focus, 1),
      ArrowUp: () => plusDays(focus, -7),
      ArrowDown: () => plusDays(focus, 7),
      PageUp: () => plusMonths(focus, event.shiftKey ? -12 : -1),
      PageDown: () => plusMonths(focus, event.shiftKey ? 12 : 1),
      Home: () => plusDays(focus, -weekday),
      End: () => plusDays(focus, 6 - weekday),
    };
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose(true);
      return;
    }
    const move = moves[event.key];
    if (move && event.target instanceof HTMLElement && event.target.dataset.day) {
      event.preventDefault();
      setFocus(move());
    }
  };
  return createPortal(
    // Not modal: the page stays as it is; a press outside or Tab away
    // closes it, as a menu does.
    <dialog
      ref={ref}
      open
      aria-labelledby={titleId}
      style={style}
      className="fixed z-50 m-0 flex w-64 flex-col gap-2 rounded-xl border border-hairline bg-surface p-2.5 text-ink shadow-card"
      data-testid={testId}
    >
      <div className="flex items-center justify-between gap-2">
        <button type="button" aria-label="Previous month" onKeyDown={onKey} onClick={() => setFocus(plusMonths(focus, -1))} className="flex size-7 items-center justify-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          <IconChevronLeft width={16} height={16} aria-hidden="true" />
        </button>
        <h2 id={titleId} aria-live="polite" className="text-meta font-semibold text-ink">
          {title}
        </h2>
        <button type="button" aria-label="Next month" onKeyDown={onKey} onClick={() => setFocus(plusMonths(focus, 1))} className="flex size-7 items-center justify-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          <IconChevronRight width={16} height={16} aria-hidden="true" />
        </button>
      </div>
      <table aria-labelledby={titleId} className="w-full border-collapse text-center">
        <thead>
          <tr>
            {weekdays.map((day) => (
              <th key={day.long} scope="col" abbr={day.long} className="pb-1 text-label font-medium text-muted">
                {day.short}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {weeks.map((week, row) => (
            <tr key={row}>
              {week.map((iso, column) => {
                if (!iso) return <td key={column} aria-hidden="true" />;
                const chosen = iso === value;
                const off = outside(iso);
                return (
                  <td key={iso}>
                    <button
                      type="button"
                      tabIndex={iso === focus ? 0 : -1}
                      disabled={off}
                      aria-label={dateOf(iso).toLocaleDateString([], { weekday: "long", month: "long", day: "numeric", year: "numeric" })}
                      aria-current={iso === today ? "date" : undefined}
                      aria-pressed={chosen}
                      data-day={iso}
                      onKeyDown={onKey}
                      onClick={() => onPick(iso)}
                      className={
                        "size-8 rounded-lg text-xs tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-not-allowed disabled:text-muted/50 " +
                        (chosen ? `${PRIMARY_FILL} font-semibold` : iso === today ? "font-semibold text-accent-hi hover:bg-surface-2" : "text-ink hover:bg-surface-2")
                      }
                    >
                      {Number(iso.slice(8))}
                    </button>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </dialog>,
    document.body,
  );
}
