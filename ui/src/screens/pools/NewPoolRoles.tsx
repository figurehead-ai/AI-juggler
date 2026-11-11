/**
 * AJ-E3007 step 3: roles and talk, with the template's defaults
 * (wireframe 3).
 *
 * - Each member: its role (segmented) and its own level ("pool's" by
 *   default, D-62 decision 12).
 * - Who may talk to whom: the role-pair grid, green, amber or red; a click
 *   cycles allowed → ask first → blocked. One pair of members can differ:
 *   its line on the canvas.
 * - What agents may do (the pool's level), and the seal with its
 *   consequence in plain words.
 */
import type { JSX } from "react";

import { Checkbox, Picker, Segmented } from "../../components/controls";
import type { PoolRole } from "../../lib/ipc/pools";
import type { TalkRule } from "../../lib/ipc/pool-model";
import { leadLater, rolesToChoose, type Draft } from "../../lib/new-pool";
import { nextRule, pairKey, ruleOf, TALK_ROLES } from "../../lib/pool-templates";
import { CLAUDE_CHANNEL_NOTE, leadHint, levelLabel, roleLabel, rolesFor } from "../../lib/pools";

const LEVEL_WORDS: Record<Draft["level"], string> = {
  read: "Read: see the roster, cards, tasks and their own messages. Anything more asks you in Needs you.",
  work_together: "Work together: status, messages, shared docs and tasks. Anything more asks you in Needs you.",
  full: "Full: Work together, and messages go straight into members' sessions, within the loop limits.",
};

export const RULE_WORDS: Record<TalkRule, string> = { allowed: "Allowed", ask: "Ask first", blocked: "Blocked" };
const RULE_TONE: Record<TalkRule, string> = {
  allowed: "bg-success-soft text-success",
  ask: "bg-warning-soft text-warning",
  blocked: "bg-danger-soft text-danger",
};

export function NewPoolRoles({ draft, onDraft }: { draft: Draft; onDraft: (next: Draft) => void }): JSX.Element {
  const setMember = (ref: string, change: Partial<Draft["members"][number]>) =>
    onDraft({ ...draft, members: draft.members.map((member) => (member.ref === ref ? { ...member, ...change } : member)) });
  // A Lead no row can take is said apart, with how one gets chosen (D-92).
  const later = leadLater(draft);
  const toChoose = rolesToChoose(draft).filter((role) => !(later && role === "lead"));
  const cycle = (from: PoolRole, to: PoolRole) => onDraft({ ...draft, talk: { ...draft.talk, [pairKey(from, to)]: nextRule(ruleOf(draft.talk, from, to)) } });
  return (
    <div className="flex flex-col gap-5">
      <section className="flex flex-col gap-2" aria-label="Roles">
        <h3 className="text-body font-semibold text-ink">Roles</h3>
        <p className="text-meta text-ink-2">One Lead; a Reviewer reads everything, gives its verdict on work in review and doesn't build.</p>
        {draft.members.length > 0 && toChoose.length > 0 && (
          <p className="text-meta font-medium text-ink" data-testid="new-pool-roles-to-choose">
            Still to choose: {toChoose.map(roleLabel).join(", ")}. Pick {toChoose.length === 1 ? "it" : "them"} in the list below.
          </p>
        )}
        {later && (
          <p className="text-meta font-medium text-ink" data-testid="new-pool-lead-later">
            {later}
          </p>
        )}
        {draft.members.length === 0 ? (
          <p className="text-meta text-ink-2">No members yet. Go back to add some, or set them on the canvas later.</p>
        ) : (
          <ul className="flex flex-col divide-y divide-hairline/60 rounded-card border border-hairline bg-surface" data-testid="new-pool-roles">
            {draft.members.map((member) => (
              <li key={member.ref} className="flex flex-wrap items-center gap-3 px-3 py-2">
                <span className="min-w-0 flex-1 truncate text-body font-medium text-ink">{member.label}</span>
                <Segmented
                  label={`${member.label}'s role`}
                  value={member.role}
                  segments={rolesFor(member.kind).map((role) => ({ value: role, label: roleLabel(role) }))}
                  onChange={(role) => setMember(member.ref, { role })}
                />
                <Picker
                  choice
                  size="sm"
                  ariaLabel={`${member.label}'s level`}
                  value={member.level ?? "pool"}
                  options={[
                    { value: "pool", label: `Level: pool's (${levelLabel(draft.level)})` },
                    { value: "read", label: "Level: Read" },
                    { value: "work_together", label: "Level: Work together" },
                    { value: "full", label: "Level: Full" },
                  ]}
                  onChange={(level) => setMember(member.ref, { level: level === "pool" ? null : level })}
                />
                {leadHint(member.kind) && (
                  <span className="basis-full text-meta text-ink-2" data-testid="new-pool-lead-hint">
                    {leadHint(member.kind)}
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="flex flex-col gap-2" aria-label="Who may talk to whom">
        <h3 className="text-body font-semibold text-ink">Who may talk to whom</h3>
        <div className="overflow-x-auto">
          <table className="border-separate border-spacing-1 text-meta" data-testid="new-pool-talk">
            <thead>
              <tr>
                <th className="px-1.5 text-left text-label font-semibold uppercase tracking-wide text-muted">From ↓ · To →</th>
                {TALK_ROLES.map((to) => (
                  <th key={to} className="px-1.5 text-left text-label font-semibold uppercase tracking-wide text-muted">
                    {roleLabel(to)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {TALK_ROLES.map((from) => (
                <tr key={from}>
                  <th scope="row" className="px-1.5 text-left font-semibold text-ink">
                    {roleLabel(from)}
                  </th>
                  {TALK_ROLES.map((to) => {
                    if (from === to && from !== "member") return <td key={to} className="min-w-24 rounded-md bg-surface-3 px-2 py-1.5 text-center text-muted">—</td>;
                    const rule = ruleOf(draft.talk, from, to);
                    return (
                      <td key={to} className="p-0">
                        <button
                          type="button"
                          onClick={() => cycle(from, to)}
                          aria-label={`${roleLabel(from)} to ${roleLabel(to)}: ${RULE_WORDS[rule]}. Click to change.`}
                          className={"min-w-24 rounded-md px-2 py-1.5 text-center font-semibold outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + RULE_TONE[rule]}
                          data-testid="new-pool-talk-cell"
                          data-pair={`${from}>${to}`}
                        >
                          {RULE_WORDS[rule]}
                        </button>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="text-meta text-ink-2">Click a cell to change it: Allowed, then Ask first, then Blocked. One pair of members can differ: click its line on the canvas.</p>
        <p className="text-meta text-ink-2" data-testid="new-pool-talk-claude">
          {CLAUDE_CHANNEL_NOTE}
        </p>
      </section>

      <section className="flex flex-col gap-2" aria-label="What agents may do">
        <h3 className="text-body font-semibold text-ink">What agents may do</h3>
        <Segmented
          label="The pool's level"
          value={draft.level}
          segments={[
            { value: "read", label: "Read" },
            { value: "work_together", label: "Work together" },
            { value: "full", label: "Full" },
          ]}
          onChange={(level) => onDraft({ ...draft, level })}
          testId="new-pool-level"
          className="self-start"
        />
        <p className="text-meta text-ink-2">{LEVEL_WORDS[draft.level]}</p>
        <p className="text-meta text-ink-2">Waking an idle session with a message: only at Full, or by a rule you make.</p>
      </section>

      <label className={"flex cursor-pointer items-start gap-3 rounded-card border p-3 " + (draft.sealed ? "border-danger/60 bg-danger-soft" : "border-danger/30 bg-surface")} data-testid="new-pool-seal">
        <Checkbox checked={draft.sealed} onChange={(event) => onDraft({ ...draft, sealed: event.target.checked })} className="mt-1" />
        <span className="flex flex-col gap-0.5">
          <span className="text-body font-semibold text-ink">Seal this pool</span>
          <span className="text-meta text-ink-2">Its sessions leave other pools. No links, and no messages or docs out through AI Juggler. Claude Code sessions AI Juggler starts here also leave Claude's own messages between sessions, from their next start.</span>
        </span>
      </label>
    </div>
  );
}
