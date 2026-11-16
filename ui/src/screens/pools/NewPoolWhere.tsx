/**
 * AJ-E3011 step 1, Where: the pool's name and which projects. A name and
 * its projects make a working pool; everything after is filled in.
 *
 * - The projects: every project, or the ones picked (chips, Add a
 *   project: open now first, then All). The "Add a project" list has no
 *   heading row repeating its label (the walks' "first row does nothing").
 */
import type { JSX } from "react";

import { Picker, Segmented } from "../../components/controls";
import type { Draft } from "../../lib/new-pool";
import { projectChoices } from "../../lib/project-words";
import { StepHead } from "./StepHead";

export function NewPoolWhere({
  draft,
  onDraft,
  projects,
  nameFixed = false,
  suggestedName,
  scopeFixed,
}: {
  /** A project's own pool (round 5): its scope, said, not chosen. */
  scopeFixed?: string;
  draft: Draft;
  onDraft: (next: Draft) => void;
  /** Every project; `open`: it has sessions open now (shown first). */
  projects: { id: string; name: string; root?: string; open: boolean }[];
  /** Setting up an existing pool: its name stays. */
  nameFixed?: boolean;
  /** The name it gets when none is typed ("web team"). */
  suggestedName: string;
}): JSX.Element {
  const name = (id: string | null) => (id === null ? "Sessions with no project" : (projects.find((project) => project.id === id)?.name ?? "A project"));
  const addable = projects.filter((project) => !draft.projects.includes(project.id));
  return (
    <div className="flex max-w-3xl flex-col gap-5" data-testid="new-pool-step-where">
      <StepHead step="where" />
      <label className="flex flex-col gap-1.5">
        <span className="text-meta font-medium text-ink">The pool's name</span>
        <input
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          readOnly={nameFixed}
          value={draft.name}
          onChange={(event) => onDraft({ ...draft, name: event.target.value })}
          maxLength={48}
          placeholder={suggestedName}
          className="h-9 max-w-md rounded-xl bg-fill px-3 text-body text-ink outline-none placeholder:text-muted focus-visible:ring-2 focus-visible:ring-accent/60 seg-fill"
          data-testid="new-pool-name"
        />
        <span className="text-meta text-muted">{draft.name.trim() ? "You can rename it later." : `Left empty, it's called «${suggestedName}».`}</span>
      </label>
      {scopeFixed ? (
        <p className="text-meta text-ink" data-testid="new-pool-where-fixed">
          {scopeFixed}
        </p>
      ) : (
      <fieldset className="flex min-w-0 flex-col gap-2">
        <legend className="mb-2 text-meta font-medium text-ink">Which projects its sessions come from</legend>
        <Segmented
          label="Which projects its sessions come from"
          value={draft.every ? "every" : "these"}
          segments={[
            { value: "these", label: "These projects" },
            { value: "every", label: "Every project" },
          ]}
          onChange={(value) => onDraft({ ...draft, every: value === "every" })}
          testId="new-pool-where"
          className="self-start"
        />
        {!draft.every && (
          <div className="flex flex-wrap items-center gap-1.5" data-testid="new-pool-projects">
            {draft.projects.map((id) => (
              <span key={id ?? "none"} className="inline-flex items-center gap-1 rounded-full bg-surface-3 py-0.5 pl-2.5 pr-1 text-meta text-ink">
                {name(id)}
                <button type="button" aria-label={`Take ${name(id)} out`} className="rounded-full px-1 text-muted outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => onDraft({ ...draft, projects: draft.projects.filter((one) => one !== id) })}>
                  ×
                </button>
              </span>
            ))}
            <Picker
              ariaLabel="Add a project"
              value={null}
              groups={[{ label: "Open now", values: addable.filter((project) => project.open).map((project) => project.id) }]}
              allHeading="All projects"
              searchLabel="Search projects"
              size="sm"
              look="quiet"
              placeholder="+ Add a project"
              options={[...projectChoices(addable), ...(draft.projects.includes(null) ? [] : [{ value: "__none", label: "Sessions with no project" }])]}
              onChange={(value) => onDraft({ ...draft, projects: [...draft.projects, value === "__none" ? null : value] })}
              testId="new-pool-add-project"
            />
          </div>
        )}
        <p className="text-meta text-ink-2" data-testid="new-pool-where-words">
          {draft.every
            ? "Sessions in any project, and those with no project, can join. They see only this pool's members."
            : draft.projects.length === 0
              ? "Pick at least one project: only its sessions can join."
              : `Only sessions in ${draft.projects.map(name).join(", ")} can join, and they see only this pool's members.`}
        </p>
      </fieldset>
      )}
    </div>
  );
}
