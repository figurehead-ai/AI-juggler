/**
 * D-62 (AJ-E3008): a member rule in the inspector: what it brings in
 * (its sessions now, and new ones), its role and what its sessions may do
 * on their own ("Same as the pool" by default), how they hear a message
 * (D-119), and taking it out. A desktop member (an App, or a session
 * another app runs) says how it's reached through its app's connector, or
 * why it's Watched, with the consent card (AJ-E3013); Continue as CLI is
 * in its card's menu. Its name, kind and role are the inspector's title
 * and subtitle (AJ-E3012).
 */
import { useState, type JSX } from "react";

import { Button } from "../../components/primitives";

import { ShellOnly } from "../../components/ShellOnly";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { poolJoin, type PoolRuleInfo, type WatchedMemberInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo, PoolRole } from "../../lib/ipc/pools";
import { setPools } from "../../lib/pools";
import { useApps } from "../../lib/apps";
import { CONNECTOR_MEANING, reachOf, useDesktopMembers, wakeLine } from "../../lib/connector";
import type { DesktopMemberInfo } from "../../lib/ipc/connector";
import { ConnectorCard } from "./connector/ConnectorCard";
import { asRole, leadSentence, wakeWords } from "../../lib/pool-words";
import { cliFolderOf } from "../../lib/start-lead";
import { usePendingRoles, withPendingRole } from "../../lib/pool-pending";
import type { LeadChoice } from "../../lib/new-pool";
import { MemberOwnLevel } from "./setup/OwnLevelChoice";
import { takeOutMember } from "./actions";
import { RolePicker, RoleWhy } from "./setup/RoleCards";
import { StartLeadButton } from "./setup/StartLead";

/** Who leads, as an App member's inspector sets it. */
export interface RuleLead {
  /** This App's next session leads now. */
  leadsNext: boolean;
  where: string;
  onStart: (member: string) => void;
  /** Opens the pool's Who leads. */
  onChange: () => void;
  onLead: (choice: LeadChoice) => Promise<void>;
}

export function InspectRule({
  pool,
  rule,
  watched,
  label,
  onChanged,
  onGone,
  shared = null,
  lead = null,
}: {
  /** Who leads, from here (round 3, #1): whether this App's next session
   * leads, setting it (the pool's own setter), "Start one now" (New
   * session on it in the pool's project) and opening Who leads. */
  lead?: RuleLead | null;
  /** The tool and App all its sessions share (an App or Tool member):
   * its "Shell only" is theirs (the P8 walk). */
  shared?: { tool: string; account: string | null } | null;
  pool: PoolInfo;
  rule: PoolRuleInfo | undefined;
  watched: WatchedMemberInfo | undefined;
  label: string;
  onChanged: () => void;
  onGone: () => void;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const member = rule?.member ?? watched?.member ?? "";
  const role: PoolRole = asRole(rule?.role ?? watched?.role ?? "member");
  const level = rule?.level ?? null;
  const run = async (what: string, call: () => Promise<unknown>) => {
    setBusy(true);
    try {
      const info = await call();
      if (info && typeof info === "object" && "pools" in (info as object)) setPools(info as Parameters<typeof setPools>[0]);
      onChanged();
    } catch (cause) {
      showToast({ message: `Couldn't ${what}`, detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const fixed = pool.kind === "default" && member === "all";
  const desktop = useDesktopMembers(pool.id)?.get(member);
  return (
    <div className="flex flex-col gap-4 px-4" data-testid="inspect-rule">
      <div className="flex flex-col gap-1">
        {shared && (
          <span className="flex flex-wrap items-center gap-2">
            <ShellOnly tool={shared.tool} account={shared.account} />
          </span>
        )}
        {!watched && (
          <span className="text-meta text-ink-2">
            {`${rule?.sessions ? `${rule.sessions} open here now` : "None open here now"}; its new sessions in this pool's projects join by themselves, and its ended ones are in the List view.`}
          </span>
        )}
      </div>
      {(watched || desktop) && <DesktopSection poolId={pool.id} member={member} watched={!!watched} desktop={desktop} />}
      <RoleSection member={member} label={label} role={role} lead={lead} poolId={pool.id} busy={busy} onRole={(next) => roleChange(next, { lead, run, poolId: pool.id, member, level })} />
      {!watched && !desktop && (
        <div className="flex flex-col gap-1.5">
          <span className="text-meta font-medium text-ink">What its sessions may do on their own</span>
          <MemberOwnLevel prefix="" name={label} value={level} poolLevel={pool.access.level} disabled={busy} onChange={(next) => void run("change what its sessions may do", () => poolJoin(pool.id, member, role, next))} testId="inspect-rule-level" />
          <span className="text-meta text-ink-2">Its own choice replaces the pool's for the sessions it brings in.</span>
          <span className="text-meta text-muted" data-testid="inspect-rule-wake">
            Messages: its sessions {wakeWords(level ?? pool.access.level, false, true)}.
          </span>
        </div>
      )}
      {!fixed && (
        // Held, like Stop (round 4): Undo in the toast keeps it.
        <Button variant="secondary" size="sm" className="self-start" disabled={busy} onClick={() => {
            takeOutMember(pool, member, label);
            onGone();
          }} data-testid="inspect-rule-leave">
          Take out of the pool
        </Button>
      )}
    </div>
  );
}

/** AJ-E3013: a desktop member, through its app's connector or Watched:
 * how a message reaches it, or why it can't take part, then the consent
 * card for its app (Add, or where it stands and Remove). */
function DesktopSection({ poolId, member, watched, desktop }: { poolId: string; member: string; watched: boolean; desktop: DesktopMemberInfo | undefined }): JSX.Element {
  const apps = useApps();
  const reach = reachOf(desktop);
  const app = desktop?.app_id ? apps?.find((one) => one.id === desktop.app_id) : undefined;
  const name = app?.name ?? "its app";
  const who = watched ? "It" : "Its sessions";
  const wake = desktop ? wakeLine(desktop.wake) : "";
  const words =
    reach.kind === "watched"
      ? `Watched: ${reach.line.charAt(0).toLowerCase()}${reach.line.slice(1)} (${CONNECTOR_MEANING}). ${who} ${watched ? "is" : "are"} shown and counted here, without the pool's tools.`
      : reach.kind === "added"
        ? `AI Juggler's connector is in ${name}. ${who} ${watched ? "joins" : "join"} at the next start; then: ${wake.charAt(0).toLowerCase()}${wake.slice(1)}.`
        : `${who} ${watched ? "talks" : "talk"} in this pool through AI Juggler's connector. ${wake}.`;
  return (
    <div className="flex flex-col gap-2" data-testid="inspect-rule-desktop" data-reach={reach.kind} data-member={member}>
      <span className="text-meta text-ink-2" data-testid="inspect-rule-reach">
        {words}
      </span>
      {desktop?.app_id && <ConnectorCard appId={desktop.app_id} app={name} kind={app?.app} profile={app?.profile} poolId={poolId} heading={4} testId="inspect-connector" />}
    </div>
  );
}

/** Round 2 (#2, #3): this App leads from its next session in the pool's
 * projects; "Start one now" opens New session on it there (a CLI App). */
function LeadsNext({ member, label, where, onStart, onChange }: { member: string; label: string } & RuleLead): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" data-testid="inspect-rule-leads-next">
      <span className="text-meta font-medium text-ink">Who leads</span>
      <span className="text-meta text-ink-2">{leadSentence(null, label, where)}</span>
      <span className="flex flex-wrap gap-2">
        {cliFolderOf(member) && <StartLeadButton member={member} name={label} where={where} onStart={onStart} testId="inspect-rule-start-lead" />}
        <Button size="sm" variant="ghost" onClick={onChange} data-testid="inspect-rule-change-lead">
          Change who leads
        </Button>
      </span>
    </div>
  );
}

/** A role chosen in the picker: on an App, Lead is its next session's
 * (round 3, #1, through the pool's one lead setter); another role ends
 * that wait first. */
function roleChange(next: PoolRole, input: { lead: RuleLead | null; run: (what: string, call: () => Promise<unknown>) => Promise<void>; poolId: string; member: string; level: string | null }): void {
  const { lead, run, poolId, member, level } = input;
  // Round 4 (#5): the new role shows at once, pending, until the rules read back.
  const pending = { poolId, ref: member, role: next };
  if (next === "lead" && lead && member.startsWith("app:")) {
    void run("change who leads", () => withPendingRole(pending, () => lead.onLead({ kind: "next", app: member })));
  } else {
    void run("change its role", () =>
      withPendingRole(pending, async () => {
        if (lead?.leadsNext) await lead.onLead({ kind: "none" });
        return poolJoin(poolId, member, next, level);
      }),
    );
  }
}

/** Who leads from here (an App whose next session leads: its line,
 * Start one now, Change), then its role, Lead included on an App, and why
 * a wider member can't lead, beside it. */
function RoleSection({ member, label, role, lead, busy, onRole, poolId }: { member: string; label: string; role: PoolRole; lead: RuleLead | null; busy: boolean; onRole: (role: PoolRole) => void; poolId: string }): JSX.Element {
  const pending = usePendingRoles(poolId).get(member);
  const leadsNext = pending ? pending === "lead" && member.startsWith("app:") : lead?.leadsNext === true;
  return (
    <>
      {lead && leadsNext && <LeadsNext member={member} label={label} {...lead} />}
      <div className="flex flex-col gap-1.5">
        <span className="text-meta font-medium text-ink">Its role</span>
        <RolePicker prefix="" member={member} name={label} value={pending && pending !== "lead" ? asRole(pending) : role} leadsNext={leadsNext} onChange={onRole} disabled={busy} testId="inspect-rule-role" />
        {pending && (
          <output className="text-label text-muted" data-testid="inspect-rule-role-saving">
            Saving…
          </output>
        )}
        <RoleWhy member={member} testId="inspect-rule-lead-hint" />
      </div>
    </>
  );
}
