/**
 * 0.3 — a project page's state chips beside its name, as the Projects
 * screen says them: what needs you there ("1 asks you") and what's
 * working ("2 working").
 */
import { useMemo, type JSX } from "react";
import { Pill } from "../../components/Pill";
import { useBackendNeeds } from "../../lib/needs-you";
import { useConversations } from "../../lib/conversations";
import { needChips, needsByProject, openByProject } from "../../lib/project-summary";
import type { InstanceInfo } from "../../lib/bridge";

export function ProjectChips({ projectId, sessions }: { projectId: string; sessions: InstanceInfo[] }): JSX.Element {
  const needs = useBackendNeeds();
  const { all } = useConversations();
  const chips = needChips(useMemo(() => needsByProject(needs.rows), [needs.rows]).get(projectId));
  const open = useMemo(() => openByProject(sessions, all), [sessions, all]).get(projectId);
  return (
    <span className="flex flex-wrap items-center gap-1.5" data-testid="project-chips">
      {chips.map((chip) => (
        <Pill key={chip.key} tone={chip.tone}>
          {chip.words}
        </Pill>
      ))}
      {open && open.working > 0 && <Pill tone="live">{open.working} working</Pill>}
    </span>
  );
}
