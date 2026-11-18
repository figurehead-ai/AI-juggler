/**
 * A pool in small, for its card on the Pools screen (D-126 §4d, the X2
 * walk: the thumbnail was a near-empty dotted box). Its team as a little
 * diagram: who leads on top (the crown, in the accent), the members
 * below as their tools' logos, each with how many of its sessions are
 * open now and its role's mark, past four a "+N". With no lead, the
 * members sit in a row; with no one, it says so. Drawn, not interactive:
 * the card around it opens the pool, and its words say who's in.
 */
import type { JSX } from "react";

import type { PoolRole } from "../../lib/ipc/pools";
import type { PoolRulesInfo } from "../../lib/ipc/pool-model";
import { asRole } from "../../lib/pool-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { logoOf, MemberLogo, type LogoSources, type LogoSpec } from "./MemberLogo";

export interface MiniMember {
  ref: string;
  logo: LogoSpec;
  role: PoolRole;
  /** Its sessions open now. */
  count: number;
}

/** Sessions open now as members: alike tools together, a lead apart. */
function bySession(open: readonly InstanceInfo[], roleOf: (sessionId: string) => PoolRole, sources: LogoSources): { lead: MiniMember | null; members: MiniMember[] } {
  let lead: MiniMember | null = null;
  const groups = new Map<string, MiniMember>();
  for (const session of open) {
    const member: MiniMember = { ref: `session:${session.id}`, logo: logoOf(`session:${session.id}`, sources), role: roleOf(session.id), count: 1 };
    if (member.role === "lead" && !lead) {
      lead = member;
      continue;
    }
    const key = `${member.logo.kind === "tool" ? `${member.logo.tool}|${member.logo.appId ?? ""}|${member.logo.look}` : "tile"}|${member.role}`;
    const was = groups.get(key);
    groups.set(key, was ? { ...was, count: was.count + 1 } : member);
  }
  return { lead, members: [...groups.values()] };
}

/** A pool's team in small. Its named members (Apps, sessions) from its
 * member rules once read, each with its open sessions counted; what its
 * broad members (everyone, an Account, a Tool, a Mac) bring in now as
 * the tools of those sessions, alike ones together (their own tile when
 * nothing is open); before the rules are read, its sessions open now. */
export function miniTeam(rules: PoolRulesInfo | null, open: readonly InstanceInfo[], roleOf: (sessionId: string) => PoolRole, sources: LogoSources): { lead: MiniMember | null; members: MiniMember[] } {
  if (!rules) return bySession(open, roleOf, sources);
  const named = rules.members.filter((rule) => rule.kind === "app" || rule.kind === "session");
  const broad = rules.members.filter((rule) => !named.includes(rule));
  const all = named.map((rule) => ({ ref: rule.member, logo: logoOf(rule.member, sources), role: asRole(rule.role), count: rule.sessions }));
  const leading = all.find((one) => one.role === "lead") ?? all.find((one) => rules.lead_next !== null && one.ref === `app:${rules.lead_next}`);
  const covered = new Set(named.flatMap((rule) => rule.session_ids));
  const rest = bySession(
    open.filter((session) => !covered.has(session.id)),
    (id) => (roleOf(id) === "lead" ? "lead" : asRole(broad[0]?.role ?? "member")),
    sources,
  );
  const wide = broad.length === 0 ? [] : rest.members.length > 0 ? rest.members : broad.map((rule) => ({ ref: rule.member, logo: logoOf(rule.member, sources), role: asRole(rule.role), count: rule.sessions }));
  const lead = leading ? { ...leading, role: "lead" as const } : broad.length > 0 ? rest.lead : null;
  return { lead, members: [...all.filter((one) => one !== leading), ...wide] };
}

/** Members drawn; past this, "+N". */
export const MINI_ROW = 4;

function Chip({ member }: { member: MiniMember }): JSX.Element {
  return (
    <span className="relative inline-flex" data-testid="pool-mini-member" data-ref={member.ref} data-count={member.count}>
      <MemberLogo spec={member.logo} size={24} badge={member.role === "reviewer" ? "reviewer" : undefined} />
      {member.count > 1 && (
        <span className="absolute -right-2 -top-1.5 min-w-4 rounded-full bg-surface-3 px-1 text-center text-label font-semibold leading-4 text-ink shadow-[0_0_0_2px_var(--color-canvas-hi)]">{member.count}</span>
      )}
    </span>
  );
}

export function PoolMini({ lead, members, known = true, height = 92 }: { lead: MiniMember | null; members: readonly MiniMember[]; /** Its members are read (an empty one then says so). */ known?: boolean; height?: number }): JSX.Element {
  const shown = members.length > MINI_ROW ? members.slice(0, MINI_ROW - 1) : members;
  const more = members.length - shown.length;
  const across = shown.length + (more > 0 ? 1 : 0);
  // The bottom row's places, as % of the width.
  const xs = Array.from({ length: across }, (_, at) => (100 * (at + 1)) / (across + 1));
  return (
    <div
      aria-hidden="true"
      className="relative w-full overflow-hidden rounded-lg bg-canvas-hi bg-[radial-gradient(var(--color-hairline)_1px,transparent_1.2px)] bg-[length:12px_12px] inset-ring-1 inset-ring-hairline/70"
      style={{ height }}
      data-testid="pool-mini"
      data-lead={lead ? lead.ref : undefined}
    >
      {lead && across > 0 && (
        <svg className="absolute inset-0 size-full" viewBox="0 0 100 100" preserveAspectRatio="none" fill="none">
          {xs.map((x) => (
            <path key={x} d={`M50 36 C50 52 ${x} 48 ${x} 62`} stroke="var(--color-faint)" strokeOpacity={0.6} strokeWidth={1.25} vectorEffect="non-scaling-stroke" />
          ))}
        </svg>
      )}
      {lead && (
        <span className="absolute left-1/2 top-2.5 -translate-x-1/2 rounded-[10px] p-0.5 ring-[1.5px] ring-accent">
          <MemberLogo spec={lead.logo} size={24} badge="lead" />
        </span>
      )}
      {across === 0 && !lead && known && <span className="absolute inset-0 flex items-center justify-center text-label text-muted">No one in it yet</span>}
      {shown.map((member, at) => (
        <span key={member.ref} className="absolute -translate-x-1/2 -translate-y-1/2" style={{ left: `${xs[at]}%`, top: lead ? "74%" : "50%" }}>
          <Chip member={member} />
        </span>
      ))}
      {more > 0 && (
        <span className="absolute -translate-x-1/2 -translate-y-1/2 rounded-full bg-surface-3 px-2 text-label font-semibold leading-5 text-ink-2" style={{ left: `${xs[across - 1]}%`, top: lead ? "74%" : "50%" }} data-testid="pool-mini-more">
          +{more}
        </span>
      )}
    </div>
  );
}
