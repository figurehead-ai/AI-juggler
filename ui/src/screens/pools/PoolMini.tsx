/**
 * A pool in small, for its card on the Pools screen (D-126 §4d, the X2
 * walk: the thumbnail was a near-empty dotted box). Its team as a little
 * diagram: who leads on top (the crown, in the accent), the members
 * below as their tools' logos, each with how many of its sessions are
 * open now and its role's mark, past four a "+N"; with two or fewer, as
 * cards with their names (a one-session pool isn't a lone logo in an
 * empty box). With no lead, the members sit in a row; with no one, it
 * says so. Drawn, not interactive:
 * the card around it opens the pool, and its words say who's in.
 */
import type { JSX } from "react";

import type { PoolRole } from "../../lib/ipc/pools";
import type { PoolRulesInfo } from "../../lib/ipc/pool-model";
import { asRole, roleCard } from "../../lib/pool-words";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { logoOf, MemberLogo, type LogoSources, type LogoSpec } from "../../components/MemberLogo";
import { PreviewCard } from "./NewPoolPreview";

export interface MiniMember {
  ref: string;
  logo: LogoSpec;
  role: PoolRole;
  /** Its sessions open now. */
  count: number;
  /** Its name: a session's title; alike sessions by their App's name. */
  name: string;
}

/** A member's name by its ref (a session's title, an App's name). */
export type MiniName = (ref: string) => string;

/** What a member of each kind is called when nothing names it. */
const KIND_WORDS: Record<string, string> = { app: "An App", account: "An Account", tool: "A tool", mac: "A computer" };

/** What a member is called when nothing names it: its tool's name, else
 * a word for its kind, never its id (F-rc3f-6: "sess-731993261005 ·
 * Lead" on a Pools card). */
const fallbackName = (ref: string, logo: LogoSpec): string => (ref === "all" ? "Everyone" : logo.kind === "tool" && logo.name ? logo.name : (KIND_WORDS[ref.slice(0, ref.indexOf(":"))] ?? "A session"));

/** Sessions open now as members: alike tools together (by their App's
 * name), a lead apart. */
function bySession(open: readonly InstanceInfo[], roleOf: (sessionId: string) => PoolRole, sources: LogoSources, nameOf: MiniName): { lead: MiniMember | null; members: MiniMember[] } {
  let lead: MiniMember | null = null;
  const groups = new Map<string, MiniMember>();
  for (const session of open) {
    const ref = `session:${session.id}`;
    const logo = logoOf(ref, sources);
    const member: MiniMember = { ref, logo, role: roleOf(session.id), count: 1, name: nameOf(ref) || fallbackName(ref, logo) };
    if (member.role === "lead" && !lead) {
      lead = member;
      continue;
    }
    const key = `${logo.kind === "tool" ? `${logo.tool}|${logo.appId ?? ""}|${logo.look}` : "tile"}|${member.role}`;
    const was = groups.get(key);
    // Alike sessions together go by their App's name.
    groups.set(key, was ? { ...was, count: was.count + 1, name: logo.kind === "tool" && logo.name ? logo.name : "Sessions" } : member);
  }
  return { lead, members: [...groups.values()] };
}

/** A pool's team in small, as its card counts it: its sessions open now
 * (alike tools together), and its member rules once read — an App or a
 * session each with its open sessions counted, a broad member (everyone,
 * an Account, a Tool, a Mac) by its own tile while none of its sessions
 * is open. Its lead on top: the session that leads, or the App whose
 * next session will. A pool whose members are sessions only (rules read,
 * none of them) draws those sessions: the 0.4.6 rc3 Pools render said
 * "No one in it yet" over "6 sessions · 4 need you". */
export function miniTeam(rules: PoolRulesInfo | null, open: readonly InstanceInfo[], roleOf: (sessionId: string) => PoolRole, sources: LogoSources, nameOf: MiniName = () => ""): { lead: MiniMember | null; members: MiniMember[] } {
  const named = (rules?.members ?? []).filter((rule) => rule.kind === "app" || rule.kind === "session");
  const broad = (rules?.members ?? []).filter((rule) => !named.includes(rule));
  const ruled = named.map((rule) => {
    const logo = logoOf(rule.member, sources);
    return { ref: rule.member, logo, role: asRole(rule.role), count: rule.sessions, name: nameOf(rule.member) || fallbackName(rule.member, logo) };
  });
  const leading = ruled.find((one) => one.role === "lead") ?? ruled.find((one) => rules?.lead_next != null && one.ref === `app:${rules.lead_next}`);
  const covered = new Set([...named.flatMap((rule) => rule.session_ids), ...named.map((rule) => rule.member.replace(/^session:/, ""))]);
  const rest = bySession(
    open.filter((session) => !covered.has(session.id)),
    roleOf,
    sources,
    nameOf,
  );
  const wide =
    rest.members.length > 0 || rest.lead
      ? rest.members
      : broad.map((rule) => {
          const logo = logoOf(rule.member, sources);
          return { ref: rule.member, logo, role: asRole(rule.role), count: rule.sessions, name: nameOf(rule.member) || fallbackName(rule.member, logo) };
        });
  const lead = leading ? { ...leading, role: "lead" as const } : rest.lead;
  return { lead, members: [...ruled.filter((one) => one !== leading), ...wide] };
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

/** At most this many members drawn as cards (a logo, its name and its
 * line); more, as logos with counts. */
export const MINI_CARDS = 2;

/** A member's card's second line: its role, or how many are open. */
const lineOf = (member: MiniMember): string => (member.role === "lead" ? roleCard("lead").label : member.count > 1 ? `${member.count} open · ${roleCard(member.role).label}` : roleCard(member.role).label);

export function PoolMini({ lead, members, known = true, height = 92 }: { lead: MiniMember | null; members: readonly MiniMember[]; /** Its members are read (an empty one then says so). */ known?: boolean; height?: number }): JSX.Element {
  const cards = members.length <= MINI_CARDS;
  const shown = members.length > MINI_ROW ? members.slice(0, MINI_ROW - 1) : members;
  const more = members.length - shown.length;
  const across = shown.length + (more > 0 ? 1 : 0);
  // The bottom row's places, as % of the width.
  const xs = cards ? ([[50], [50], [27, 73]][shown.length] ?? [50]) : Array.from({ length: across }, (_, at) => (100 * (at + 1)) / (across + 1));
  const card = (member: MiniMember, at: { left: string; top: string }, wide: number) => (
    <PreviewCard key={member.ref} node={{ ref: member.ref, name: member.name, line: lineOf(member), logo: member.logo, role: member.role }} at={at} across={wide} oneLine testId="pool-mini-card" />
  );
  return (
    <div
      aria-hidden="true"
      className="relative w-full overflow-hidden rounded-lg bg-canvas-hi bg-[radial-gradient(var(--color-hairline)_1px,transparent_1.2px)] bg-[length:12px_12px] inset-ring-1 inset-ring-hairline/70"
      style={{ height }}
      data-testid="pool-mini"
      data-lead={lead ? lead.ref : undefined}
      data-look={cards ? "cards" : "logos"}
    >
      {lead && across > 0 && (
        <svg className="absolute inset-0 size-full" viewBox="0 0 100 100" preserveAspectRatio="none" fill="none">
          {xs.map((x) => (
            <path key={x} d={cards ? `M50 46 C50 52 ${x} 50 ${x} 56` : `M50 36 C50 52 ${x} 48 ${x} 62`} stroke="var(--color-faint)" strokeOpacity={0.6} strokeWidth={1.25} vectorEffect="non-scaling-stroke" />
          ))}
        </svg>
      )}
      {lead &&
        (cards ? (
          card(lead, { left: "50%", top: across > 0 ? "6%" : "30%" }, 1)
        ) : (
          <span className="absolute left-1/2 top-2.5 -translate-x-1/2 rounded-[10px] p-0.5 ring-[1.5px] ring-accent">
            <MemberLogo spec={lead.logo} size={24} badge="lead" />
          </span>
        ))}
      {across === 0 && !lead && known && <span className="absolute inset-0 flex items-center justify-center text-label text-muted">No one in it yet</span>}
      {cards
        ? shown.map((member, at) => card(member, { left: `${xs[at]}%`, top: lead ? "56%" : "30%" }, shown.length))
        : shown.map((member, at) => (
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
