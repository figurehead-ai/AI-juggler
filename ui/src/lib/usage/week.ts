/**
 * AJ-E2902: the weekly review's week — the last full week, starting on
 * the locale's first day of the week (Monday in most of the world,
 * Sunday in the US), at local midnight.
 */

/** The locale's first day of the week, 1 (Monday) to 7 (Sunday). */
export function firstDayOfWeek(locale: string = typeof navigator === "undefined" ? "en-GB" : navigator.language): number {
  try {
    const info = new Intl.Locale(locale) as Intl.Locale & { getWeekInfo?: () => { firstDay: number }; weekInfo?: { firstDay: number } };
    const day = info.getWeekInfo?.().firstDay ?? info.weekInfo?.firstDay;
    if (day && day >= 1 && day <= 7) return day;
  } catch {
    // An odd locale: Monday.
  }
  return /-US$|^en-US/.test(locale) ? 7 : 1;
}

/** The last full week before `now`: `[start, end)` epoch ms, and the week before it. */
export function lastFullWeek(now: number, firstDay: number): { start: number; end: number; before: number } {
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  // JS days: 0 Sunday … 6 Saturday; firstDay: 1 Monday … 7 Sunday.
  const back = (today.getDay() - (firstDay % 7) + 7) % 7;
  const thisWeek = new Date(today);
  thisWeek.setDate(today.getDate() - back);
  const start = new Date(thisWeek);
  start.setDate(thisWeek.getDate() - 7);
  const before = new Date(start);
  before.setDate(start.getDate() - 7);
  return { start: start.getTime(), end: thisWeek.getTime(), before: before.getTime() };
}

/** "27 Oct – 2 Nov". */
export function weekWords(start: number, end: number): string {
  const last = new Date(end - 1);
  const format = (at: Date) => at.toLocaleDateString("en-GB", { day: "numeric", month: "short" });
  return `${format(new Date(start))} – ${format(last)}`;
}
