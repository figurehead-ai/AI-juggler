/**
 * 0.4.6 (AJ-E3011 §4): the four roles, each with its icon, one line on
 * what it may do and who holds it now; and the role picker a member gets,
 * with the same lines. The setup's "Everyone else" step and the diagram's
 * inspector show the same list. Round 2 (#9): a plain reference ("What
 * each role does"), text with an icon: no surface, edge or hover, since
 * nothing is chosen from it (the cards read as buttons).
 */
import type { JSX, SVGProps } from "react";

import { Picker } from "../../../components/controls";
import { IconLead, IconTasks } from "../../../components/icons-canvas";
import { IconDone } from "../../../components/icons-menu";
import { IconView } from "../../../components/icons-places";
import type { PoolRole } from "../../../lib/ipc/pools";
import { ROLE_CARDS, asRole, roleCard } from "../../../lib/pool-words";
import { rolesFor } from "../../../lib/pools";

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

const ICONS: Record<PoolRole, Icon> = { lead: IconLead, member: IconTasks, reviewer: IconDone, observer: IconView };

export function RoleIcon({ role, size = 14 }: { role: string; size?: number }): JSX.Element {
  const Shape = ICONS[asRole(role)];
  return <Shape aria-hidden="true" width={size} height={size} className="shrink-0 text-muted" />;
}

/** The four roles as a reference; `holders` names who holds each now. */
export function RoleCards({ holders, testId = "role-cards" }: { holders: Partial<Record<PoolRole, string[]>>; testId?: string }): JSX.Element {
  return (
    <ul className="grid grid-cols-1 gap-x-6 gap-y-2.5 sm:grid-cols-2" aria-label="What each role does" data-testid={testId}>
      {ROLE_CARDS.map((card) => {
        const who = holders[card.id] ?? [];
        return (
          <li key={card.id} className="flex gap-2" data-testid="role-card" data-role={card.id}>
            <span className="pt-0.5">
              <RoleIcon role={card.id} />
            </span>
            <span className="flex min-w-0 flex-col gap-0.5">
              <span className="text-meta font-semibold text-ink">{card.label}</span>
              <span className="text-meta text-ink-2">{card.line}</span>
              <span className="text-label text-muted" data-testid="role-card-holders">
                {who.length === 0 ? "No one yet" : `Now: ${who.join(", ")}`}
              </span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

/** A member's role, with each role's line under it. A broad member (an
 * App, Account, Tool, Mac or everyone) can't lead (D-92): its picker
 * says why. */
export function RolePicker({
  member,
  name,
  value,
  onChange,
  disabled = false,
  prefix = "Role:",
  testId,
}: {
  /** Its visible label, before the value (round 2, #10). */
  prefix?: string;
  /** Its kind (`app`) or stored ref (`app:…`). */
  member: string;
  name: string;
  value: PoolRole;
  onChange: (role: PoolRole) => void;
  disabled?: boolean;
  testId?: string;
}): JSX.Element {
  const may = new Set(rolesFor(member));
  return (
    <Picker
      choice
      size="sm"
      // The visible label starts its name (axe: label-content-name-mismatch).
      ariaLabel={prefix && prefix !== "Role:" ? `${name}: ${prefix.replace(/:$/, "")}` : `${name}'s role`}
      heading={`${name}'s role`}
      prefix={prefix}
      value={value}
      disabled={disabled}
      options={ROLE_CARDS.map((card) => ({
        value: card.id,
        label: card.label,
        hint: card.line,
        disabled: !may.has(card.id),
        reason: may.has(card.id) ? undefined : "Lead is one session. Pick who leads in Who leads.",
      }))}
      onChange={onChange}
      testId={testId}
    />
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
