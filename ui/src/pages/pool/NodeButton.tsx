/**
 * 0.4.6 (the rc2 walk P5, AJ-E3304 L4): a session's node is a real
 * button. The walk's accessibility press on a session node answered
 * "nothing to press there": the node was a group around its name. Now
 * the name's button is the node — named by its title and when it started
 * (D-126 §4a), its role and state in its description — and it covers the
 * whole card, so a click, VoiceOver's press, the walk tool's AXPress and
 * Enter anywhere on it open it in the inspector. The card's own controls
 * (its ⋯, its App, an ask's answers) sit above it.
 *
 * Keyboard focus rings the whole card (the ink ring, outside the chosen
 * card's accent one).
 */
import { useId, type JSX, type KeyboardEvent, type ReactNode } from "react";

import { IconLead } from "../../components/icons-canvas";
import { roleLabel } from "../../lib/pools";
import { titleWords, type NodeTitle } from "./node-title";
import type { MemberNode } from "./useCanvas";

/** A member's title, its name where none was worked out (a node drawn on
 * its own, in a test or a preview). */
export const titleOf = (member: Pick<MemberNode, "title" | "name">): NodeTitle => member.title ?? { title: member.name, when: null };

/** The button's cover over its card, and the card's focus ring on it. */
const COVER = "after:absolute after:inset-0 focus-visible:after:outline-2 focus-visible:after:outline-offset-4 focus-visible:after:outline-ink-2";

export function NodeButton({
  member,
  title,
  selected,
  tabStop,
  description,
  onSelect,
  onKey,
  className = "",
  radius = "after:rounded-xl",
  children,
}: {
  member: MemberNode;
  title: NodeTitle;
  selected: boolean;
  tabStop: boolean;
  description: string;
  onSelect: (id: string, focusInspector: boolean) => void;
  onKey: (id: string, event: KeyboardEvent<HTMLButtonElement>) => void;
  className?: string;
  /** The card's corners, for its cover and focus ring. */
  radius?: string;
  /** What it shows, when not its title as it is (a clamped span). */
  children?: ReactNode;
}): JSX.Element {
  const role = member.role === "lead" ? "Lead" : roleLabel(member.role);
  const described = useId();
  return (
    <>
    <button
      type="button"
      tabIndex={tabStop ? 0 : -1}
      data-node-name={member.id}
      aria-label={titleWords(title)}
      aria-pressed={selected}
      aria-keyshortcuts="Enter Delete ArrowUp ArrowDown ArrowLeft ArrowRight"
      aria-describedby={described}
      onClick={(event) => onSelect(member.id, event.detail === 0)}
      onKeyDown={(event) => onKey(member.id, event)}
      className={`min-w-0 rounded text-left font-semibold text-ink outline-none [overflow-wrap:normal] ${COVER} ${radius} ${className}`}
      title={titleWords(title)}
    >
      {children ?? title.title}
    </button>
    {/* Its role and state, read after its name. */}
    <span id={described} hidden data-testid="node-description">
      {`${role}. ${description}`}
    </span>
    </>
  );
}

/** A node's role in words, the lead with its mark (the List view's words:
 * the rc19 walk read Member only in an expanded row). */
export function RoleWords({ role }: { role: string }): JSX.Element {
  if (role === "lead")
    return (
      <span className="inline-flex shrink-0 items-center gap-1 font-medium text-accent-hi" data-testid="node-lead">
        <IconLead width={12} height={12} aria-hidden="true" />
        Lead
      </span>
    );
  return (
    <span className="shrink-0" data-testid="node-role">
      {roleLabel(role)}
    </span>
  );
}
