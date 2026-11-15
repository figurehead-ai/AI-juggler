/**
 * 0.4.6 (AJ-E3011 §5): what sessions may do on their own (the old
 * "level"): a labelled segmented control — Look only, Work together, Work
 * together and wake each other — with the chosen one's consequence and an
 * example under it. Waking is the serious one (each wake is a turn on the
 * user's subscription, D-28, D-119): its words say so, and "What does this
 * mean?" opens the tips and the guide. A member's own choice offers "Same
 * as the pool" first. The setup and the inspector share it.
 */
import { useId, type JSX } from "react";

import { Picker, Segmented } from "../../../components/controls";
import { Explain } from "../../../components/switch/Explain";
import { OWN_LABEL, OWN_LEVELS, POOLS_GUIDE, SAME_AS_POOL, asOwnLevel, ownLabel, ownLevel, type OwnLevel } from "../../../lib/pool-words";

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
  const id = useId();
  const chosen = ownLevel(value);
  return (
    <div className="flex flex-col gap-2" data-testid={testId}>
      <span id={id} className="text-meta font-medium text-ink">
        {label}
      </span>
      <div className={disabled ? "pointer-events-none opacity-60" : ""}>
        <Segmented
          calm
          wrap
          label={label}
          value={value}
          segments={OWN_LEVELS.map((one) => ({ value: one.id, label: one.label }))}
          onChange={(level) => !disabled && onChange(level)}
          testId={`${testId}-choice`}
          className="w-fit"
        />
      </div>
      <p className="text-meta text-ink-2" data-testid={`${testId}-line`}>
        {chosen.line}
      </p>
      <p className="text-meta text-muted" data-testid={`${testId}-example`}>
        Example: {chosen.example}
      </p>
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
    </div>
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
  testId,
}: {
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
