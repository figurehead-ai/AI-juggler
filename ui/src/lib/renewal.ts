/**
 * AJ-E411 (D-12): subscription-expiry thresholds — the TS mirror of
 * `crates/juggler-core/src/renewal.rs` (kept in lockstep by boundary
 * tests on both sides). Dates are user-entered local calendar
 * `YYYY-MM-DD` strings; the label is computed against the machine's
 * LOCAL today (JS Date), never timezone-converted.
 *
 * D-12: no date → unknown. Inference from plan tier, usage, or billing
 * cadence is prohibited.
 */

export type RenewalTier = "date" | "countdown" | "warning" | "danger";

export type RenewalLabel = {
  tier: RenewalTier;
  /** The exact display copy. */
  text: string;
};

/** Local calendar days between `YYYY-MM-DD` and the local date of `now`. */
function daysUntil(date: string, now: number): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  if (!match) return null;
  const [, y, m, d] = match;
  const target = Date.UTC(Number(y), Number(m) - 1, Number(d)) / 86_400_000;
  const local = new Date(now);
  const today =
    Date.UTC(local.getFullYear(), local.getMonth(), local.getDate()) / 86_400_000;
  return Math.round(target - today);
}

/**
 * The expiry label for a stored renewal date; `null` when no (valid)
 * date exists — callers render `Expiry: unknown`, never a guess.
 */
export function renewalLabel(date: string | null, now: number): RenewalLabel | null {
  if (date === null) return null;
  const days = daysUntil(date, now);
  if (days === null) return null;
  if (days > 14) return { tier: "date", text: `Renews ${date}` };
  if (days >= 8) return { tier: "countdown", text: `Renews in ${days} days` };
  if (days >= 1) return { tier: "warning", text: `Renews in ${days} days` };
  return { tier: "danger", text: "Renewal date passed" };
}

/** Whether the tier surfaces on the Account row / Needs-me tie-break
 * (Req.2/Req.5): only at 7 days or fewer. */
export function renewalRowVisible(label: RenewalLabel | null): boolean {
  return label !== null && (label.tier === "warning" || label.tier === "danger");
}

/** Rank for the Needs-me tie-break: danger above warning above the rest. */
export function renewalTieBreak(label: RenewalLabel | null): number {
  if (label?.tier === "danger") return 0;
  if (label?.tier === "warning") return 1;
  return 2;
}
