/**
 * New pool's form for the step shown: each step its own component, given
 * the draft and what it needs from the wizard.
 */
import type { JSX } from "react";

import type { AppModel } from "../../lib/apps";
import type { Candidate, Draft } from "../../lib/new-pool";
import type { MemberKind } from "../../lib/ipc/pool-model";
import type { SetupStep } from "../../lib/pool-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { NewPoolEveryone } from "./NewPoolEveryone";
import { NewPoolLead } from "./NewPoolLead";
import { NewPoolOwn } from "./NewPoolOwn";
import { NewPoolReview } from "./NewPoolReview";
import { NewPoolTalk } from "./NewPoolTalk";
import { NewPoolTemplate } from "./NewPoolTemplate";
import { NewPoolTools } from "./NewPoolTools";
import { NewPoolWhere, type WhereProject } from "./NewPoolWhere";
import { NewPoolWho } from "./NewPoolWho";

export function NewPoolStep({
  current,
  apps,
  draft,
  setDraft,
  name,
  where,
  memberName,
  candidates,
  sessions,
  projects,
  stepProjects,
  openNow,
  onFix,
  onChoose,
  onGo,
  onToolsAnswer,
  firstPool,
  foldTemplates,
  nameFixed,
  suggestedName,
  scopeFixed,
}: {
  current: SetupStep;
  /** Every App, the CLI ones too (read with the folders). */
  apps: readonly AppModel[] | null;
  draft: Draft;
  setDraft: (next: Draft) => void;
  name: string;
  where: string;
  memberName: (ref: string, label: string) => string;
  candidates: Record<MemberKind, Candidate[]>;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  stepProjects: WhereProject[];
  openNow: ReadonlyMap<string, number>;
  onFix: (row: Candidate) => void;
  onChoose: (template: string) => void;
  onGo: (step: SetupStep) => void;
  onToolsAnswer: () => void;
  firstPool: boolean;
  foldTemplates: boolean;
  nameFixed: boolean;
  suggestedName: string;
  scopeFixed: string | undefined;
}): JSX.Element | null {
  switch (current) {
    case "template":
      return <NewPoolTemplate chosen={draft.template} onChoose={onChoose} first={firstPool} fold={foldTemplates} />;
    case "where":
      return <NewPoolWhere draft={draft} onDraft={setDraft} projects={stepProjects} sessions={sessions} nameFixed={nameFixed} suggestedName={suggestedName} scopeFixed={scopeFixed} />;
    case "who":
      return <NewPoolWho draft={draft} onDraft={setDraft} candidates={candidates} onFix={onFix} nameOf={memberName} where={where} sessions={sessions} apps={apps} />;
    case "lead":
      return <NewPoolLead draft={draft} onDraft={setDraft} sessions={candidates.session} nameOf={memberName} where={where} open={sessions} apps={apps} />;
    case "everyone":
      return <NewPoolEveryone draft={draft} onDraft={setDraft} where={where} nameOf={memberName} open={sessions} apps={apps} />;
    case "talk":
      return <NewPoolTalk draft={draft} onDraft={setDraft} />;
    case "own":
      return <NewPoolOwn draft={draft} onDraft={setDraft} where={where} />;
    case "tools":
      return <NewPoolTools onAnswer={onToolsAnswer} members={draft.members.map((member) => member.ref)} open={sessions} apps={apps} />;
    case "review":
      return <NewPoolReview draft={draft} name={name} where={where} nameOf={memberName} openNow={openNow} projects={projects} onChange={onGo} />;
    default:
      return null;
  }
}
