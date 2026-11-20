/**
 * 0.4.6 (D-126): what sessions may do on their own, as the kit's choice
 * cards, each with a small drawing — Look only, Work together, Work
 * together and wake each other — with the same words as the diagram's
 * control (`OwnLevelChoice`; paths-parity holds them word for word): what
 * it covers, each one's consequence, the chosen one's example (and the
 * wake one's always: each wake is a turn on the user's subscription,
 * D-28, D-119), and "What does this mean?".
 */
import type { JSX } from "react";

import { Alert } from "../../components/Alert";
import { RadioCard, RadioCards } from "../../components/controls";
import { Explain } from "../../components/switch/Explain";
import { asOwnLevel, OWN_LABEL, OWN_LEVELS, OWN_SCOPE, POOLS_GUIDE, type OwnLevel } from "../../lib/pool-words";
import { OwnArt } from "./illustrations";

/** The guide's section on what agents may do. */
const GUIDE = `${POOLS_GUIDE}#what-agents-may-do`;

/** "What does this mean?": the diagram's own tips. */
const OWN_TIPS: readonly string[] = [
  "It's for the sessions in this pool only, and only for AI Juggler's pool tools: messages, shared notes and tasks.",
  "Anything above it becomes an ask in Needs you: Allow once, Always allow here, or Deny.",
  "Waking starts a turn in an idle session, and each turn uses your subscription. A chain of replies pauses and asks you after a few in a row.",
  "Each tool still asks its own permission prompts; AI Juggler never answers them for you.",
];

export function OwnCards({ value, onChange, label = OWN_LABEL, testId = "own-cards" }: { value: OwnLevel; onChange: (level: OwnLevel) => void; label?: string; testId?: string }): JSX.Element {
  return (
    <fieldset className="flex min-w-0 flex-col gap-3" data-testid={testId}>
      <legend className="sr-only">{label}</legend>
      <Alert level="info" live={null} testId={`${testId}-scope`}>
        {OWN_SCOPE}
      </Alert>
      <div data-testid={`${testId}-choice`}>
        <RadioCards label={label} value={value} onChange={(next) => onChange(asOwnLevel(next))}>
          {OWN_LEVELS.map((one) => {
            const on = one.id === value;
            return (
              <RadioCard
                key={one.id}
                value={one.id}
                logo={
                  <span aria-hidden="true" className="flex h-16 w-32 items-center rounded-[10px] bg-canvas-hi px-1">
                    <OwnArt level={one.id} on={on} />
                  </span>
                }
                title={one.label}
                sub={
                  <>
                    <span data-testid={on ? `${testId}-line` : undefined}>{one.line}</span>
                    {(on || one.id === "full") && (
                      <span className="mt-0.5 block text-muted" data-testid={on ? `${testId}-example` : `${testId}-example-${one.id}`}>
                        Example: {one.example}
                      </span>
                    )}
                  </>
                }
                testId={`${testId}-choice-${one.id}`}
              />
            );
          })}
        </RadioCards>
      </div>
      <Explain testId={`${testId}-explain`} guide={GUIDE} tips={[...OWN_TIPS]} />
    </fieldset>
  );
}
