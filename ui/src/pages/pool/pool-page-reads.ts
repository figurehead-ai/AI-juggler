/**
 * A pool page's reads (moved out of PoolPage.tsx, 0.4.6): the palette's
 * other candidates, a member rule's name, and an App or Tool member's
 * shared tool.
 */
import { useCallback, useMemo, useState } from "react";

import type { LogoSources } from "../../components/MemberLogo";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { useAgentBridge } from "../../lib/ipc/agent-bridge";
import { useApps } from "../../lib/apps";
import { useSubscriptions } from "../../lib/subscriptions";
import { useTools } from "../../lib/tools";
import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { broadCandidates, draftOf, memberLabel, memberTool, sessionCandidates, thisMacOf, type Candidate } from "../../lib/new-pool";
import { usePairedMacs } from "../../lib/ipc/fleet-launch";
import { templateById } from "../../lib/pool-templates";
import { usePools } from "../../lib/pools";
import { useConversations } from "../../lib/conversations";
import type { MoreKind } from "./PaletteMore";
import { nameOf } from "./actions";

/** D-62: the palette's other candidates for `pool`, by kind, as the
 * wizard lists them (one rule for both), less the members it has. */
export function useMoreCandidates(pool: PoolInfo, sessions: InstanceInfo[], projects: ProjectInfo[], accounts: AccountInfo[], members: string[]): Record<MoreKind, Candidate[]> {
  const pools = usePools();
  const tools = useTools();
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const paired = usePairedMacs();
  const { all } = useConversations();
  const bridge = useAgentBridge();
  // When the page opened: what is recent (a week) is read from it.
  const [opened] = useState(() => Date.now());
  return useMemo(() => {
    const draft = { ...draftOf(templateById("blank"), null), every: pool.scope_all, projects: pool.scope };
    const projectName = (id: string | null) => (id === null ? "No project" : (projects.find((one) => one.id === id)?.name ?? "A project"));
    const named = new Set(members);
    const own = sessionCandidates({ draft, sessions, conversations: all, pools, tools, bridge, projectName, nameOf, now: opened }).filter((row) => !named.has(row.ref) && !(pools?.placements ?? []).some((place) => `session:${place.session_id}` === row.ref && place.pool_id === pool.id));
    const thisMac = thisMacOf(sessions);
    const broad = (kind: "app" | "account" | "tool" | "mac") => broadCandidates({ kind, apps: apps ?? [], subscriptions: subscriptions ?? [], tools, sessions, thisMac, paired, bridge }).filter((row) => !named.has(row.ref));
    return {
      watched: own.filter((row) => row.reach === "watched" && row.blocked === null),
      app: broad("app"),
      account: broad("account"),
      tool: broad("tool"),
      mac: broad("mac"),
      everyone: named.has("all") ? [] : [{ ref: "all", kind: "everyone", label: "Everyone in this pool's projects", detail: "every session in scope, now and later", group: "open", reach: "later", blocked: null, lastMs: 0 }],
      // Open sessions waiting to join; an ended one's place is History
      // (the P3 walk: "docs · Ended" offered Add docs, which did nothing).
      blocked: own.filter((row) => row.blocked !== null && row.group === "open"),
    };
  }, [pool, sessions, projects, members, pools, tools, bridge, apps, subscriptions, all, paired, opened]);
}

/** D-62: a member rule's name, from the same Apps, Accounts and Tools the
 * other screens list. */
export function useRuleLabel(accounts: AccountInfo[], sessions: InstanceInfo[], everyone = "Everyone in this pool's projects"): (rule: PoolRuleInfo) => string {
  const label = useMemberLabel(accounts, sessions, everyone);
  return useCallback((rule: PoolRuleInfo) => label(rule.member), [label]);
}

/** A broad member's name by its ref (an App, an Account, a tool, a Mac),
 * as the canvas says it; the Pools cards say it the same (SK-9: a card
 * read "An Account" where the canvas said "Personal"). */
export function useMemberLabel(accounts: AccountInfo[], sessions: InstanceInfo[], everyone = "Everyone in this pool's projects"): (member: string) => string {
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const tools = useTools();
  const paired = usePairedMacs();
  return useCallback(
    // AJ-E3011: everyone by its scope ("Everyone in web").
    (member: string) => (member === "all" ? everyone : memberLabel(member, { apps: apps ?? [], subscriptions: subscriptions ?? [], tools, thisMac: thisMacOf(sessions), paired })),
    [apps, subscriptions, tools, sessions, paired, everyone],
  );
}

/** D-62 (the P8 walk): the tool and App an App or Tool member's sessions
 * all share, so its node and inspector say "Shell only" as theirs do. */
export function useRuleTool(accounts: AccountInfo[]): (rule: PoolRuleInfo) => { tool: string; account: string | null } | null {
  const apps = useApps(accounts);
  return useCallback((rule: PoolRuleInfo) => memberTool(rule.member, apps ?? []), [apps]);
}

/** D-126 §3: where a member's logo is looked up (its Apps, the sessions
 * open now), for the shared MemberLogo. */
export function useLogoSources(accounts: AccountInfo[], sessions: InstanceInfo[]): LogoSources {
  const apps = useApps(accounts);
  return useMemo(() => ({ apps: apps ?? [], sessions }), [apps, sessions]);
}
