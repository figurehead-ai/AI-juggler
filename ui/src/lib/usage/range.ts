/**
 * 0.1.5 — the usage ranges (24 hours, 7, 30 and 90 days, all) cut into
 * buckets in the user's own time zone: an hour, six hours, a day, a week
 * or a month, each starting where the user's clock says (a day at local
 * midnight, across summer time too). The backend sums into these edges.
 */

export type RangeId = "24h" | "7d" | "30d" | "90d" | "all";
export type Step = "hour" | "6h" | "day" | "week" | "month";

export const RANGES: { id: RangeId; label: string; before: string; spoken: string }[] = [
  { id: "24h", label: "24 h", before: "the 24 hours before", spoken: "the last 24 hours" },
  { id: "7d", label: "7 days", before: "the 7 days before", spoken: "the last 7 days" },
  { id: "30d", label: "30 days", before: "the 30 days before", spoken: "the last 30 days" },
  { id: "90d", label: "90 days", before: "the 90 days before", spoken: "the last 90 days" },
  { id: "all", label: "All", before: "the same span before", spoken: "all your usage" },
];

export type Cut = { edges: number[]; step: Step };

const DAY = 86_400_000;

function startOfDay(at: number): Date {
  const date = new Date(at);
  date.setHours(0, 0, 0, 0);
  return date;
}

/** Local edges from `start` stepping by `next` until past `end`. */
function walk(start: Date, end: number, next: (date: Date) => void): number[] {
  const edges: number[] = [];
  const date = new Date(start);
  while (date.getTime() < end && edges.length < 401) {
    edges.push(date.getTime());
    next(date);
  }
  edges.push(date.getTime());
  return edges;
}

/** The range's buckets, ending with the current one (now is inside it). */
export function cutRange(range: RangeId, now: number, firstMs: number | null = null): Cut {
  const hourStart = new Date(now);
  hourStart.setMinutes(0, 0, 0);
  switch (range) {
    case "24h": {
      const start = new Date(hourStart);
      start.setHours(start.getHours() - 23);
      return { edges: walk(start, now + 1, (d) => d.setHours(d.getHours() + 1)), step: "hour" };
    }
    case "7d": {
      const start = startOfDay(now);
      start.setDate(start.getDate() - 6);
      return { edges: walk(start, now + 1, (d) => d.setHours(d.getHours() + 6)), step: "6h" };
    }
    case "30d":
    case "90d": {
      const start = startOfDay(now);
      start.setDate(start.getDate() - (range === "30d" ? 29 : 89));
      return { edges: walk(start, now + 1, (d) => d.setDate(d.getDate() + 1)), step: "day" };
    }
    case "all": {
      const first = Math.min(firstMs ?? now - 29 * DAY, now - 29 * DAY);
      const span = now - first;
      if (span <= 120 * DAY) {
        return { edges: walk(startOfDay(first), now + 1, (d) => d.setDate(d.getDate() + 1)), step: "day" };
      }
      if (span <= 2 * 365 * DAY) {
        const start = startOfDay(first);
        start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
        return { edges: walk(start, now + 1, (d) => d.setDate(d.getDate() + 7)), step: "week" };
      }
      const start = startOfDay(first);
      start.setDate(1);
      return { edges: walk(start, now + 1, (d) => d.setMonth(d.getMonth() + 1)), step: "month" };
    }
  }
}

const day = (at: number) => new Date(at).toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
const time = (at: number) => new Date(at).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });

/** A bucket in words, for tooltips and the table ("Thu, Oct 17"). */
export function bucketWords(step: Step): (start: number, end: number) => string {
  switch (step) {
    case "hour":
      return (start) => `${day(start)}, ${time(start)}`;
    case "6h":
      return (start, end) => `${day(start)}, ${time(start)}–${time(end)}`;
    case "day":
      return (start) => day(start);
    case "week":
      return (start) => `Week of ${new Date(start).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })}`;
    case "month":
      return (start) => new Date(start).toLocaleDateString(undefined, { month: "long", year: "numeric" });
  }
}

/** A tick's short words ("14:00", "Oct 17", "Oct"). */
export function tickWords(step: Step): (start: number) => string {
  switch (step) {
    case "hour":
      return time;
    case "6h":
      return (start) => new Date(start).toLocaleDateString(undefined, { weekday: "short", day: "numeric" });
    case "day":
    case "week":
      return (start) => new Date(start).toLocaleDateString(undefined, { day: "numeric", month: "short" });
    case "month":
      return (start) => new Date(start).toLocaleDateString(undefined, { month: "short", year: "2-digit" });
  }
}

/** What a bucket is, for a chart's subtitle ("per day"). */
export function perWords(step: Step): string {
  return { hour: "per hour", "6h": "per 6 hours", day: "per day", week: "per week", month: "per month" }[step];
}

/** Weekends get a faint band on day charts. */
export function weekend(step: Step): ((start: number) => boolean) | undefined {
  if (step !== "day") return undefined;
  return (start) => {
    const weekday = new Date(start).getDay();
    return weekday === 0 || weekday === 6;
  };
}

/** The local day starts inside `[start, end)`, with their words. */
export function daysBetween(start: number, end: number): { at: number; words: string; long: string }[] {
  const out: { at: number; words: string; long: string }[] = [];
  const date = startOfDay(start);
  while (date.getTime() < end && out.length < 400) {
    if (date.getTime() + DAY > start) {
      out.push({
        at: Math.max(start, date.getTime()),
        words: date.toLocaleDateString(undefined, { weekday: "short" }),
        long: date.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "short" }),
      });
    }
    date.setDate(date.getDate() + 1);
  }
  return out;
}
