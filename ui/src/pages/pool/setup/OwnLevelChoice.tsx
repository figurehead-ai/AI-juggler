/**
 * 0.4.6 (AJ-E3011 §5): what sessions may do on their own (the old
 * "level"): Look only, Work together, Work together and wake each other,
 * each with its consequence. Waking is the serious one (each wake is a
 * turn on the user's subscription, D-28, D-119). Round 2: a line under
 * the label says what it covers and what stays with each app (#4), and
 * the wake option's example shows at once under it (#5), the chosen
 * one's too; "What does this mean?" keeps the tips and the guide. A
 * member's own choice is labelled "On its own:" and offers "Same as the
 * pool" first (#10). Round 3: the one control for this setting, in New
 * pool, the pool's inspector and the diagram's header (`look="menu"`,
 * "On their own (everyone in this pool):"), with Custom where a made pool
 * offers it; a member's own choice is MemberOwnLevel, everywhere.
 */
import { useId, type JSX } from "react";

import { ChoiceOption, Picker } from "../../../components/controls";
import { SELECTED_FILL } from "../../../components/controls/selected";
import { Explain } from "../../../components/switch/Explain";
import { OWN_LABEL, OWN_LEVELS, OWN_SCOPE, POOLS_GUIDE, SAME_AS_POOL, asOwnLevel, ownLabel, type OwnLevel } from "../../../lib/pool-words";

/** The guide's section on what agents may do. */
const GUIDE = `${POOLS_GUIDE}#what-agents-may-do`;

/** A made pool's fourth choice: each action picked by hand. */
const CUSTOM = { label: "Custom", line: "You pick each action, under Custom actions." };

export function OwnLevelChoice({
  value,
  onChange,
  onCustom,
  look = "radios",
  label = OWN_LABEL,
  disabled = false,
  testId = "own-level",
}: {
  /** A made pool's may be "custom" (each action picked by hand). */
  value: OwnLevel | "custom";
  onChange: (level: OwnLevel) => void;
  /** A made pool: Custom is offered, and chosen with this. */
  onCustom?: () => void;
  /** Radios with each line (the setup, the inspector), or a menu (the
   * diagram's header). */
  look?: "radios" | "menu";
  /** Its visible label: what it sets and for whom. */
  label?: string;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  const name = useId();
  if (look === "menu")
    return (
      <Picker
        choice
        size="sm"
        ariaLabel={label}
        heading={label}
        prefix="On their own (everyone in this pool):"
        value={value}
        disabled={disabled}
        options={[...OWN_LEVELS.map((one) => ({ value: one.id, label: one.label, hint: one.line })), ...(onCustom ? [{ value: "custom", label: CUSTOM.label, hint: CUSTOM.line }] : [])]}
        onChange={(next) => (next === "custom" ? onCustom?.() : onChange(asOwnLevel(next)))}
        testId={testId}
      />
    );
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
        {onCustom && (
          // A made pool's own: each action picked by hand.
          <div data-made-pool-only>
          <ChoiceOption
            type="radio"
            name={name}
            checked={value === "custom"}
            disabled={disabled}
            onChange={onCustom}
            label={CUSTOM.label}
            hint={CUSTOM.line}
            className={value === "custom" ? SELECTED_FILL : ""}
            data-testid={`${testId}-choice-custom`}
          />
          </div>
        )}
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
  kept,
  testId,
}: {
  /** Its first choice's words, when it follows a wider member rather than
   * the pool ("Same as its App (Work together)"). */
  kept?: string;
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
        { value: "pool", label: kept ?? `${SAME_AS_POOL} (${ownLabel(poolLevel)})`, hint: kept ? "Follows that member's choice, now and when it changes." : "Follows the pool's choice, now and when it changes." },
        ...OWN_LEVELS.map((one) => ({ value: one.id, label: one.label, hint: one.line })),
      ]}
      onChange={(next) => onChange(next === "pool" ? null : asOwnLevel(next))}
      testId={testId}
    />
  );
}
