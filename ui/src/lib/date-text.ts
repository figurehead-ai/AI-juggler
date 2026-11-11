/**
 * A8 (the AJ-E3306 rc22 walk): a day typed as text, in the user's own
 * order (11/28/2026 in the US, 28/11/2026 in much of the world), read
 * whole or not at all. The system date control saved a half-typed date
 * on each keystroke (typing 11282026 ended on 12/09/2026) and drew an
 * unset one as today's date; the shared DateField reads text with these
 * and saves only a whole, valid day, on commit.
 *
 * Days are ISO `YYYY-MM-DD` strings, as the backend keeps them.
 */

type Part = "month" | "day" | "year";

export interface DateFormat {
  /** The parts in the user's order. */
  order: [Part, Part, Part];
  /** What goes between them ("/", ".", "-"). */
  sep: string;
}

const WIDTH: Record<Part, number> = { month: 2, day: 2, year: 4 };
const HINT: Record<Part, string> = { month: "mm", day: "dd", year: "yyyy" };

/** The order and separator `locale` writes a numeric date in. */
export function dateFormat(locale?: string): DateFormat {
  const parts = new Intl.DateTimeFormat(locale, { year: "numeric", month: "2-digit", day: "2-digit" }).formatToParts(new Date(2026, 10, 28));
  const order = parts.flatMap((part) => (part.type === "month" || part.type === "day" || part.type === "year" ? [part.type] : []));
  const sep = parts.find((part) => part.type === "literal")?.value.trim() || "/";
  const [a, b, c] = order;
  return a && b && c && new Set(order).size === 3 ? { order: [a, b, c], sep } : { order: ["month", "day", "year"], sep: "/" };
}

/** How it's typed, for a hint: "mm/dd/yyyy". */
export function datePattern(format: DateFormat): string {
  return format.order.map((part) => HINT[part]).join(format.sep);
}

/** Day `iso` as the user writes it ("11/28/2026"); "" for none. */
export function formatDate(iso: string | null | undefined, format: DateFormat): string {
  const match = iso ? /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso) : null;
  if (!match) return "";
  const value: Record<Part, string> = { year: match[1]!, month: match[2]!, day: match[3]! };
  return format.order.map((part) => value[part]).join(format.sep);
}

/** An ISO day from its numbers; `null` when there is no such day. */
export function isoDay(year: number, month: number, day: number): string | null {
  if (year < 1000 || year > 9999 || month < 1 || month > 12 || day < 1) return null;
  if (day > new Date(year, month, 0).getDate()) return null;
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

type DateRead =
  | { kind: "empty" }
  /** A whole, real day. */
  | { kind: "date"; iso: string }
  /** Still being typed: too few digits for a whole day. */
  | { kind: "partial" }
  /** Whole, but no such day (02/31/2026). */
  | { kind: "invalid" };

/** What `text` says: a day only when it is whole and real. Digits alone
 * (11282026) read in the user's order, two at a time and four for the
 * year, as the system control took them; a year first, four digits, is
 * the ISO order. */
export function readDate(text: string, format: DateFormat): DateRead {
  const trimmed = text.trim();
  if (trimmed === "") return { kind: "empty" };
  if (/[^\d\s./-]/.test(trimmed)) return { kind: "invalid" };
  let pieces = trimmed.split(/[\s./-]+/).filter(Boolean);
  if (pieces.length === 1) {
    const digits = pieces[0]!;
    if (digits.length < 8) return { kind: "partial" };
    if (digits.length > 8) return { kind: "invalid" };
    let at = 0;
    pieces = format.order.map((part) => {
      const piece = digits.slice(at, at + WIDTH[part]);
      at += WIDTH[part];
      return piece;
    });
  }
  if (pieces.length < 3) return { kind: "partial" };
  if (pieces.length > 3) return { kind: "invalid" };
  // A year first, four digits, is the ISO order (2026-11-28, pasted).
  const order: readonly Part[] = pieces[0]!.length === 4 && format.order[0] !== "year" ? ["year", "month", "day"] : format.order;
  const of = (part: Part) => pieces[order.indexOf(part)] ?? "";
  const value: Record<Part, string> = { year: of("year"), month: of("month"), day: of("day") };
  if (value.year.length < 4) return { kind: "partial" };
  if (value.year.length > 4 || value.month.length > 2 || value.day.length > 2) return { kind: "invalid" };
  const iso = isoDay(Number(value.year), Number(value.month), Number(value.day));
  return iso ? { kind: "date", iso } : { kind: "invalid" };
}

/** Text typed so far, with the separator put in once a part is whole
 * ("1128" → "11/28/"), as the system control moved on by itself. Only
 * text on that shape's way is shaped ("11/2" + "8", "11/282"); a date
 * typed with its own separators elsewhere ("1/5/2026") is left as typed. */
export function shapeTyped(text: string, format: DateFormat): string {
  const pieces = text.split(format.sep);
  const digits = pieces.join("");
  if (!/^\d+$/.test(digits) || pieces.length > 3) return text;
  // Separators only after whole parts: the shape's own, or none.
  if (pieces.slice(0, -1).some((piece, index) => piece.length !== WIDTH[format.order[index]!])) return text;
  let shaped = "";
  let at = 0;
  for (const [index, part] of format.order.entries()) {
    const piece = digits.slice(at, at + WIDTH[part]);
    at += WIDTH[part];
    shaped += piece;
    if (piece.length < WIDTH[part]) break;
    if (index < 2) shaped += format.sep;
  }
  return shaped + digits.slice(at);
}
