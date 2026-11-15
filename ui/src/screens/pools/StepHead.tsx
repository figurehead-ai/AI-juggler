/**
 * AJ-E3011: a setup step's heading and its one line on why, the same on
 * every step (one spacing scale).
 */
import type { JSX } from "react";

import { STEP_WORDS, type SetupStep } from "../../lib/pool-words";

export function StepHead({ step }: { step: SetupStep }): JSX.Element {
  const words = STEP_WORDS[step];
  return (
    <div className="flex flex-col gap-0.5">
      <h3 className="text-section font-semibold text-ink" data-testid="new-pool-step-title">
        {words.title}
      </h3>
      <p className="text-meta text-ink-2" data-testid="new-pool-step-why">
        {words.why}
      </p>
    </div>
  );
}
