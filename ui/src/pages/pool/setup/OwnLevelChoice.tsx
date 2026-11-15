/**
 * 0.4.6 (AJ-E3011 §5): what sessions may do on their own (the old
 * "level"): Look only, Work together, Work together and wake each other,
 * each with its consequence. Waking is the serious one (each wake is a
 * turn on the user's subscription, D-28, D-119). Round 2: a line under
 * the label says what it covers and what stays with each app (#4), and
 * the wake option's example shows at once under it (#5), the chosen
 * one's too; "What does this mean?" keeps the tips and the guide. A
 * member's own choice is labelled "On its own:" and offers "Same as the
 * pool" first (#10). The setup and the inspector share it.
 */
import { useId, type JSX } from "react";

import { ChoiceOption, Picker } from "../../../components/controls";
import { SELECTED_FILL } from "../../../components/controls/selected";
import { Explain } from "../../../components/switch/Explain";
import { OWN_LABEL, OWN_LEVELS, OWN_SCOPE, POOLS_GUIDE, SAME_AS_POOL, asOwnLevel, ownLabel, type OwnLevel } from "../../../lib/pool-words";

/** The guide's section on what agents may do. */
const GUIDE = `${POOLS_GUIDE}#what-agents-may-do`;

export function OwnLevelChoice({
  value,
  onChange,
  label = OWN_LABEL,
  disabled = false,
  testId = "own-level",
}: {
  value: OwnLevel;
  onChange: (level: OwnLevel) => void;
  /** Its visible label: what it sets and for whom. */
  label?: string;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  const name = useId();
  return (
    <fieldset className="flex min-w-0 flex-col gap-2" data-testid={testId}>
      <legend className="text-meta font-medium text-ink">{label}</legend>
      <p className="-mt-1 text-meta text-ink-2" data-testid={`${testId}-scope`}>
        {OWN_SCOPE}
      </p>
      <div className="flex flex-col gap-0.5" data-testid={`${testId}-choice`}>
        {OWN_LEVELS.map((one) => {
          const on = one.id === value;
          return (
            <ChoiceOption
              key={one.id}
              type="radio"
              name={name}
              checked={on}
              disabled={disabled}
              onChange={() => onChange(one.id)}
              label={one.label}
              hint={
                <>
                  <span data-testid={on ? `${testId}-line` : undefined}>{one.line}</span>
                  {(on || one.id === "full") && (
                    <span className="mt-0.5 block text-muted" data-testid={on ? `${testId}-example` : `${testId}-example-${one.id}`}>
                      Example: {one.example}
                    </span>
                  )}
                </>
              }
              className={on ? SELECTED_FILL : ""}
              data-testid={`${testId}-choice-${one.id}`}
            />
          );
        })}
      </div>
      <Explain
        testId={`${testId}-explain`}
        guide={GUIDE}
        tips={[
          "It's for the sessions in this pool only, and only for AI Juggler's pool tools: messages, shared notes and tasks.",
          "Anything above it becomes an ask in Needs you: Allow once, Always allow here, or Deny.",
          "Waking starts a turn in an idle session, and each turn uses your subscription. A chain of replies pauses and asks you after a few in a row.",
          "Each tool still asks its own permission prompts; AI Juggler never answers them for you.",
        ]}
      />
    </fieldset>
  );
}

/** A member's own choice: "Same as the pool (Work together)" first, then
 * the three. `value` null: the pool's. */
export function MemberOwnLevel({
  name,
  value,
  poolLevel,
  onChange,
  disabled = false,
  prefix = "On its own:",
  testId,
}: {
  /** Its visible label, before the value ("" where a heading says it). */
  prefix?: string;
  name: string;
  value: string | null;
  poolLevel: string;
  onChange: (level: OwnLevel | null) => void;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  return (
    <Picker
      choice
      size="sm"
      ariaLabel={`What ${name} may do on its own`}
      heading={`What ${name} may do on its own`}
      prefix={prefix}
      value={value ?? "pool"}
      disabled={disabled}
      options={[
        { value: "pool", label: `${SAME_AS_POOL} (${ownLabel(poolLevel)})`, hint: "Follows the pool's choice, now and when it changes." },
        ...OWN_LEVELS.map((one) => ({ value: one.id, label: one.label, hint: one.line })),
      ]}
      onChange={(next) => onChange(next === "pool" ? null : asOwnLevel(next))}
      testId={testId}
    />
  );
}
