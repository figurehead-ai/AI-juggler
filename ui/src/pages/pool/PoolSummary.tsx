/**
 * AJ-E3011 round 3 (#2, #5): the pool's own inspector opens on a short
 * summary in sentences — who's in, who leads, what they may do on their
 * own, the team tools — read by the same functions and component as New
 * pool's Review (PoolSentences), each with its Change; then what waits for
 * the user, naming Needs you and what it is (the walks didn't know it).
 */
import type { JSX } from "react";

import { useActionHost } from "../../components/ItemMenu";
import type { PoolRuleInfo, PoolRulesInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { talkOf, type PoolShape } from "../../lib/pool-sentences";
import { NEEDS_YOU_WHAT, asRole, type SETUP_STEPS } from "../../lib/pool-words";
import type { ToolsPlace } from "../../lib/team-tools";
import { PoolSentences, type SentenceRow } from "./setup/PoolSentences";
import { useTeamTools } from "./setup/TeamTools";
import type { MemberNode } from "./useCanvas";

type Step = (typeof SETUP_STEPS)[number];

/** The summary's rows, and the setup step each one's Change opens. */
const ROWS: readonly SentenceRow[] = ["who", "lead", "own", "tools"];
const STEP_OF: Partial<Record<SentenceRow, Step>> = { who: "who", lead: "lead", own: "own" };

/** A made pool as its sentences read it: the sessions on its diagram, by
 * name, and its wider members (Apps, Accounts, everyone…). */
export function shapeOfPool(pool: Pick<PoolInfo, "access" | "sealed">, rules: PoolRulesInfo | null, members: readonly Pick<MemberNode, "id" | "name" | "role">[], ruleLabel: (rule: PoolRuleInfo) => string): PoolShape {
  const broad = (rules?.members ?? []).filter((rule) => rule.kind !== "session" && !rule.member.startsWith("conversation:"));
  return {
    members: [
      ...members.map((member) => ({ ref: `session:${member.id}`, label: member.name, role: asRole(member.role), level: null, later: false })),
      ...broad.map((rule) => ({ ref: rule.member, label: ruleLabel(rule), role: asRole(rule.role), level: rule.level, later: rule.member !== "all" })),
    ],
    leadNext: rules?.lead_next ? `app:${rules.lead_next}` : null,
    talk: talkOf(rules?.talk ?? []),
    level: pool.access.level,
    sealed: pool.sealed,
  };
}

/** "Needs you", said with what it is, and a way there. */
export function NeedsYou(): JSX.Element {
  const host = useActionHost();
  return (
    <>
      <button type="button" onClick={() => host.openHome?.()} className="rounded font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="needs-you-link">
        Needs you
      </button>{" "}
      ({NEEDS_YOU_WHAT})
    </>
  );
}

export function PoolSummary({
  shape,
  where,
  needs,
  tools,
  onOpen,
}: {
  shape: PoolShape;
  where: string;
  /** What waits on the user here. */
  needs: number;
  tools: { projects: readonly ToolsPlace[]; apps: readonly ToolsPlace[] };
  onOpen: (step: Step) => void;
}): JSX.Element {
  const state = useTeamTools(tools.projects, tools.apps);
  return (
    <section className="mx-4 mb-3 flex flex-col gap-1.5" aria-label="This pool in short" data-testid="pool-summary">
      <PoolSentences
        compact
        shape={shape}
        where={where}
        rows={ROWS}
        tools={state}
        changeOf={(row) => {
          const step = STEP_OF[row];
          return step ? () => onOpen(step) : undefined;
        }}
        testId="pool-summary-sentences"
      />
      <p className="text-meta text-ink-2" data-testid="pool-summary-needs">
        {needs > 0 ? `${needs} ${needs === 1 ? "thing waits" : "things wait"} for your OK in ` : "Nothing waits for your OK. When something does, it's in "}
        <NeedsYou />.
      </p>
    </section>
  );
}
