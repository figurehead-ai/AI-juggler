/**
 * 0.4.6 (D-126): New pool's steps down the side, in three groups (The
 * basics; How they work, "prefilled" until it's reached; Finish), each
 * step's value under its name: what it's set to once walked, and what it
 * will be for the prefilled ones. The steps up to the furthest reached,
 * and the one after it (as Next goes), are a click away; from Who's in
 * on, every one is (a name and its projects make a working pool). One
 * not yet says after which (AJ-E2311 item 5: the rail's steps couldn't be
 * pressed, and said nothing). The guide's link at the foot.
 */
import type { JSX } from "react";

import { FlowSteps, type FlowStep } from "../../components/FlowLayout";
import { IconPage } from "../../components/icons-actions";
import { GROUP_OF, PREFILLED, WIZARD_GROUPS } from "../../lib/pool-wizard-words";
import { POOLS_GUIDE, type SetupStep } from "../../lib/pool-words";
import { navOf } from "./preview-model";
import { GuideLink } from "./StepHead";

/** From this step on, every step is a click away. */
const OPEN_FROM: SetupStep = "who";

export function NewPoolSteps({
  steps,
  step,
  furthest,
  valueOf,
  onPick,
  held = null,
  heldWhy,
}: {
  steps: readonly SetupStep[];
  /** The step shown, by its place. */
  step: number;
  /** The furthest step reached, by its place. */
  furthest: number;
  /** A step's value under its name (`null`: none). */
  valueOf: (step: SetupStep, walked: boolean) => string | null;
  onPick: (at: number) => void;
  /** Nothing goes on past this place until the choice its step needs is
   * made; the steps after it say why (`heldWhy`). */
  held?: number | null;
  heldWhy?: string;
}): JSX.Element {
  const open = furthest >= steps.indexOf(OPEN_FROM);
  const next = steps[furthest + 1];
  const rows: FlowStep[] = steps.map((id, at) => {
    const state = at === step ? "current" : at < step || at <= furthest ? "done" : "todo";
    // The basics say nothing until walked; the rest say what they're
    // filled in with.
    const value = state === "done" || (state === "todo" && GROUP_OF[id] !== "basics") ? valueOf(id, state === "done") : null;
    const stopped = held !== null && at > held;
    return { id, label: navOf(id), value: value ?? undefined, state, group: GROUP_OF[id], disabled: stopped || !(at <= furthest + 1 || open), reason: stopped ? heldWhy : next ? `After ${navOf(next)}` : undefined };
  });
  const groups = WIZARD_GROUPS.map((group) => (group.id === "how" && step <= steps.indexOf(OPEN_FROM) ? { ...group, tag: PREFILLED } : group));
  return (
    <FlowSteps
      steps={rows}
      groups={groups}
      onPick={(id) => onPick(steps.findIndex((one) => one === id))}
      testId="new-pool-steps"
      foot={
        <p className="flex items-center gap-3 px-2.5 text-meta" data-testid="new-pool-guide">
          <IconPage aria-hidden="true" width={18} height={18} className="shrink-0 text-muted" />
          <span className="flex flex-col">
            <span className="font-medium text-ink">New to pools?</span>
            <GuideLink guide={POOLS_GUIDE}>Read the 2-minute guide</GuideLink>
          </span>
        </p>
      }
    />
  );
}
