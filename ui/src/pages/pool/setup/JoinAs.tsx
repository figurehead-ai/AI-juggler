/**
 * 0.4.6 (AJ-E3011 §1, §2): "New sessions in web join as [Member ▾]", the
 * old "Everyone in this pool's projects" row as a labelled control: the
 * role of the pool's everyone-in-scope member, or "Don't join" (only
 * who's in). The setup's "Everyone else" step and the inspector share it.
 */
import type { JSX } from "react";

import { Picker } from "../../../components/controls";
import type { PoolRole } from "../../../lib/ipc/pools";
import { ROLE_CARDS } from "../../../lib/pool-words";

/** The join control's choices: a role, or nobody else joins. */
const JOIN: readonly { value: PoolRole | "none"; label: string; hint: string }[] = [
  ...ROLE_CARDS.filter((card) => card.id !== "lead").map((card) => ({ value: card.id, label: card.label, hint: card.line })),
  { value: "none", label: "Don't join", hint: "Only who's in. Other sessions in these projects stay out." },
];

export function JoinAs({
  where,
  value,
  onChange,
  disabled = false,
  testId = "join-as",
}: {
  where: string;
  value: PoolRole | null;
  onChange: (role: PoolRole | null) => void;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" data-testid={testId}>
      <span className="flex flex-wrap items-center gap-2 text-meta font-medium text-ink">
        New sessions in {where} join as
        <Picker
          choice
          size="sm"
          ariaLabel={`New sessions in ${where} join as`}
          heading={`New sessions in ${where} join as`}
          value={value ?? "none"}
          disabled={disabled}
          options={JOIN}
          onChange={(next) => onChange(next === "none" ? null : next)}
          testId={`${testId}-picker`}
        />
      </span>
    </div>
  );
}

