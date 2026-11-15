/**
 * 0.4.6 (AJ-E3011): a pool page's setup checklist — its rows in the
 * setup's order, ticked from the pool and the steps seen here — and where
 * each step opens: the scope sheet, the palette, or the pool's own section
 * in the inspector (the same controls as the setup).
 */
import { useEffect, useState } from "react";

import type { PoolRuleInfo, PoolRulesInfo, WatchedMemberInfo } from "../../lib/ipc/pool-model";
import { ruleColumnRoom } from "./RuleNodes";
import type { PoolInfo } from "../../lib/ipc/pools";
import { markSeen, seenSteps, setupRows, SETUP_SEEN, type SetupRow } from "../../lib/pool-setup";
import { everyoneLabel, whereWords, type SETUP_STEPS } from "../../lib/pool-words";
import { cliFolderOf, leadFolder } from "../../lib/start-lead";
import type { ToolsPlace } from "../../lib/team-tools";

export type SetupStepId = (typeof SETUP_STEPS)[number];

/** A pool's projects in words: "web", "web and docs", "every project". */
export function poolWhere(pool: Pick<PoolInfo, "scope_all" | "scope">, projects: readonly { id: string; name: string }[]): string {
  if (pool.scope_all) return whereWords(true, []);
  return whereWords(
    false,
    pool.scope.map((id) => (id === null ? "sessions with no project" : (projects.find((one) => one.id === id)?.name ?? "a project"))),
  );
}

/** The pool's everyone-in-scope member by name: "Everyone in web". */
export function everyoneOf(pool: Pick<PoolInfo, "scope_all" | "scope">, projects: readonly { id: string; name: string }[]): string {
  return pool.scope_all ? everyoneLabel(true, []) : `Everyone in ${poolWhere(pool, projects)}`;
}

export function usePoolSetup(input: {
  pool: Pick<PoolInfo, "id" | "kind" | "scope_all" | "scope">;
  projects: readonly { id: string; name: string; root?: string }[];
  /** It has who's in it. */
  whoIn: boolean;
  /** Its member rules, and the roles of the sessions on its canvas. */
  rules: readonly PoolRuleInfo[];
  roles: readonly string[];
  /** Its rules as read (the wait for an App's next session among them). */
  info: PoolRulesInfo | null;
  /** Its Watched members (they take the canvas's column too). */
  watched: readonly WatchedMemberInfo[];
  ruleLabel: (rule: PoolRuleInfo) => string;
  /** Where (a pool of yours): its scope sheet. */
  onScope: () => void;
  /** Who's in: the palette. */
  onWho: () => void;
  /** The other steps: the pool's section in the inspector. */
  onStep: (step: SetupStepId) => void;
}): {
  rows: SetupRow[];
  open: (step: SetupStepId) => void;
  where: string;
  leadCard: boolean;
  leadNextName: string | null;
  /** The App whose next session leads, by member ref and name. */
  leadNext: { ref: string; name: string } | null;
  /** The pool's one project folder ("Start one now" opens there). */
  leadDir: string | null;
  /** Its projects and CLI Apps, where the team tools are switched. */
  tools: { projects: ToolsPlace[]; apps: ToolsPlace[] };
  room: number;
} {
  const { pool } = input;
  const [seen, setSeen] = useState(() => seenSteps(pool.id));
  useEffect(() => {
    const changed = (event: Event) => {
      if (event instanceof CustomEvent && event.detail === pool.id) setSeen(seenSteps(pool.id));
    };
    window.addEventListener(SETUP_SEEN, changed);
    return () => window.removeEventListener(SETUP_SEEN, changed);
  }, [pool.id]);
  const led = input.rules.some((row) => row.role === "lead") || input.roles.includes("lead");
  const leadNext = input.info?.lead_next ?? null;
  const nextRule = leadNext === null ? undefined : input.rules.find((row) => row.member === `app:${leadNext}`);
  const rows = setupRows({ own: pool.kind === "default", every: pool.scope_all, whoIn: input.whoIn, led, leadNext: leadNext !== null }, seen);
  // The lead's place on the canvas: while no session leads a pool that
  // has who's in it.
  const leadCard = !led && input.whoIn;
  const open = (step: SetupStepId) => {
    if (step === "who") return input.onWho();
    markSeen(pool.id, [step]);
    if (step === "where" && pool.kind !== "default") return input.onScope();
    input.onStep(step === "where" ? "who" : step);
  };
  return {
    rows,
    open,
    where: poolWhere(pool, input.projects),
    leadCard,
    leadNextName: nextRule ? input.ruleLabel(nextRule) : null,
    leadNext: nextRule ? { ref: nextRule.member, name: input.ruleLabel(nextRule) } : null,
    leadDir: leadFolder(pool, input.projects),
    tools: {
      projects: pool.scope_all ? [] : input.projects.filter((one) => pool.scope.includes(one.id)).map((one) => ({ id: one.id, name: one.name })),
      apps: input.rules.flatMap((rule) => {
        const folder = cliFolderOf(rule.member);
        return folder ? [{ id: folder, name: input.ruleLabel(rule) }] : [];
      }),
    },
    room: ruleColumnRoom(input.rules, input.watched, leadCard),
  };
}
