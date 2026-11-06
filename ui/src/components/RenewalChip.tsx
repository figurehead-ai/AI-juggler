/**
 * AJ-E411 — a renewal 7 days off or fewer, as a coloured chip: ⚠ in the
 * warning colour, ⛔ in the danger colour once the date has passed (an
 * icon and words, never colour alone). Nothing for a date further off: the
 * line beside it says that in words. The Accounts card and the Account
 * page's header both show it.
 */
import type { JSX } from "react";
import { Chip } from "./primitives";
import { renewalRowVisible, type RenewalLabel } from "../lib/renewal";

export function RenewalChip({ renewal }: { renewal: RenewalLabel | null }): JSX.Element | null {
  if (!renewal || !renewalRowVisible(renewal)) return null;
  return (
    <span data-testid="renewal-chip" data-tier={renewal.tier}>
      <Chip tone={renewal.tier === "danger" ? "danger" : "warning"}>
        {renewal.tier === "danger" ? "⛔" : "⚠"} {renewal.text}
      </Chip>
    </span>
  );
}
