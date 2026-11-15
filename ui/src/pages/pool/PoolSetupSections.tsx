/**
 * 0.4.6 (AJ-E3011 §1): the pool's setup, after it's made, in the setup's
 * order and with the same controls: who's in, who leads (the lead picker),
 * everyone else (the join line and the role cards), who may message whom
 * (the talk table) and what they may do on their own. The canvas's
 * checklist opens each one here (`focus`), scrolled into view.
 */
import { useEffect, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { POOL_RULES_CHANGED, poolJoin, poolLeave, poolSetTalk, type PoolRuleInfo, type TalkRule } from "../../lib/ipc/pool-model";
import { setPoolLead } from "./pool-lead";
import { poolSetAccess, type PoolInfo, type PoolRole } from "../../lib/ipc/pools";
import { nextRule } from "../../lib/pool-templates";
import type { LeadChoice } from "../../lib/new-pool";
import { STEP_WORDS, asOwnLevel, asRole, asTalkRule, joinSentence, nextSessionWords } from "../../lib/pool-words";
import { cliFolderOf, startNextLead } from "../../lib/start-lead";
import { setPools } from "../../lib/pools";
import type { PoolRules } from "../../lib/pool-rules";
import { sessionState } from "../../lib/rail-items";
import { FoldSection, Section } from "./InspectPool";
import { JoinAs } from "./setup/JoinAs";
import { LeadPicker } from "./setup/LeadPicker";
import { OwnLevelChoice } from "./setup/OwnLevelChoice";
import { RoleCards, holdersOf } from "./setup/RoleCards";
import { TalkTable } from "./setup/TalkTable";
import type { MemberNode } from "./useCanvas";
import type { Selection } from "./Canvas";

const failed = (what: string) => (cause: unknown) => showToast({ message: `Couldn't ${what}`, detail: errorText(cause), tone: "error" });

/** The pool's role-pair talk as the table reads it. */
function roleRule(rules: PoolRules["rules"], from: PoolRole, to: PoolRole): TalkRule {
  const row = rules?.talk.find((one) => one.from === `role:${from}` && one.to === `role:${to}`);
  return asTalkRule(row?.rule);
}

export function PoolSetupSections({
  pool,
  rules,
  members,
  ruleLabel,
  where,
  selection,
  onAddMembers,
  leadDir = null,
}: {
  pool: PoolInfo;
  rules: PoolRules;
  /** The canvas's session members. */
  members: readonly MemberNode[];
  ruleLabel: (rule: PoolRuleInfo) => string;
  where: string;
  /** The canvas's selection: a step to scroll to (the checklist's). */
  selection: Selection;
  onAddMembers: () => void;
  /** The pool's one project folder: "Start one now" opens New session there. */
  leadDir?: string | null;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const focus = selection.kind === "pool" ? selection.focus : undefined;
  useEffect(() => {
    // Only the inspector's own list scrolls (scrollIntoView moved the whole
    // window: the app's frame slid up under the gallery's capture).
    const section = focus ? document.getElementById(`pool-setup-${focus}`) : null;
    const list = section?.closest<HTMLElement>("[data-canvas-scroll]");
    if (section && list) list.scrollTop += section.getBoundingClientRect().top - list.getBoundingClientRect().top;
  }, [focus]);
  const run = (what: string, work: () => Promise<unknown>) => {
    setBusy(true);
    void work()
      .then(() => {
        rules.reload();
        window.dispatchEvent(new CustomEvent(POOL_RULES_CHANGED, { detail: pool.id }));
      }, failed(what))
      .finally(() => setBusy(false));
  };
  const memberRules = rules.rules?.members ?? [];
  const apps = memberRules.filter((rule) => rule.kind === "app").map((rule) => ({ ref: rule.member, name: ruleLabel(rule), startable: cliFolderOf(rule.member) !== null, open: rule.sessions }));
  const leads = members.filter((member) => member.role === "lead");
  const open = members.filter((member) => sessionState(member.session).state !== "ended");
  const leadNext = rules.rules?.lead_next ?? null;
  const value: LeadChoice = leads[0] ? { kind: "session", ref: leads[0].id } : leadNext ? { kind: "next", app: `app:${leadNext}` } : { kind: "none" };
  const chooseLead = (choice: LeadChoice) => run("change who leads", () => setPoolLead(pool.id, leads.map((lead) => lead.id), choice));
  const everyone = memberRules.find((rule) => rule.member === "all");
  const nextName = leadNext ? (apps.find((app) => app.ref === `app:${leadNext}`)?.name ?? "An App") : null;
  const holders = holdersOf([
    ...members.map((member) => ({ name: member.name, role: member.role })),
    ...memberRules.filter((rule) => rule.kind !== "session" && rule.member !== "all").map((rule) => ({ name: rule.member === `app:${leadNext}` ? `${ruleLabel(rule)}'s other sessions` : ruleLabel(rule), role: rule.role })),
    ...(everyone ? [{ name: "new sessions", role: everyone.role }] : []),
    ...(nextName ? [{ name: nextSessionWords(nextName), role: "lead" }] : []),
  ]);
  const level = pool.access.level;
  return (
    <div className="flex flex-col" data-testid="pool-setup-sections">
      <div id="pool-setup-who">
        <Section title="Who's in" testId="pool-setup-who">
          <p className="text-meta text-ink-2">
            {members.length} {members.length === 1 ? "session" : "sessions"} on the diagram
            {memberRules.some((rule) => rule.kind !== "session") ? ", and the Apps or groups whose new sessions join by themselves" : ""}.
          </p>
          <Button size="sm" variant="ghost" className="self-start" onClick={onAddMembers} data-testid="pool-setup-add">
            Add a session or an App
          </Button>
        </Section>
      </div>
      <div id="pool-setup-lead">
        <Section title="Who leads" testId="pool-setup-lead">
          <p className="text-meta text-ink-2">{STEP_WORDS.lead.why}</p>
          <LeadPicker
            sessions={open.map((member) => ({ ref: member.id, name: member.name, detail: sessionState(member.session).word }))}
            apps={apps}
            value={value}
            onChange={chooseLead}
            disabled={busy}
            where={where}
            onStart={(app) => startNextLead(app, leadDir)}
            testId="pool-lead"
          />
        </Section>
      </div>
      <div id="pool-setup-everyone">
        <Section title="Everyone else" testId="pool-setup-everyone">
          <JoinAs
            where={where}
            value={(everyone ? asRole(everyone.role) : null)}
            disabled={busy || (pool.kind === "default" && !everyone)}
            onChange={(role) =>
              run("change how new sessions join", async () => {
                if (role === null) setPools(await poolLeave(pool.id, "all"));
                else setPools(await poolJoin(pool.id, "all", role, everyone?.level ?? null));
              })
            }
            testId="pool-join"
          />
          <p className="text-meta text-ink-2" data-testid="pool-join-sentence">
            {joinSentence(where, (everyone ? asRole(everyone.role) : null), everyone?.level ?? level)}
          </p>
          <RoleCards holders={holders} testId="pool-role-cards" />
          <p className="text-meta text-muted">Change one member's role by clicking its card on the diagram.</p>
        </Section>
      </div>
      <div id="pool-setup-talk">
        <FoldSection title={STEP_WORDS.talk.title} line={STEP_WORDS.talk.why} open={focus === "talk"} testId="pool-setup-talk">
          <TalkTable
            compact
            disabled={busy}
            ruleOf={(from, to) => roleRule(rules.rules, from, to)}
            onCycle={(from, to) => {
              const next = nextRule(roleRule(rules.rules, from, to), from);
              run("change who may message whom", () => poolSetTalk(pool.id, `role:${from}`, `role:${to}`, next === "allowed" ? null : next));
            }}
            testId="pool-talk"
          />
        </FoldSection>
      </div>
      <div id="pool-setup-own">
        <FoldSection title={STEP_WORDS.own.title} line={STEP_WORDS.own.why} open={focus === "own"} testId="pool-setup-own">
          <OwnLevelChoice
            value={level === "custom" ? "custom" : asOwnLevel(level)}
            label="What sessions in this pool may do on their own"
            disabled={busy}
            onChange={(next) => run("change what they may do", async () => setPools(await poolSetAccess(pool.id, next)))}
            onCustom={() => run("change what they may do", async () => setPools(await poolSetAccess(pool.id, "custom", pool.access.actions)))}
            testId="pool-own"
          />
          <p className="text-label text-muted" data-testid="pool-applies-next">
            Changes apply from each session's next call; nothing it is doing now is stopped.
          </p>
        </FoldSection>
      </div>
    </div>
  );
}
