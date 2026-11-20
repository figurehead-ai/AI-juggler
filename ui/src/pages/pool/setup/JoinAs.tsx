/**
 * 0.4.6 (AJ-E3011 §1, §2): the old "Everyone in this pool's projects" row
 * as a labelled control: the role of the pool's everyone-in-scope member,
 * or not in the pool (only who's in). The setup's "Everyone else" step and
 * the inspector share it.
 *
 * SK-12: it read "New sessions in web join as", but that member is every
 * session in scope, the ones in already too: Reviewer turned 59 sessions
 * into Reviewers. So it says so, "Everyone in web, now and new, is:
 * [Reviewer ▾] · 59 sessions now", with how many a change moves at once.
 */
import type { JSX } from "react";

import { Picker } from "../../../components/controls";
import type { PoolRole } from "../../../lib/ipc/pools";
import { ROLE_CARDS } from "../../../lib/pool-words";

/** The join control's choices: a role, or nobody else joins. */
const JOIN: readonly { value: PoolRole | "none"; label: string; hint: string }[] = [
  ...ROLE_CARDS.filter((card) => card.id !== "lead").map((card) => ({ value: card.id, label: card.label, hint: card.line })),
  { value: "none", label: "Not in the pool", hint: "Only who's in. Other sessions in these projects stay out." },
];

export function JoinAs({
  where,
  value,
  onChange,
  disabled = false,
  count = null,
  testId = "join-as",
}: {
  where: string;
  value: PoolRole | null;
  /** The sessions in scope now, whose role a change sets at once. */
  count?: number | null;
  onChange: (role: PoolRole | null) => void;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" data-testid={testId}>
      <span className="flex flex-wrap items-center gap-2 text-meta font-medium text-ink">
        Everyone in {where}, now and new, is:
        <Picker
          choice
          size="sm"
          ariaLabel={`Everyone in ${where}, now and new, is`}
          heading={`Everyone in ${where}, now and new, is`}
          value={value ?? "none"}
          disabled={disabled}
          options={JOIN}
          onChange={(next) => onChange(next === "none" ? null : next)}
          testId={`${testId}-picker`}
        />
        {value !== null && count !== null && (
          <span className="font-normal text-muted" data-testid={`${testId}-count`}>
            · {count} {count === 1 ? "session" : "sessions"} now
          </span>
        )}
      </span>
    </div>
  );
}

