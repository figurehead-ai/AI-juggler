/**
 * 0.4.6 (AJ-E3011 §4): the four roles as small cards, each with its icon,
 * one line on what it may do and who holds it now; and the role picker a
 * member gets, with the same lines. The setup's "Everyone else" step and
 * the canvas's inspector show the same cards.
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

/** The four cards; `holders` names who holds each role now. */
export function RoleCards({ holders, testId = "role-cards" }: { holders: Partial<Record<PoolRole, string[]>>; testId?: string }): JSX.Element {
  return (
    <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2" aria-label="The roles" data-testid={testId}>
      {ROLE_CARDS.map((card) => {
        const who = holders[card.id] ?? [];
        return (
          <li key={card.id} className="flex flex-col gap-1 rounded-xl bg-surface-2 px-3 py-2.5" data-testid="role-card" data-role={card.id}>
            <span className="flex items-center gap-1.5">
              <RoleIcon role={card.id} />
              <span className="text-body font-semibold text-ink">{card.label}</span>
            </span>
            <span className="text-meta text-ink-2">{card.line}</span>
            <span className="text-label text-muted" data-testid="role-card-holders">
              {who.length === 0 ? "No one yet" : `Now: ${who.join(", ")}`}
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
  testId,
}: {
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
      ariaLabel={`${name}'s role`}
      heading={`${name}'s role`}
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
