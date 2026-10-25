/**
 * 0.1.5 — the usage ranges cut in the user's own time zone: each bucket
 * starts where the user's clock says, the current one holds now, and no
 * range asks for more than 400 buckets.
 */
import { describe, expect, it } from "vitest";
import { cutRange, daysBetween } from "./range";

const NOW = new Date(2026, 9, 23, 14, 40).getTime();

describe("usage ranges", () => {
  it("24 hours: hourly buckets ending with the current hour", () => {
    const { edges, step } = cutRange("24h", NOW);
    expect(step).toBe("hour");
    expect(edges.length).toBe(25);
    expect(edges[edges.length - 2]).toBeLessThanOrEqual(NOW);
    expect(edges[edges.length - 1]).toBeGreaterThan(NOW);
    expect(new Date(edges[0]!).getMinutes()).toBe(0);
  });

  it("7 days: six-hour buckets from local midnight", () => {
    const { edges, step } = cutRange("7d", NOW);
    expect(step).toBe("6h");
    expect(new Date(edges[0]!).getHours()).toBe(0);
    expect(edges.length - 1).toBe(6 * 4 + 3);
  });

  it("30 and 90 days: one bucket per local day, today last", () => {
    const month = cutRange("30d", NOW);
    expect(month.step).toBe("day");
    expect(month.edges.length).toBe(31);
    expect(new Date(month.edges[29]!).getDate()).toBe(23);
    expect(cutRange("90d", NOW).edges.length).toBe(91);
  });

  it("all: days, then weeks, then months, never over 400 buckets", () => {
    expect(cutRange("all", NOW, NOW - 60 * 86_400_000).step).toBe("day");
    expect(cutRange("all", NOW, NOW - 400 * 86_400_000).step).toBe("week");
    const years = cutRange("all", NOW, NOW - 12 * 365 * 86_400_000);
    expect(years.step).toBe("month");
    expect(years.edges.length).toBeLessThanOrEqual(401);
    expect(years.edges.every((edge, i) => i === 0 || edge > years.edges[i - 1]!)).toBe(true);
  });

  it("the days between two instants, with their words", () => {
    const days = daysBetween(NOW - 3 * 86_400_000, NOW);
    expect(days.length).toBe(4);
    expect(days[0]!.at).toBe(NOW - 3 * 86_400_000);
  });
});
