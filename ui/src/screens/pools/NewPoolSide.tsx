/**
 * 0.4.6 (D-126): what New pool shows beside its form — How to start's
 * "What a pool does", then on every other step the live preview (the
 * drawing lights the part the step sets, its row too), and on Review the
 * team as its diagram will open, with the first thing to tell its lead.
 * Under 960 px it folds into a strip above the form.
 */
import type { ReactNode } from "react";

import type { Draft } from "../../lib/new-pool";
import type { DraftView } from "../../lib/pool-wizard-words";
import type { SetupStep } from "../../lib/pool-words";
import { firstPrompt, shapeOfDraft } from "../../lib/pool-sentences";
import type { LogoSources } from "./MemberLogo";
import { LeadPrompt } from "./LeadPrompt";
import { NewPoolPreview, PreviewDiagram, PreviewStrip, type PreviewFocus } from "./NewPoolPreview";
import { NewPoolStartSide } from "./NewPoolStartSide";
import { previewNodes, previewRows, rowOf } from "./preview-model";

const FOCUS: Partial<Record<SetupStep, PreviewFocus>> = { who: "members", lead: "lead", everyone: "reviewers" };

export function sideFor({
  current,
  draft,
  name,
  where,
  view,
  visited,
  sources,
  memberName,
  first,
}: {
  current: SetupStep;
  draft: Draft;
  name: string;
  where: string;
  view: DraftView;
  /** A step walked already (its value chosen). */
  visited: (step: SetupStep) => boolean;
  sources: LogoSources;
  memberName: (ref: string, label: string) => string;
  /** No pool of the user's yet (the intro is for a newcomer). */
  first: boolean;
}): { preview: ReactNode; summary: ReactNode } {
  const hasProject = draft.every || draft.projects.length > 0;
  const rows = previewRows(view, current, visited, hasProject);
  const summary = <PreviewStrip name={name} rows={rows} />;
  if (current === "template") return { preview: <NewPoolStartSide first={first} />, summary: null };
  const reached = visited("who") || current === "who";
  const nodes = previewNodes(draft, memberName, where, sources);
  const diagram = <PreviewDiagram lead={nodes.lead} members={nodes.members} reached={reached} focus={FOCUS[current] ?? null} />;
  if (current === "review") {
    const shape = shapeOfDraft(draft, memberName);
    // No lead: nothing to tell one; its rows instead.
    const prompt = firstPrompt(shape, where) ? <LeadPrompt shape={shape} where={where} testId="new-pool-first-prompt" /> : undefined;
    return {
      preview: <NewPoolPreview name={name} subtitle="Your team, as its diagram will open" diagram={diagram} rows={rows} current={null} foot={prompt === undefined} extra={prompt} />,
      summary,
    };
  }
  return { preview: <NewPoolPreview name={name} diagram={diagram} rows={rows} current={rowOf(current)} />, summary };
}
