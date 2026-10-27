/**
 * Numbers in a few characters, the one place that picks a unit: counts
 * ("940", "12.3k", "1.2M", "10.3B"), file and disk sizes ("512 B", "3 KB",
 * "640 MB", "3.2 GB") and memory ("8.6 GB of 32 GB"). A number that rounds
 * up to the next unit reads in that unit: 999,999 tokens is "1.0M", never
 * "1000.0k" (0.1.8: the Prices sheet's "1000k"), and 999.6 MB is "1.0 GB".
 */

/** A unit: its size and how it is written after the number. */
type Unit = { size: number; suffix: string };

/** Counts, largest first. */
const COUNT_UNITS: Unit[] = [
  { size: 1_000_000_000, suffix: "B" },
  { size: 1_000_000, suffix: "M" },
  { size: 1_000, suffix: "k" },
];

/**
 * Byte sizes, largest first, in decimal units as Finder and the backend's
 * This Mac words count them (`juggler_supervisor::machine::bytes_words`).
 */
const BYTE_UNITS: Unit[] = [
  { size: 1e12, suffix: " TB" },
  { size: 1e9, suffix: " GB" },
  { size: 1e6, suffix: " MB" },
  { size: 1e3, suffix: " KB" },
];

/**
 * Memory, largest first, in the units macOS counts it in (1,024-based, as
 * Activity Monitor and "32 GB of memory" do): a 32 GB Mac reads 32 GB, not
 * 34.4 (0.1.8: This Mac said "103 GB" for a 96 GB Mac).
 */
const MEMORY_UNITS: Unit[] = [
  { size: 1024 ** 4, suffix: " TB" },
  { size: 1024 ** 3, suffix: " GB" },
  { size: 1024 ** 2, suffix: " MB" },
  { size: 1024, suffix: " KB" },
];

/** Bytes, KB and MB whole; GB and TB with one decimal under 10. */
const sizeDigits = (value: number, suffix: string): number => ((suffix === " GB" || suffix === " TB") && value < 10 ? 1 : 0);

/**
 * `n` in the largest of `units` it reaches, with `digits(value, suffix)`
 * decimals; decimals and unit are both picked on the value as rounded.
 * Below the smallest unit: the whole number with `below`.
 */
function compactIn(n: number, units: Unit[], below: string, digits: (value: number, suffix: string) => number): string {
  const inUnit = (index: number): string => {
    const { size, suffix } = units[index]!;
    const value = n / size;
    // Decimals for the value as shown: 9.99M rounds to 10.0, shown "10".
    const rounded = Number(value.toFixed(digits(value, suffix)));
    return `${value.toFixed(digits(rounded, suffix))}${suffix}`;
  };
  const index = units.findIndex(({ size }) => n >= size);
  if (index === -1) {
    const whole = Math.round(n);
    // 999.6 rounds to a thousand: one of the smallest unit.
    return whole >= 1_000 && units.length > 0 ? inUnit(units.length - 1) : `${whole}${below}`;
  }
  const shown = inUnit(index);
  // Rounded up to a thousand of this unit: one of the next unit up.
  return index > 0 && parseFloat(shown) >= 1_000 ? inUnit(index - 1) : shown;
}

/**
 * A count in its unit, with `digits(value, suffix)` decimals for that
 * unit's value (e.g. one decimal under 100, none from 100 on). Under
 * 1,000 it is the whole number.
 */
export function compactCount(n: number, digits: (value: number, suffix: string) => number): string {
  return compactIn(n, COUNT_UNITS, "", digits);
}

/**
 * A file or disk size, in decimal units as Finder counts them (and the
 * backend's `bytes_words`): "640 MB", "3.2 GB", "24 GB".
 */
export function byteSize(bytes: number): string {
  return compactIn(Math.max(0, bytes), BYTE_UNITS, " B", sizeDigits);
}

/**
 * Memory, in the units macOS counts it in (as the backend's
 * `memory_words`): "8.6 GB", "640 MB", "96 GB".
 */
export function memorySize(bytes: number): string {
  return compactIn(Math.max(0, bytes), MEMORY_UNITS, " B", sizeDigits);
}
