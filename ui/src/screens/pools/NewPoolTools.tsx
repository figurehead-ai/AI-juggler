/**
 * AJ-E3011 round 4 (main's ruling), D-30: New pool's own step for the one
 * question, before Review, while it's unanswered: the pool page's own
 * card (TeamToolsQuestion). Once answered here, it says where they stand.
 */
import type { JSX } from "react";

import { teamToolsLine } from "../../lib/team-tools";
import { TeamToolsQuestion, useTeamTools } from "../../pages/pool/setup/TeamTools";
import { StepHead } from "./StepHead";

export function NewPoolTools({ onAnswer }: { onAnswer?: () => void }): JSX.Element {
  const state = useTeamTools();
  return (
    <div className="flex max-w-3xl flex-col gap-4" data-testid="new-pool-step-tools">
      <StepHead step="tools" />
      <TeamToolsQuestion testId="new-pool-tools" onAnswer={onAnswer} />
      {state?.kind === "on" && (
        <p className="text-meta text-ink" data-testid="new-pool-tools-answered">
          {teamToolsLine(state)}
        </p>
      )}
    </div>
  );
}
