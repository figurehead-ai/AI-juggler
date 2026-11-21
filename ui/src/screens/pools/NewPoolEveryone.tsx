/**
 * AJ-E3011 step 4, Who builds, who checks (§2, §4): the four roles as a
 * plain reference, then each member's role, everyone in the projects
 * too, and (if wanted) what one member may do on its own when it differs
 * from the pool.
 *
 * D-126: each member a row with its logo (its role's mark at the
 * corner), its name and what it is, and its role as one segmented
 * control: Member, Reviewer, Observer. Who leads is the step before's: a
 * session that leads says so here; an App whose next session leads says
 * "Its next session leads; the role here is for its other sessions." and its control sets the
 * others' role, keeping the lead (round 2, #2: one row, not a Lead
 * beside a Member). Every new session's role is the same control
 * (everyone joins at all is Who's in's switch).
 */
import type { JSX } from "react";

import { Disclosure, Picker, Segmented } from "../../components/controls";
import { LogoTile } from "../../components/ToolLogo";
import { useApps, type AppModel } from "../../lib/apps";
import type { PoolRole } from "../../lib/ipc/pools";
import type { Candidate, Draft, DraftMember } from "../../lib/new-pool";
import { leadCandidates, withReviewer } from "../../lib/new-pool-lead";
import { templateById } from "../../lib/pool-templates";
import { APP_KIND_WORDS, JOINS_BY_ITSELF, ROLE_CARDS, roleCard } from "../../lib/pool-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { MemberOwnLevel } from "../../pages/pool/setup/OwnLevelChoice";
import { logoOf, MemberLogo, ROLE_ICONS } from "../../components/MemberLogo";
import { StepHead } from "./StepHead";

/** The roles a member's control offers (the lead is Who leads'). */
const CHOOSE: readonly PoolRole[] = ["member", "reviewer", "observer"];

/** What a member is, in a few words: "Desktop app · Profile Personal". */
function memberKindWords(member: Pick<DraftMember, "ref" | "kind">, apps: readonly AppModel[]): string {
  if (member.kind === "app") {
    const app = apps.find((one) => `app:${one.id}` === member.ref);
    if (!app) return member.ref.startsWith("app:cli:") ? APP_KIND_WORDS.cli : "App";
    return [APP_KIND_WORDS[app.kind], app.profile ? `Profile ${app.profile}` : null].filter(Boolean).join(" · ");
  }
  return { session: "One session", account: "Every session of an account", tool: "Every session of a tool", mac: "Every session on a computer", everyone: JOINS_BY_ITSELF }[member.kind];
}

function RoleControl({ name, value, onChange }: { name: string; value: PoolRole; onChange: (role: PoolRole) => void }): JSX.Element {
  return (
    <Segmented<PoolRole>
      label={`${name}'s role`}
      value={value}
      segments={CHOOSE.map((role) => ({ value: role, label: roleCard(role).label, icon: ROLE_ICONS[role] }))}
      onChange={onChange}
      testId="new-pool-member-role"
      className="shrink-0"
    />
  );
}

/** SK-13: a template that promises a Reviewer ("Writer and reviewer")
 * names one here: a member already in, or a session open now, added as
 * the Reviewer. Shown while no named member reviews. */
function NameReviewer({ draft, onDraft, nameOf, candidates }: { draft: Draft; onDraft: (next: Draft) => void; nameOf: (ref: string, label: string) => string; candidates: readonly Candidate[] }): JSX.Element | null {
  const named = draft.members.filter((member) => member.ref !== "all" && member.role !== "lead" && member.ref !== draft.leadNext);
  const open = leadCandidates(candidates).filter((row) => !draft.members.some((member) => member.ref === row.ref));
  const options = [...named.map((member) => ({ value: member.ref, label: nameOf(member.ref, member.label) })), ...open.map((row) => ({ value: row.ref, label: row.label, hint: row.detail }))];
  return (
    <li className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-card bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-name-reviewer">
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="text-body font-semibold text-ink">Who checks the work</span>
        <span className="text-meta text-ink-2">{options.length > 0 ? "This template has a Reviewer: pick the session that checks the others' work." : `No session to pick yet: open one in ${draft.every ? "a project" : "its project"}, or add it in Who's in.`}</span>
      </span>
      {options.length > 0 && (
        <Picker
          groups={[{ label: "In the pool", values: named.map((member) => member.ref) }, { label: "Open now", values: open.map((row) => row.ref) }]}
          size="sm"
          ariaLabel="The Reviewer"
          heading="Who checks the work"
          placeholder="Pick a session…"
          value={null}
          options={options}
          onChange={(ref) => onDraft(withReviewer(draft, ref, open.find((row) => row.ref === ref)))}
          testId="new-pool-reviewer-pick"
        />
      )}
    </li>
  );
}

export function NewPoolEveryone({
  draft,
  onDraft,
  where,
  nameOf,
  open = [],
  apps: given,
  candidates = [],
}: {
  /** The sessions that could join (a Reviewer can be named from them). */
  candidates?: readonly Candidate[];
  draft: Draft;
  onDraft: (next: Draft) => void;
  where: string;
  nameOf: (ref: string, label: string) => string;
  open?: readonly InstanceInfo[];
  /** Every App, the CLI ones too (each row's logo and what it is). */
  apps?: readonly AppModel[] | null;
}): JSX.Element {
  const read = useApps();
  const apps = given ?? read ?? [];
  const setMember = (ref: string, change: Partial<DraftMember>) => onDraft({ ...draft, members: draft.members.map((member) => (member.ref === ref ? { ...member, ...change } : member)) });
  const everyone = draft.members.find((member) => member.ref === "all");
  const named = draft.members.filter((member) => member.ref !== "all");
  const own = named.some((member) => member.level !== null);
  return (
    <div className="flex flex-col gap-7" data-testid="new-pool-step-everyone">
      <StepHead step="everyone" />
      <ul aria-label="What each role does" className="grid grid-cols-2 gap-2.5 sm:grid-cols-4" data-testid="new-pool-role-cards">
        {ROLE_CARDS.map((card) => (
          <li key={card.id} className="flex flex-col gap-2 rounded-xl bg-surface-2 p-3" data-testid="role-card" data-role={card.id}>
            <LogoTile icon={ROLE_ICONS[card.id]} size={24} />
            <span className="flex flex-col">
              <span className="text-meta font-semibold text-ink">{card.label}</span>
              <span className="text-label text-muted">{card.line}</span>
            </span>
          </li>
        ))}
      </ul>
      <section aria-labelledby="new-pool-each-title" className="flex flex-col gap-2.5">
        <h4 id="new-pool-each-title" className="section-label">
          Each member
        </h4>
        <ul className="flex flex-col gap-2" data-testid="new-pool-roles">
          {named.map((member) => {
            const name = nameOf(member.ref, member.label);
            const leadsNext = member.ref === draft.leadNext;
            const leads = member.role === "lead";
            return (
              <li key={member.ref} className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-card bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-member-row" data-leads-next={leadsNext || undefined}>
                <MemberLogo spec={logoOf(member.ref, { apps, sessions: open })} size={32} badge={leads || leadsNext ? "lead" : member.role} />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-body font-semibold text-ink" data-testid="new-pool-member-name">
                    {name}
                  </span>
                  <span className="text-meta text-ink-2" data-testid="new-pool-member-line">
                    {leads ? "Leads the team. Change it in Who leads." : leadsNext ? "Its next session leads; the role here is for its other sessions." : memberKindWords(member, apps)}
                  </span>
                </span>
                {!leads && <RoleControl name={leadsNext ? `${name}'s other sessions` : name} value={member.role} onChange={(role) => setMember(member.ref, { role })} />}
              </li>
            );
          })}
          {everyone && (
            <li className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-card bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-member-row" data-everyone>
              <MemberLogo spec={logoOf("all", { apps, sessions: open })} size={32} badge={everyone.role} />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-body font-semibold text-ink">Every new session in {where}</span>
                <span className="text-meta text-ink-2">{JOINS_BY_ITSELF}</span>
              </span>
              <RoleControl name="Every new session" value={everyone.role === "lead" ? "member" : everyone.role} onChange={(role) => setMember("all", { role })} />
            </li>
          )}
          {templateById(draft.template).roles.includes("reviewer") && !named.some((member) => member.role === "reviewer") && <NameReviewer draft={draft} onDraft={onDraft} nameOf={nameOf} candidates={candidates} />}
          {named.length === 0 && !everyone && <li className="text-meta text-ink-2">No one yet: add members in Who's in.</li>}
        </ul>
      </section>
      {named.length > 0 && (
        <Disclosure
          open={own}
          summary={
            <span>
              One member works differently on its own <span className="font-normal text-faint">· optional, per member</span>
            </span>
          }
          tone="heading"
          testId="new-pool-own-fold"
        >
          {/* Round 6 (Sam): "On its own: Same as the pool" said nothing. */}
          <p className="mt-2 text-meta text-ink-2" data-testid="new-pool-own-hint">
            What this member may do without asking, if different from the pool.
          </p>
          <ul className="mt-2 flex flex-col gap-1.5">
            {named.map((member) => (
              <li key={member.ref} className="flex flex-wrap items-center gap-x-3 gap-y-1">
                <span className="min-w-0 flex-1 truncate text-meta text-ink">{nameOf(member.ref, member.label)}</span>
                <MemberOwnLevel name={nameOf(member.ref, member.label)} value={member.level} poolLevel={draft.level} onChange={(level) => setMember(member.ref, { level })} testId="new-pool-member-level" />
              </li>
            ))}
          </ul>
        </Disclosure>
      )}
    </div>
  );
}
