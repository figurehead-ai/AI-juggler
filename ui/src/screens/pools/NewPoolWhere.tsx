/**
 * AJ-E3011 step 1, Where: the pool's name and which projects. A name and
 * its projects make a working pool; everything after is filled in.
 *
 * D-126: the projects as the kit's choice cards, each saying what's open
 * in it now (and in which tools) or when it was last worked in; then
 * "Sessions with no project" and "Every project". The cards are the
 * chosen ones, then those open now, then the latest, at most six; the
 * rest are a search away (Add another project: open now first, then
 * All). Every project takes them all in, so their cards say so.
 */
import { useEffect, useId, useRef, useState, type JSX } from "react";

import { CheckCard, Picker } from "../../components/controls";
import { Alert } from "../../components/Alert";
import { IconProjects, IconSessions } from "../../components/icons";
import { LogoTile } from "../../components/ToolLogo";
import type { Draft } from "../../lib/new-pool";
import { projectChoices } from "../../lib/project-words";
import { sessionState } from "../../lib/rail-items";
import { agoWords } from "../../lib/time-words";
import { toolSpec } from "../../lib/tools";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { StepHead } from "./StepHead";

/** At most this many project cards; the rest are a search away. */
export const PROJECT_CARDS = 6;

export interface WhereProject {
  id: string;
  name: string;
  root?: string;
  /** It has sessions open now (offered first). */
  open: boolean;
}

/** A project pick a template change cleared (AJ-E2312 item 6): what was
 * picked, and how the new template uses its project. */
export interface ClearedPick {
  /** What was picked, named as Review names it ("team", "every project"). */
  what: string;
  /** "«Sealed client project» keeps one project apart". */
  why: string;
}

/** What's in a project now, in a line: "2 sessions open now · Claude
 * Code, Codex", else "Nothing open now · last used 3 h ago". */
export function projectLine(id: string | null, sessions: readonly InstanceInfo[], now: number): string {
  const here = sessions.filter((session) => session.project_id === id);
  const open = here.filter((session) => sessionState(session).state !== "ended");
  if (open.length > 0) {
    const tools = [...new Set(open.map((session) => toolSpec(session.provider).display_name))];
    return `${open.length} ${open.length === 1 ? "session" : "sessions"} open now · ${tools.join(", ")}`;
  }
  const last = Math.max(0, ...here.map((session) => session.last_activity_ms ?? session.started_at_ms));
  const when = agoWords(last, now);
  return when ? `Nothing open now · last used ${when}` : "Nothing open now";
}

/** The projects shown as cards: the chosen ones, then open now, then the
 * latest worked in, at most {@link PROJECT_CARDS}. */
export function cardProjects(projects: readonly WhereProject[], chosen: readonly (string | null)[], sessions: readonly InstanceInfo[]): WhereProject[] {
  const last = (id: string) => Math.max(0, ...sessions.filter((session) => session.project_id === id).map((session) => session.last_activity_ms ?? session.started_at_ms));
  const rank = (project: WhereProject) => (chosen.includes(project.id) ? 0 : project.open ? 1 : 2);
  return [...projects].sort((a, b) => rank(a) - rank(b) || last(b.id) - last(a.id)).slice(0, Math.max(PROJECT_CARDS, chosen.length));
}

export function NewPoolWhere({
  draft,
  onDraft,
  cleared = null,
  projects,
  sessions = [],
  nameFixed = false,
  suggestedName,
  scopeFixed,
  now,
}: {
  /** A project's own pool (round 5): its scope, said, not chosen. */
  scopeFixed?: string;
  draft: Draft;
  onDraft: (next: Draft) => void;
  /** The pick the last template change cleared: said until a project is
   * picked again. */
  cleared?: ClearedPick | null;
  /** Every project; `open`: it has sessions open now (shown first). */
  projects: readonly WhereProject[];
  /** What's open in each, and when each was last worked in. */
  sessions?: readonly InstanceInfo[];
  /** Setting up an existing pool: its name stays. */
  nameFixed?: boolean;
  /** The name it gets when none is typed ("web team"). */
  suggestedName: string;
  now?: number;
}): JSX.Element {
  const [opened] = useState(() => Date.now());
  const at = now ?? opened;
  const ids = useId();
  const shown = cardProjects(projects, draft.projects, sessions);
  const others = projects.filter((project) => !shown.includes(project));
  const has = (id: string | null) => draft.projects.includes(id);
  const flip = (id: string | null, on: boolean) => onDraft({ ...draft, projects: on ? [...draft.projects.filter((one) => one !== id), id] : draft.projects.filter((one) => one !== id) });
  const every = draft.every ? "Every project is in" : undefined;
  // Opened with no project, Next waits for one: the first project takes
  // the keyboard's focus (Next can't hold it while it waits).
  const choices = useRef<HTMLFieldSetElement | null>(null);
  const none = !draft.every && draft.projects.length === 0;
  const [openedEmpty] = useState(none);
  useEffect(() => {
    if (openedEmpty) choices.current?.querySelector<HTMLInputElement>("input:not(:disabled)")?.focus();
  }, [openedEmpty]);
  return (
    <div className="flex flex-col gap-7" data-testid="new-pool-step-where">
      <StepHead step="where" />
      <div className="flex flex-col gap-2">
        <label htmlFor={`${ids}-name`} className="text-meta font-medium text-ink">
          Pool name
        </label>
        <input
          id={`${ids}-name`}
          aria-describedby={`${ids}-hint`}
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          readOnly={nameFixed}
          value={draft.name}
          onChange={(event) => onDraft({ ...draft, name: event.target.value })}
          maxLength={48}
          placeholder={suggestedName}
          className="h-11 w-full max-w-[440px] rounded-[10px] bg-surface px-3.5 text-body text-ink outline-none inset-ring-1 inset-ring-hairline/75 placeholder:text-muted focus-visible:inset-ring-[1.5px] focus-visible:inset-ring-accent focus-visible:ring-4 focus-visible:ring-accent-soft"
          data-testid="new-pool-name"
        />
        <span id={`${ids}-hint`} className="text-meta text-muted">
          {draft.name.trim() ? "You can rename it any time." : `Left empty, it's called «${suggestedName}». You can rename it any time.`}
        </span>
      </div>
      {scopeFixed ? (
        <p className="text-meta text-ink" data-testid="new-pool-where-fixed">
          {scopeFixed}
        </p>
      ) : (
        <fieldset ref={choices} className="flex min-w-0 flex-col gap-2.5" data-testid="new-pool-where">
          <legend className="mb-2.5 text-meta font-medium text-ink">Which project</legend>
          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2" data-testid="new-pool-projects">
            {shown.map((project) => (
              <CheckCard
                key={project.id}
                checked={draft.every || has(project.id)}
                onChange={(on) => flip(project.id, on)}
                disabled={draft.every}
                reason={every}
                logo={<LogoTile icon={IconProjects} size={32} on={draft.every || has(project.id)} />}
                title={project.name}
                sub={projectLine(project.id, sessions, at)}
                testId="new-pool-project"
              />
            ))}
            <CheckCard
              checked={draft.every || has(null)}
              onChange={(on) => flip(null, on)}
              disabled={draft.every}
              reason={every}
              logo={<LogoTile icon={IconProjects} size={32} on={draft.every || has(null)} />}
              title="Sessions with no project"
              sub="Chats started outside a folder"
              testId="new-pool-project-none"
            />
            <CheckCard
              checked={draft.every}
              onChange={(on) => onDraft({ ...draft, every: on })}
              logo={<LogoTile icon={IconSessions} size={32} on={draft.every} />}
              title="Every project"
              sub="Any session, wherever it works"
              testId="new-pool-project-every"
            />
          </div>
          {others.length > 0 && !draft.every && (
            <Picker
              ariaLabel="Add another project"
              value={null}
              groups={[{ label: "Open now", values: others.filter((project) => project.open).map((project) => project.id) }]}
              allHeading="All projects"
              searchLabel="Search projects"
              size="sm"
              look="quiet"
              placeholder={`+ Add another project (${others.length} more)`}
              options={projectChoices(others.filter((project) => !has(project.id)))}
              onChange={(value) => flip(value, true)}
              testId="new-pool-add-project"
            />
          )}
          {none ? (
            <Alert level="needs-you" className="mt-1.5" testId="new-pool-where-words">
              {cleared ? `${cleared.why}, so your pick (${cleared.what}) was cleared. Pick at least one project: only its sessions can join.` : "Pick at least one project: only its sessions can join."}
            </Alert>
          ) : cleared ? (
            <Alert level="info" className="mt-1.5" testId="new-pool-where-words">
              {cleared.why}, so your pick ({cleared.what}) was cleared.
            </Alert>
          ) : (
            <Alert level="info" className="mt-1.5" testId="new-pool-where-words">
              The sessions in a pool see only this pool's members, never your other pools.
            </Alert>
          )}
        </fieldset>
      )}
    </div>
  );
}
