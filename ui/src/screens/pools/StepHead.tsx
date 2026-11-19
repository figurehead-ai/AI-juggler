/**
 * AJ-E3011: a setup step's heading and its one line on why, the same on
 * every step (one spacing scale). D-126: on the type scale, as the page's
 * title (22 px semibold ink) with its line in body type (14 px, ink-2);
 * the step's question as the mock-ups ask it ("Who's in this team?"); an
 * eyebrow over it saying where it is ("Step 3 of 9 · The basics", from
 * the wizard around it, or given), and a guide link after the line where
 * the step has one ("What a lead does ↗").
 */
import { createContext, useContext, type JSX } from "react";

import { IconUpRight } from "../../components/icons-places";
import { trustOpen } from "../../lib/ipc/trust";
import { WIZARD_HEADS } from "../../lib/pool-wizard-words";
import type { SetupStep } from "../../lib/pool-words";

/** The eyebrow the wizard gives the step it shows. */
export const StepEyebrow = createContext<string | undefined>(undefined);

/** A guide link in a line ("What a lead does ↗"). */
export function GuideLink({ guide, children, testId }: { guide: string; children: string; testId?: string }): JSX.Element {
  return (
    <button
      type="button"
      onClick={() => void trustOpen({ kind: "guide", id: guide })}
      className="inline-flex items-center gap-0.5 rounded font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid={testId}
    >
      {children}
      <IconUpRight aria-hidden="true" width={13} height={13} />
    </button>
  );
}

export function StepHead({ step, eyebrow, title }: { step: SetupStep; eyebrow?: string; title?: string }): JSX.Element {
  const words = WIZARD_HEADS[step];
  const given = useContext(StepEyebrow);
  const shown = eyebrow ?? given;
  return (
    <div className="flex flex-col gap-1.5">
      {shown && (
        <p className="section-label text-accent-hi" data-testid="new-pool-step-eyebrow">
          {shown}
        </p>
      )}
      <h3 className="type-title" data-testid="new-pool-step-title">
        {title ?? words.title}
      </h3>
      <p className="max-w-[62ch] text-body text-ink-2" data-testid="new-pool-step-why">
        {words.why}
        {words.learn && (
          <>
            {" "}
            <GuideLink guide={words.learn.guide} testId="new-pool-step-learn">
              {words.learn.label}
            </GuideLink>
          </>
        )}
      </p>
    </div>
  );
}
