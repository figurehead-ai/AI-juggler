/**
 * 0.5.0 (D-142, DESIGN-SYSTEM §4.6, §4.7): a switch target, drawn the
 * same wherever one is shown — "Continue in another app"'s ranked list
 * (SwitchTargetRows, ContinueOn) and the switch wizard's Target step. The
 * maintainer: "easy to read, clear icons, tags… not a monochrome hell".
 *
 * - `TargetLogo`: the App's own logo at 40 px (ToolLogo: a CLI with its
 *   terminal badge), its profile's badge beside it, never over it (ICN-6).
 * - `RoomTag`: the account's room as a tag in its status colour (plenty
 *   the quiet green, low amber, out red; credits, unmeasured and signed
 *   out neutral), words always with it (TAG-3).
 * - `SuggestedTag`, `BadgeTag`: one accent "Suggested" (RC-3), and how
 *   much carries as a neutral tag, never an amber sentence (COL-4).
 * - `TargetButton`: a pick in a list, a card-like row (logo, the App's
 *   name, its account, its tags), a chevron saying it opens the switch;
 *   the Suggested one wears the accent's edge.
 */
import { forwardRef, type ButtonHTMLAttributes, type JSX, type ReactNode } from "react";
import { cliAppKey } from "../../lib/app-name";
import type { RailPlace } from "../../lib/rail-items";
import type { TargetRoom } from "../../lib/room-targets";
import { IconChevronRight } from "../icons-controls";
import { Pill, type PillTone } from "../Pill";
import { ProfileBadge } from "../ProfileBadge";
import { ToolLogo } from "../ToolLogo";

/** The App's logo, 40 px, and its profile's badge beside it. */
export function TargetLogo({ tool, kind, place, folderId, label, size = 40 }: { tool: string | null; kind: "cli" | "desktop"; place?: RailPlace | null; folderId: string; label: string; size?: 24 | 32 | 40 }): JSX.Element {
  const key = place?.key ?? (tool ? cliAppKey(tool, folderId, label) : `cli:${folderId}`);
  return (
    <span className="flex shrink-0 items-end gap-1" data-testid="target-logo">
      {tool ? <ToolLogo tool={tool} kind={kind} appId={place?.appId ?? undefined} appPid={place?.appPid ?? null} size={size} /> : <span aria-hidden="true" className="rounded-xl bg-surface-3" style={{ width: size, height: size }} />}
      <ProfileBadge app={key} name={place?.profile ?? label} size={8} />
    </span>
  );
}

/** A room's tag: its tone and its short words. */
export function roomTag(room: TargetRoom | null): { tone: PillTone; words: string } {
  if (room === null) return { tone: "neutral", words: "Not measured" };
  switch (room.kind) {
    case "room":
      return { tone: "success", words: room.left === null ? "Has room" : `${room.left}% left` };
    case "low":
      return { tone: "warning", words: room.left === null ? "Low" : `${room.left}% left` };
    case "out":
      return { tone: "danger", words: "Out of room" };
    case "credits":
      return { tone: "neutral", words: "Using credits" };
    case "blocked":
      return { tone: "neutral", words: "Not signed in" };
    default:
      return { tone: "neutral", words: "Not measured" };
  }
}

export function RoomTag({ room, testId = "room-tag" }: { room: TargetRoom | null; testId?: string }): JSX.Element {
  const { tone, words } = roomTag(room);
  return (
    <span className="contents" data-room={room?.kind ?? "unknown"}>
      <Pill tone={tone} testId={testId}>
        {words}
      </Pill>
    </span>
  );
}

/** The one recommendation: the account with the most room. */
export function SuggestedTag(): JSX.Element {
  return (
    <Pill tone="accent" testId="suggested-mark">
      Suggested
    </Pill>
  );
}

/** How much carries ("Full story, fresh start · Beta"): a neutral tag,
 * the words saying beta; the accent only once it's validated. */
export function BadgeTag({ badge, validated }: { badge: string; validated: boolean }): JSX.Element {
  return (
    <span className="contents" data-testid="fidelity-badge" data-validated={validated ? "true" : "false"}>
      <Pill tone={validated ? "accent" : "neutral"}>{badge}</Pill>
    </span>
  );
}

/** A pick in a target list: a card-like row that opens the switch. */
export const TargetButton = forwardRef<
  HTMLButtonElement,
  {
    logo: ReactNode;
    name: ReactNode;
    /** Its account, under the name. */
    sub?: ReactNode;
    /** Its tags (room, Suggested, how much carries): two at most. */
    tags?: ReactNode;
    /** Lines under the tags: the room in words, what doesn't carry. */
    detail?: ReactNode;
    /** The Suggested one: the accent's edge. */
    suggested?: boolean;
  } & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children">
>(function TargetButton({ logo, name, sub, tags, detail, suggested = false, className = "", ...rest }, ref) {
  return (
    <button
      ref={ref}
      type="button"
      {...rest}
      className={
        "group/target relative flex w-full min-w-0 items-start gap-3.5 rounded-card px-4 py-3 text-left outline-none transition-[background-color,box-shadow] " +
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-60 " +
        (suggested ? "bg-surface inset-ring-[1.5px] inset-ring-accent/70 hover:bg-surface-2 " : "bg-surface inset-ring-1 inset-ring-hairline hover:bg-surface-2 hover:inset-ring-faint/60 ") +
        className
      }
    >
      {logo}
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="truncate text-body font-semibold text-ink">{name}</span>
        {sub && <span className="truncate text-meta text-ink-2">{sub}</span>}
        {tags && <span className="mt-1 flex min-w-0 flex-wrap items-center gap-1.5">{tags}</span>}
        {detail && <span className="mt-1 flex min-w-0 flex-col gap-0.5 text-meta text-ink-2">{detail}</span>}
      </span>
      <IconChevronRight aria-hidden="true" width={16} height={16} className="shrink-0 text-faint group-hover/target:text-ink-2 group-disabled/target:hidden" />
    </button>
  );
});
