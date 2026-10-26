/**
 * A count in a few characters ("940", "12.3k", "1.2M", "10.3B"), the one
 * place that picks the unit. A count that rounds up to the next unit reads
 * in that unit: 999,999 is "1.0M", never "1000.0k" (0.1.7: the Prices
 * sheet's "1000k").
 */

/** Largest first. */
const UNITS: { size: number; suffix: string }[] = [
  { size: 1_000_000_000, suffix: "B" },
  { size: 1_000_000, suffix: "M" },
  { size: 1_000, suffix: "k" },
];

/**
 * `n` in its unit, with `digits(value, suffix)` decimals for that unit's
 * value (e.g. one decimal under 100, none from 100 on). Under 1,000 it is
 * the whole number.
 */
export function compactCount(n: number, digits: (value: number, suffix: string) => number): string {
  const inUnit = (index: number): string => {
    const { size, suffix } = UNITS[index]!;
    const value = n / size;
    // Decimals for the value as shown: 9.99M rounds to 10.0, shown "10".
    const rounded = Number(value.toFixed(digits(value, suffix)));
    return `${value.toFixed(digits(rounded, suffix))}${suffix}`;
  };
  const index = UNITS.findIndex(({ size }) => n >= size);
  if (index === -1) return String(Math.round(n));
  const shown = inUnit(index);
  // Rounded up to a thousand of this unit: one of the next unit up.
  return index > 0 && parseFloat(shown) >= 1_000 ? inUnit(index - 1) : shown;
}
