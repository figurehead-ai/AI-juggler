/**
 * AJ-E3007 step 4: the review, one sentence per rule (wireframe 4): its
 * name, where, who, talk, level and seal, then what each member can
 * actually do here (full, shell only, watched, or joins later).
 */
import type { JSX } from "react";

import { Chip } from "../../components/primitives";
import { MACHINES_WORD } from "../../lib/machine-words";
import type { Draft, Reach } from "../../lib/new-pool";
import { ruleOf, TALK_ROLES } from "../../lib/pool-templates";
import { levelLabel, roleLabel } from "../../lib/pools";
import { RULE_WORDS } from "./NewPoolRoles";

const CAN: Record<Reach, { words: string; tone: "success" | "neutral" | "warning" | "accent" }> = {
  full: { words: "sees · messages · tasks · can be asked", tone: "success" },
  shell: { words: "acts through its shell: the juggler-agent command", tone: "neutral" },
  watched: { words: "watched: shown, can't act until continued in AI Juggler", tone: "warning" },
  later: { words: "its sessions here join by themselves, now and later", tone: "accent" },
};

/** The talk grid in a sentence: "Allowed both ways" or its exceptions. */
export function talkSentence(draft: Draft): string {
  const changed: string[] = [];
  for (const from of TALK_ROLES) {
    for (const to of TALK_ROLES) {
      const rule = ruleOf(draft.talk, from, to);
      if (rule !== "allowed" && from !== "observer") changed.push(`${roleLabel(from)} to ${roleLabel(to)}: ${RULE_WORDS[rule].toLowerCase()}`);
    }
  }
  return changed.length === 0 ? "Everyone may message everyone; an Observer only reads." : `${changed.join("; ")}. The rest is allowed; an Observer only reads.`;
}

/** Who, in a sentence: the members with their roles, and what joins. */
export function whoSentence(draft: Draft): string {
  if (draft.members.length === 0) return "No one yet: add members on the canvas.";
  const named = draft.members.map((member) => `${member.label} (${roleLabel(member.role).toLowerCase()})`).join(", ");
  const joins = draft.members.some((member) => member.reach === "later");
  return `${named}. ${joins ? `New sessions of its Apps, Accounts, Tools or ${MACHINES_WORD} join by themselves.` : "Nothing joins by itself."}`;
}

export function NewPoolReview({
  draft,
  onName,
  projectName,
  nameFixed = false,
}: {
  draft: Draft;
  onName: (name: string) => void;
  /** Setting up an existing pool: its name stays. */
  nameFixed?: boolean;
  projectName: (id: string | null) => string;
}): JSX.Element {
  const where = draft.every ? "Every project, and sessions with no project." : draft.projects.length > 0 ? `${draft.projects.map(projectName).join(", ")}. Agents see only ${draft.projects.length === 1 ? "this project" : "these projects"}.` : "No project yet: pick one on the canvas.";
  return (
    <div className="flex flex-col gap-4">
      <label className="flex flex-col gap-1">
        <span className="text-label font-semibold uppercase tracking-wide text-muted">Name</span>
        <input
          readOnly={nameFixed}
          value={draft.name}
          onChange={(event) => onName(event.target.value)}
          maxLength={48}
          placeholder="Name the pool"
          className="h-9 max-w-md rounded-xl border border-hairline bg-surface-2 px-3 text-body text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
          data-testid="new-pool-name"
          data-autofocus
        />
      </label>
      <div className="grid grid-cols-1 gap-2.5 md:grid-cols-3" data-testid="new-pool-review">
        <Sentence title="Where">{where}</Sentence>
        <Sentence title="Who">{whoSentence(draft)}</Sentence>
        <Sentence title="Talk">
          {talkSentence(draft)} At {levelLabel(draft.level)}: anything more asks you.
          {draft.sealed && <span className="mt-1 block font-semibold text-danger">Sealed: its sessions are in no other pool, and nothing leaves it.</span>}
        </Sentence>
      </div>
      {draft.members.length > 0 && (
        <section className="flex flex-col gap-1.5" aria-label="What each member can do here">
          <h3 className="text-body font-semibold text-ink">What each member can do here</h3>
          <ul className="flex flex-col divide-y divide-hairline/60 rounded-card border border-hairline bg-surface">
            {draft.members.map((member) => (
              <li key={member.ref} className="flex flex-wrap items-center gap-2 px-3 py-2">
                <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{member.label}</span>
                <Chip tone={CAN[member.reach].tone}>{CAN[member.reach].words}</Chip>
              </li>
            ))}
          </ul>
        </section>
      )}
      <p className="text-meta text-ink-2">You can change all of this on the pool's canvas.</p>
    </div>
  );
}

function Sentence({ title, children }: { title: string; children: React.ReactNode }): JSX.Element {
  return (
    <div className="flex flex-col gap-1 rounded-card border border-hairline bg-surface p-3">
      <span className="text-label font-semibold uppercase tracking-wide text-muted">{title}</span>
      <span className="text-meta text-ink">{children}</span>
    </div>
  );
}
