/**
 * A8: a day typed as text is read whole or not at all, in the user's own
 * order; digits alone read as the system control took them.
 */
import { describe, expect, it } from "vitest";
import { dateFormat, datePattern, formatDate, readDate, shapeTyped, type DateFormat } from "./date-text";

const US: DateFormat = { order: ["month", "day", "year"], sep: "/" };
const DE: DateFormat = { order: ["day", "month", "year"], sep: "." };

describe("A8 a typed date", () => {
  it("reads the user's own order from the locale", () => {
    expect(dateFormat("en-US")).toEqual(US);
    expect(dateFormat("de-DE")).toEqual(DE);
    expect(datePattern(US)).toBe("mm/dd/yyyy");
    expect(datePattern(DE)).toBe("dd.mm.yyyy");
  });

  it("is a day only when whole and real", () => {
    expect(readDate("11282026", US)).toEqual({ kind: "date", iso: "2026-11-28" });
    expect(readDate("11/28/2026", US)).toEqual({ kind: "date", iso: "2026-11-28" });
    expect(readDate("1/5/2026", US)).toEqual({ kind: "date", iso: "2026-01-05" });
    expect(readDate("28.11.2026", DE)).toEqual({ kind: "date", iso: "2026-11-28" });
    expect(readDate("2026-11-28", US)).toEqual({ kind: "date", iso: "2026-11-28" });
    expect(readDate("", US)).toEqual({ kind: "empty" });
    for (const partial of ["1", "11", "1128", "11/28", "11/28/", "11/28/202", "1128202"]) expect(readDate(partial, US)).toEqual({ kind: "partial" });
    for (const wrong of ["02/30/2026", "13/01/2026", "11/28/20266", "11-28-2026x"]) expect(readDate(wrong, US)).toEqual({ kind: "invalid" });
  });

  it("puts the separator in as each part is whole, and leaves its own alone", () => {
    expect(shapeTyped("11", US)).toBe("11/");
    expect(shapeTyped("112", US)).toBe("11/2");
    expect(shapeTyped("11/28", US)).toBe("11/28/");
    expect(shapeTyped("11/28/2026", US)).toBe("11/28/2026");
    expect(shapeTyped("1/5", US)).toBe("1/5");
    expect(shapeTyped("2811", DE)).toBe("28.11.");
  });

  it("is written back in the same order", () => {
    expect(formatDate("2026-11-28", US)).toBe("11/28/2026");
    expect(formatDate("2026-11-28", DE)).toBe("28.11.2026");
    expect(formatDate(null, US)).toBe("");
  });
});
