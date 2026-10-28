/**
 * The first read of the tools' history, in words (the 0.2 walk, F6: for
 * about ten minutes after the first start every screen said "0 tokens"
 * and "Not measured yet" while eight sessions worked). While it runs the
 * backend says so (`UsageHistoryInfo.reading`,
 * `QuotaDisplayData.history_reading`), and every screen says this instead
 * of a zero; what it has found shows as soon as it's done.
 */

/** In place of a figure that is still coming. */
export const READING_SHORT = "Reading your history…";

/** Under a limit that isn't read yet. */
export const READING_LIMITS = "Reading your history: limits show when it's done";

/** The line over a page whose figures are still coming. */
export function readingLine(conversations: number): string {
  const read = conversations === 1 ? "1 conversation" : `${conversations.toLocaleString("en-US")} conversations`;
  return `Reading your history for the first time: ${read} so far. The figures fill in when it's done, usually within a few minutes.`;
}

/** How often a page re-reads while the first read runs, ms. */
export const READING_POLL_MS = 5_000;
