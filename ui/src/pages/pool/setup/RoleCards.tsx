/**
 * 0.4.6 (AJ-E3011 §4): the four roles, each with its icon, one line on
 * what it may do and who holds it now; and the role picker a member gets,
 * with the same lines. The setup's "Everyone else" step and the diagram's
 * inspector show the same list. Round 2 (#9): a plain reference ("What
 * each role does"), text with an icon: no surface, edge or hover, since
 * nothing is chosen from it (the cards read as buttons).
 */
import type { JSX } from "react";

import { Picker } from "../../../components/controls";
import { ROLE_ICONS } from "../../../components/MemberLogo";
import type { PoolRole } from "../../../lib/ipc/pools";
import { ROLE_CARDS, asRole, roleCard } from "../../../lib/pool-words";
import { leadHint, rolesFor } from "../../../lib/pools";

/** A role's icon: New pool's (its role tiles, the mark on a member's
 * logo), one map for both paths (D-126). */
function RoleIcon({ role, size = 14 }: { role: string; size?: number }): JSX.Element {
  const Shape = ROLE_ICONS[asRole(role)];
  return <Shape aria-hidden="true" width={size} height={size} className="shrink-0 text-muted" />;
}

/** At most this many names on a role's line; then "and N more" (X2 §4b:
 * fifty names wrapped down a narrow column). */
const NAMES = 3;

/** "Now: A, B, C and 47 more". */
export function holdersLine(who: readonly string[]): string {
  if (who.length === 0) return "No one yet";
  const shown = who.slice(0, NAMES);
  return who.length > NAMES ? `Now: ${shown.join(", ")} and ${who.length - NAMES} more` : `Now: ${shown.join(", ")}`;
}

/** The four roles as a reference; `holders` names who holds each now.
 * `narrow`: one column (the inspector's panel, whatever the window). */
export function RoleCards({ holders, narrow = false, testId = "role-cards" }: { holders: Partial<Record<PoolRole, string[]>>; narrow?: boolean; testId?: string }): JSX.Element {
  return (
    <ul className={"grid min-w-0 grid-cols-1 gap-x-6 gap-y-2.5 " + (narrow ? "" : "sm:grid-cols-2")} aria-label="What each role does" data-testid={testId}>
      {ROLE_CARDS.map((card) => {
        const who = holders[card.id] ?? [];
        return (
          <li key={card.id} className="flex min-w-0 gap-2" data-testid="role-card" data-role={card.id}>
            <span className="pt-0.5">
              <RoleIcon role={card.id} />
            </span>
            <span className="flex min-w-0 flex-col gap-0.5">
              <span className="text-meta font-semibold text-ink">{card.label}</span>
              <span className="text-meta text-ink-2">{card.line}</span>
              <span className="text-label text-muted [overflow-wrap:anywhere]" data-testid="role-card-holders">
                {holdersLine(who)}
              </span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

/** Round 3 (#1): an App's Lead, which its next session takes. */
const NEXT_LEAD_OPTION = { label: "Lead, once one of its sessions starts", hint: "It takes charge as soon as one of its sessions starts in the pool's projects. Its other sessions keep their role." };

/** Whether a member is an App (its next session can lead). */
const isApp = (member: string) => member === "app" || member.startsWith("app:");

/** A member's role, with each role's line under it. D-92: Lead is one
 * session. An App's Lead is its next session's (round 3, #1: the walk
 * clicked a greyed Lead three times); an Account, Tool, Mac or everyone
 * can't lead, and {@link RoleWhy} says why beside it. */
export function RolePicker({
  member,
  name,
  value,
  onChange,
  disabled = false,
  prefix = "Role:",
  leadsNext = false,
  testId,
}: {
  /** Its visible label, before the value (round 2, #10). */
  prefix?: string;
  /** Its kind (`app`) or stored ref (`app:…`). */
  member: string;
  name: string;
  value: PoolRole;
  /** An App whose next session leads: its value reads Lead. */
  leadsNext?: boolean;
  /** "lead" on an App: its next session leads. */
  onChange: (role: PoolRole) => void;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  const may = new Set(rolesFor(member));
  const app = isApp(member);
  return (
    <Picker
      choice
      size="sm"
      // The visible label starts its name (axe: label-content-name-mismatch).
      ariaLabel={prefix && prefix !== "Role:" ? `${name}: ${prefix.replace(/:$/, "")}` : `${name}'s role`}
      heading={`${name}'s role`}
      prefix={prefix}
      value={leadsNext ? "lead" : value}
      disabled={disabled}
      options={ROLE_CARDS.map((card) =>
        card.id === "lead" && app
          ? { value: card.id, label: NEXT_LEAD_OPTION.label, hint: NEXT_LEAD_OPTION.hint }
          : { value: card.id, label: card.label, hint: card.line, disabled: !may.has(card.id), reason: may.has(card.id) ? undefined : (leadHint(member) ?? undefined) },
      )}
      onChange={onChange}
      testId={testId}
    />
  );
}

/** Why a wider member (an Account, Tool, Mac, everyone) can't lead, as
 * text beside its role (round 3, #1: not only inside the menu). */
export function RoleWhy({ member, testId }: { member: string; testId?: string }): JSX.Element | null {
  const why = isApp(member) ? null : leadHint(member);
  if (!why) return null;
  return (
    <span className="text-meta text-ink-2" data-testid={testId}>
      {why}
    </span>
  );
}

/** Who holds each role, from members' names and roles. */
export function holdersOf(members: readonly { name: string; role: string }[]): Partial<Record<PoolRole, string[]>> {
  const out: Partial<Record<PoolRole, string[]>> = {};
  for (const member of members) {
    const role = roleCard(member.role).id;
    (out[role] ??= []).push(member.name);
  }
  return out;
}
