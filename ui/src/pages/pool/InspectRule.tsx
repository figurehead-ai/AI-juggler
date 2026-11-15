/**
 * D-62 (AJ-E3008): a member rule in the inspector: what it brings in
 * (its sessions now, and new ones), its role and what its sessions may do
 * on their own ("Same as the pool" by default), how they hear a message
 * (D-119), and taking it out. A Watched member says what it can't do and
 * offers Continue in AI Juggler. Its name, kind and role are the
 * inspector's title and subtitle (AJ-E3012).
 */
import { useState, type JSX } from "react";

import { Button } from "../../components/primitives";

import { useActionHost } from "../../components/ItemMenu";
import { ShellOnly } from "../../components/ShellOnly";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { poolJoin, poolLeave, type PoolRuleInfo, type WatchedMemberInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo, PoolRole } from "../../lib/ipc/pools";
import { CONTINUE_WATCHED, setPools } from "../../lib/pools";
import { asRole, leadSentence, wakeWords } from "../../lib/pool-words";
import { cliFolderOf } from "../../lib/start-lead";
import type { LeadChoice } from "../../lib/new-pool";
import { MemberOwnLevel } from "./setup/OwnLevelChoice";
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
  const host = useActionHost();
  const { all } = useConversations();
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
  const conversation = watched ? all.find((row) => row.account_id === watched.folder && row.id === watched.conversation_id) : undefined;
  const fixed = pool.kind === "default" && member === "all";
  return (
    <div className="flex flex-col gap-4 px-4" data-testid="inspect-rule">
      <div className="flex flex-col gap-1">
        {shared && (
          <span className="flex flex-wrap items-center gap-2">
            <ShellOnly tool={shared.tool} account={shared.account} />
          </span>
        )}
        <span className="text-meta text-ink-2">
          {watched
            ? "Runs in another app: shown here and counted, with no tools and no rules, until it's continued in AI Juggler."
            : `${rule?.sessions ? `${rule.sessions} open here now` : "None open here now"}; its new sessions in this pool's projects join by themselves, and its ended ones are in the List view.`}
        </span>
      </div>
      {conversation && (
        <div className="flex flex-col gap-1">
          <Button variant="primary" size="sm" className="self-start" onClick={() => void host.continueConversation?.(conversation)} data-testid="inspect-rule-continue">
            Continue in AI Juggler
          </Button>
          <span className="text-meta text-ink-2" data-testid="inspect-rule-continue-what">
            {CONTINUE_WATCHED}
          </span>
        </div>
      )}
      <RoleSection member={member} label={label} role={role} lead={lead} open={rule?.sessions} busy={busy} onRole={(next) => roleChange(next, { lead, run, poolId: pool.id, member, level })} />
      {!watched && (
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
        <Button
          variant="secondary"
          size="sm"
          className="self-start"
          disabled={busy}
          onClick={() =>
            void run("take it out", async () => {
              const info = await poolLeave(pool.id, member);
              onGone();
              return info;
            })
          }
          data-testid="inspect-rule-leave"
        >
          Take out of the pool
        </Button>
      )}
    </div>
  );
}

/** Round 2 (#2, #3): this App leads from its next session in the pool's
 * projects; "Start one now" opens New session on it there (a CLI App). */
function LeadsNext({ member, label, open, where, onStart, onChange }: { member: string; label: string; open: number | undefined } & RuleLead): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" data-testid="inspect-rule-leads-next">
      <span className="text-meta font-medium text-ink">Who leads</span>
      <span className="text-meta text-ink-2">{leadSentence(null, label, where, open)}</span>
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
  if (next === "lead" && lead && member.startsWith("app:")) {
    void run("change who leads", () => lead.onLead({ kind: "next", app: member }));
  } else {
    void run("change its role", async () => {
      if (lead?.leadsNext) await lead.onLead({ kind: "none" });
      return poolJoin(poolId, member, next, level);
    });
  }
}

/** Who leads from here (an App whose next session leads: its line,
 * Start one now, Change), then its role, Lead included on an App, and why
 * a wider member can't lead, beside it. */
function RoleSection({ member, label, role, lead, open, busy, onRole }: { member: string; label: string; role: PoolRole; lead: RuleLead | null; open: number | undefined; busy: boolean; onRole: (role: PoolRole) => void }): JSX.Element {
  const leadsNext = lead?.leadsNext === true;
  return (
    <>
      {lead && leadsNext && <LeadsNext member={member} label={label} open={open} {...lead} />}
      <div className="flex flex-col gap-1.5">
        <span className="text-meta font-medium text-ink">Its role</span>
        <RolePicker prefix="" member={member} name={label} value={role} leadsNext={leadsNext} onChange={onRole} disabled={busy} testId="inspect-rule-role" />
        <RoleWhy member={member} testId="inspect-rule-lead-hint" />
      </div>
    </>
  );
}
