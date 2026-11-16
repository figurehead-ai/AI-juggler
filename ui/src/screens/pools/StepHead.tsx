/**
 * AJ-E3011: a setup step's heading and its one line on why, the same on
 * every step (one spacing scale). D-126: on the type scale, as the page's
 * title (22 px semibold ink) with its line in body type (14 px, ink-2),
 * and an eyebrow over it when the step says where it is ("Step 3 of 9 ·
 * The basics").
 */
import type { JSX } from "react";

import { STEP_WORDS, type SetupStep } from "../../lib/pool-words";

export function StepHead({ step, eyebrow }: { step: SetupStep; eyebrow?: string }): JSX.Element {
  const words = STEP_WORDS[step];
  return (
    <div className="flex flex-col gap-1.5">
      {eyebrow && (
        <p className="section-label text-accent" data-testid="new-pool-step-eyebrow">
          {eyebrow}
        </p>
      )}
      <h3 className="type-title" data-testid="new-pool-step-title">
        {words.title}
      </h3>
      <p className="max-w-[60ch] text-body text-ink-2" data-testid="new-pool-step-why">
        {words.why}
      </p>
    </div>
  );
}
