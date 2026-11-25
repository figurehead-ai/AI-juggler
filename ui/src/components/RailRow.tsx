/**
 * D-25: the rail's session row (state mark, title, one line of state and
 * time, its App and account). Moved out of WorkflowRail.tsx (its size
 * budget); WorkflowRail re-exports it.
 */
import { MACHINE_WORD } from "../lib/machine-words";
import { ProjectMark } from "./ProjectMark";
import { AppIcon } from "./AppIcon";
import { appOfSession } from "../lib/app-of";
import { RailQuietNotes } from "./RailNotes";
import { parkedWords } from "../lib/parked";
import { providerDisplayName } from "../lib/providers";
import { PinMark } from "./RailProjectRows";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { shortHeavy, useHeavyNote } from "../lib/machine";
import { REFRESH_INTERVALS, staleCaption } from "../lib/density";
import { shortName } from "../lib/session-titles";
import { sessionAppKey, sessionItem, sessionState } from "../lib/rail-items";
import { sessionWhen } from "../lib/time-words";
import { StaleChip, attentionTooltip, isActiveStatus, lowLeftWords } from "./indicator";
import { StateMark, rowState } from "./row/SessionRow";
import { appTone } from "../lib/attention";
import type { Freshness } from "../lib/live-deltas";
import { ProfileBadge } from "./ProfileBadge";
import { SELECTED_FILL } from "./controls/selected";

/** 0.5.1 F55: a title's readable minimum is 5rem (about ten letters);
 * what shares its line gives way below it, by the words' own width: the
 * "app" chip (2.5rem) first, then compact's age (3.75rem). */
const CHIP_ROOM = { comfortable: "@max-[7.5rem]/row:hidden", compact: "@max-[11.5rem]/row:hidden" } as const;
const AGE_ROOM = "@max-[8.75rem]/row:hidden";

/**
 * D-25 — the rail row: a state shape, what it is (the branch, else the
 * folder), one line of state and time, and the Account's colour. Quota
 * shows only when it's low (25 % or less left). Everything else — model,
 * tokens, processes, resources — is one click away in the session view.
 * Comfortable is two lines; compact is one.
 */
export function RailRow({
  session,
  showProject = false,
  selected,
  onSelect,
  density,
  now,
  freshness = {},
  trailing = false,
  note = null,
}: {
  /** Room for its always-shown tools (⋯ ↗) at its end. */
  trailing?: boolean;
  /** Its snooze ("snoozed 12m", "muted"), said in its line. */
  note?: string | null;
  session: InstanceInfo;
  showProject?: boolean;
  selected: boolean;
  onSelect: (id: string) => void;
  density: "comfortable" | "compact";
  now: number;
  freshness?: Freshness;
}) {
  // AJ-E1502: the user's name for it shows over its own title; untitled,
  // its facts (D-96), by the rail's one rule.
  const title = shortName(session, now);
  const ended = ["finished", "stopped", "handed_over"].includes(session.status);
  const parked = parkedWords(session, now);
  // AJ-E2104 Req.3: the place rows' words (Asks you, Working, Idle,
  // Ended; why it needs you when it does), the glyph a detail beside them.
  const said = sessionState(session);
  const state = parked ? parked.chip : said.word;
  // D-117: the App lens's calm look, from its parts: its state's mark (a
  // level's dot, a ring for a guess, a calm dot, a review's ✓), its word
  // in ink-2 at L2/L3 and muted below (the loud copy is its Needs you
  // row), and its App's icon muted unless it needs you or is current.
  const item = sessionItem(session, () => null, false);
  const look = rowState(item);
  const lit = look.level >= 2 || selected;
  // AJ-E3603: its state already says "Limit reached"; "0% left" beside it said it twice.
  const low = item.need === "limit" ? null : lowLeftWords(session);
  const when = sessionWhen(session, now);
  // D-25: the row stays calm — these markers appear only when they are
  // the exception worth a glance (a sign-in to check, uncommitted work,
  // a value that stopped updating); the session view has the rest.
  // E14 (AJ-E1405): resources show only when the session is heavy.
  const heavyNote = useHeavyNote(session.id);
  const heavy = heavyNote ? shortHeavy(heavyNote) : null;
  const stale = staleOf(session, freshness, now);
  return (
    <button
      type="button"
      data-instance-id={session.id}
      aria-current={selected ? "true" : undefined}
      data-rail-selected={selected ? "true" : undefined}
      onClick={() => onSelect(session.id)}
      className={
        "tone-host relative flex w-full min-w-0 items-center gap-2.5 rounded-lg pl-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (trailing ? "pr-14 " : "pr-2 ") +
        (density === "compact" ? "py-1.5 " : "py-2 ") +
        // Ended rows are quieter by their title colour, never by opacity:
        // faded small text would fall below 4.5:1 (WCAG 1.4.3).
        (selected ? SELECTED_FILL : "hover:bg-surface/60")
      }
    >
      <span aria-hidden="true" className="flex w-4 shrink-0 justify-center">
        <StateMark item={item} level={look.level} dot={look.dot} />
      </span>
      {/* 0.5.1 F55: the row's words measure their own room (a container),
          so a title keeps a readable minimum: in a short row the "app" chip
          gives way first, then compact's age (the App's icon at the end
          still says which app). */}
      <span className="@container/row flex min-w-0 flex-1 flex-col">
        <TitleLine session={session} showProject={showProject} selected={selected} ended={ended} title={title} density={density} note={note} when={when} />
        {density === "comfortable" && <StateLine session={session} parked={parked} state={state} level={look.level} low={low} heavy={heavy} heavyNote={heavyNote} stale={stale} note={note} when={when} />}
        <span className="sr-only">
          {session.pinned ? ", pinned" : ""}, {state}{note ? `, ${note}` : ""}, {providerDisplayName(session.provider)} on {session.account_label}
          {low ? `, ${low}` : ""}
          {heavyNote ? `, working your ${MACHINE_WORD} hard: ${heavyNote}` : ""}
        </span>
      </span>
      {density === "compact" && low && <span className="shrink-0 text-label tabular-nums text-muted">{low}</span>}
      {density === "compact" && !low && heavy && (
        <span data-testid="rail-heavy" className="shrink-0 font-mono text-label text-muted">
          {heavy}
        </span>
      )}
      {/* AJ-E1532 with D-38: the tool (its own app's icon), and the account serving it now (its avatar
          follows the session when it switches accounts). */}
      <AppIcon {...appOfSession(session)} size={16} className={appTone(lit)} />
      <ProfileBadge app={sessionAppKey(session)} name={session.account_label} className={appTone(lit)} />
    </button>
  );
}

/** A comfortable row's second line: its state, then the exceptions worth
 * a glance, then its time. One line: what doesn't fit wraps onto a
 * second, hidden line, so the time shows whole or not at all (0.3.2:
 * never "· 7 …"). */
function StateLine({ session, parked, state, level, low, heavy, heavyNote, stale, note, when }: {
  session: InstanceInfo;
  parked: ReturnType<typeof parkedWords>;
  state: string;
  level: number;
  low: string | null;
  heavy: string | null;
  heavyNote: string | null;
  stale: string | null;
  note: string | null;
  when: string;
}) {
  return (
    <span className="flex h-[1.4em] min-w-0 flex-wrap items-center gap-x-1 overflow-hidden text-label text-muted">
      <span
        title={parked ? parked.long : attentionTooltip(session)}
        data-testid={parked ? "rail-parked" : "rail-state-word"}
        // F55: alone on its line it ends on "…", never mid-letter.
        className={"min-w-0 truncate " + (parked ? "text-ink" : level >= 2 ? "font-medium text-ink-2" : "text-muted")}
      >
        {state}
      </span>
      {/* AJ-E3603: low room and a heavy load are facts here, muted
          (their loud copy is the Needs you row, as in the App lens). */}
      {low && <span className="shrink-0 whitespace-nowrap">· {low}</span>}
      {heavy && (
        <span title={`Working your ${MACHINE_WORD} hard: ${heavyNote}`} data-testid="rail-heavy" className="shrink-0 whitespace-nowrap">
          · {heavy}
        </span>
      )}
      <RailQuietNotes session={session} />
      <StaleChip caption={stale} />
      {note && (
        <span data-testid="rail-snoozed" className="shrink-0 whitespace-nowrap">
          · {note}
        </span>
      )}
      {/* The time yields first: the notes above matter more. */}
      <span className="whitespace-nowrap">· {when}</span>
    </span>
  );
}

/** Its quota's or its resources' reading that stopped updating, while it runs. */
function staleOf(session: InstanceInfo, freshness: Freshness, now: number): string | null {
  const sampled = freshness[session.id];
  if (!sampled || !isActiveStatus(session.status)) return null;
  return (session.quota !== null ? staleCaption(sampled.quota, REFRESH_INTERVALS.quota, now) : null) ??
    (session.cpu_pct !== null ? staleCaption(sampled.resources, REFRESH_INTERVALS.resources, now) : null);
}

/** A row's first line: its project's mark when not grouped by project,
 * its title, its pin, the "app" chip, and compact's age (or snooze). */
function TitleLine({ session, showProject, selected, ended, title, density, note, when }: {
  session: InstanceInfo;
  showProject: boolean;
  selected: boolean;
  ended: boolean;
  title: string;
  density: "comfortable" | "compact";
  note: string | null;
  when: string;
}) {
  return (
    <span className="flex min-w-0 items-baseline gap-1.5">
      {showProject && session.project_id && <ProjectMark projectId={session.project_id} size={12} />}
      <span className={"min-w-0 truncate text-meta " + (selected ? "font-semibold text-ink" : "font-medium " + (ended ? "text-muted" : "text-ink-2"))}>{title}</span>
      {session.pinned && <PinMark />}
      {session.kind === "desktop" && (
        <span title="Runs in the Claude desktop app" data-testid="rail-app-chip" className={"shrink-0 rounded border border-hairline px-1 text-label font-medium text-muted " + (density === "compact" ? CHIP_ROOM.compact : CHIP_ROOM.comfortable)}>
          app
        </span>
      )}
      {density === "compact" && <span className={"ml-auto shrink-0 text-label tabular-nums text-muted " + AGE_ROOM} data-testid={note ? "rail-snoozed" : undefined}>{note ?? when}</span>}
    </span>
  );
}
