/**
 * AJ-E2027 (ADR-0039 §6) — Reminders, beside the watches: the other half
 * of "what AI Juggler will do for me later". Each reminder's words, what
 * it is about, when it is due (or how it ended), and Change time, Pause
 * and Remove; Pause all in the head. Hidden until the user has set one
 * (D-127): Home's Remind me and Bot's confirmed "remind me at…" make
 * them. A reminder is told through the bell and a notification; quiet
 * hours don't hold it (the user picked the time).
 *
 * Built for other tools' schedules later: a row whose `source` isn't
 * AI Juggler's is shown read-only (ADR-0039 §6). One bounded read (the
 * newest 50), remembered between visits.
 */
import { useCallback, useEffect, useRef, useState, type JSX, type MouseEvent } from "react";
import { showToast } from "../Toast";
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "../RowMenu";
import { refreshPaged, usePagedList } from "../../lib/paged";
import { errorText } from "../../lib/bridge";
import { clock, minutes as span, whenWords } from "../../lib/time-words";
import { REMIND_CHOICES } from "../../lib/feed-watch";
import { onSchedulesChanged, scheduleRemind, schedulePause, scheduleRemove, schedulesPage, schedulesPauseAll, schedulesSummary, scheduleUpdate, type ScheduleInfo, type SchedulesSummary } from "../../lib/ipc/schedules";
import { WatchChip, StateCell } from "./WatchParts";
import { optionOf, targetKey, type StateTone, type TargetOption } from "./watches-view";
import { IconBin, IconClockFace, IconPause, IconPencil, IconPlay } from "./watch-icons";

const GHOST = "inline-flex h-7 shrink-0 items-center gap-1.5 whitespace-nowrap rounded-lg px-2 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60";
const ICON_BUTTON = "inline-flex size-7 items-center justify-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60";

const REMINDERS = {
  title: "Reminders",
  line: (active: number, paused: number) => [active > 0 ? `${active} set` : null, paused > 0 ? `${paused} paused` : null].filter(Boolean).join(" · ") || "All told",
  pauseAll: "Pause all",
  resumeAll: "Resume all",
  change: "Change time",
  pause: "Pause",
  resume: "Resume",
  remove: "Remove",
  about: (name: string) => `About ${name}`,
  in: (ms: number, now: number) => (ms - now < 60_000 ? "Due now" : `In ${span(now, ms)}`),
  once: "Once, then done",
  paused: "Paused",
  wasDue: "Its time passed; told when you resume",
  resumeTo: "Resume to be told",
  told: (ms: number) => `Told ${clock(ms)}`,
  missed: "Not told",
  doneLine: "Done",
  otherTool: (source: string) => `Set in ${source}; change it there`,
  removed: (title: string) => `Removed “${title}”.`,
  moved: (when: string) => `Reminder moved to ${when}.`,
  failed: (why: string) => `That didn't work: ${why}`,
} as const;

/** What a reminder is about, by name: as the window's lists name it
 * when it holds it, else the backend's name for its session (0.5.2 G42:
 * the one rule, as its post said it), else its kind's word. */
function aboutName(book: Map<string, TargetOption>, info: ScheduleInfo): string | null {
  if (!info.about) return null;
  const held = book.get(targetKey(info.about));
  return held?.name ?? info.about_name ?? optionOf(book, info.about).name;
}

/** The state column for a reminder. */
function stateOf(info: ScheduleInfo, now: number): { tone: StateTone; top: string; sub: string } {
  if (info.ended_ms !== null) {
    const told = info.last_outcome === "ok" && info.last_run_ms !== null;
    return { tone: told ? "told" : "waiting", top: told ? REMINDERS.told(info.last_run_ms!) : REMINDERS.missed, sub: REMINDERS.doneLine };
  }
  const at = info.next_due_ms ?? (info.trigger.kind === "at" ? info.trigger.at_ms : null);
  if (info.paused) return { tone: "paused", top: REMINDERS.paused, sub: at !== null && at <= now ? REMINDERS.wasDue : REMINDERS.resumeTo };
  return { tone: "waiting", top: at !== null ? REMINDERS.in(at, now) : "", sub: REMINDERS.once };
}

interface RowActions {
  move: (info: ScheduleInfo, minutes: number) => void;
  pause: (info: ScheduleInfo, paused: boolean) => void;
  remove: (info: ScheduleInfo) => void;
}

function TimeMenu({ at, info, onPick, onClose }: { at: MenuAt; info: ScheduleInfo; onPick: (minutes: number) => void; onClose: () => void }): JSX.Element {
  const [opened] = useState(() => Date.now());
  return (
    <RowMenu heading={REMINDERS.change} ariaLabel={`${REMINDERS.change}: ${info.title}`} at={at} onClose={onClose} testId="reminder-time-menu">
      {REMIND_CHOICES(opened).map((choice) => (
        <MenuItem
          key={choice.label}
          onClick={() => {
            onClose();
            onPick(choice.minutes);
          }}
          testId="reminder-time-choice"
        >
          <MenuLine icon={IconClockFace} label={choice.label} />
        </MenuItem>
      ))}
    </RowMenu>
  );
}

function ReminderRow({ info, book, now, actions, focused }: { info: ScheduleInfo; book: Map<string, TargetOption>; now: number; actions: RowActions; focused: boolean }): JSX.Element {
  const [menuAt, setMenuAt] = useState<MenuAt | null>(null);
  const rowRef = useRef<HTMLLIElement | null>(null);
  // 0.5.2 G23: opened on this reminder (Remind me's toast, Bot's card): it
  // comes into view with the keyboard on it, marked as the selected row.
  useEffect(() => {
    if (!focused) return;
    rowRef.current?.scrollIntoView?.({ block: "center" });
    rowRef.current?.focus({ preventScroll: true });
  }, [focused]);
  const ours = info.source === "ai_juggler";
  const ended = info.ended_ms !== null;
  const dim = info.paused || ended;
  const about = aboutName(book, info);
  const state = stateOf(info, now);
  const open = (event: MouseEvent<HTMLElement>) => setMenuAt(menuPoint(event));
  return (
    <li
      ref={rowRef}
      tabIndex={focused ? -1 : undefined}
      className={"relative grid grid-cols-[44px_minmax(0,1fr)_minmax(150px,200px)_236px] items-center gap-x-3.5 rounded-[10px] px-3 py-3 outline-none [&+&]:rounded-none [&+&]:border-t [&+&]:border-divider" + (menuAt || focused ? " bg-selected" : "")}
      data-testid="reminder-row"
      data-schedule-id={info.id}
      data-focused={focused ? "1" : undefined}
      data-state={ended ? "ended" : info.paused ? "paused" : "set"}
    >
      <span aria-hidden="true" className={"flex size-9 items-center justify-center rounded-[10px] " + (dim ? "bg-surface-3 text-faint" : "bg-accent-soft text-accent-hi")}>
        <IconClockFace width={18} height={18} />
      </span>
      <div className="min-w-0">
        <p className={"truncate text-body font-medium " + (dim ? "text-muted" : "text-ink")} data-testid="reminder-title">
          {info.title}
        </p>
        <p className="mt-1 flex flex-wrap items-center gap-x-2.5 gap-y-1.5 text-meta text-muted">
          {about && <span data-testid="reminder-about">{REMINDERS.about(about)}</span>}
          {!ours && <WatchChip>{REMINDERS.otherTool(info.source)}</WatchChip>}
          {ours && !ended && info.trigger.kind === "at" && (
            <WatchChip icon={IconClockFace} accent={!info.paused} testId="reminder-when">
              {whenWords(info.trigger.at_ms, now)}
            </WatchChip>
          )}
        </p>
      </div>
      <StateCell {...state} />
      <div className="flex items-center justify-end gap-0.5">
        {ours && !ended && (
          <>
            <button type="button" className={GHOST} onClick={open} aria-haspopup="menu" aria-expanded={menuAt !== null} aria-label={`${REMINDERS.change}: ${info.title}`} data-testid="reminder-change">
              <IconPencil width={14} height={14} aria-hidden="true" />
              {REMINDERS.change}
            </button>
            {info.paused ? (
              <button type="button" className={GHOST} onClick={() => actions.pause(info, false)} aria-label={`${REMINDERS.resume}: ${info.title}`} data-testid="reminder-resume">
                <IconPlay width={13} height={13} aria-hidden="true" />
                {REMINDERS.resume}
              </button>
            ) : (
              <button type="button" className={ICON_BUTTON} onClick={() => actions.pause(info, true)} aria-label={`${REMINDERS.pause}: ${info.title}`} title={REMINDERS.pause} data-testid="reminder-pause">
                <IconPause width={15} height={15} aria-hidden="true" />
              </button>
            )}
          </>
        )}
        {ours && (
          <button type="button" className={ICON_BUTTON} onClick={() => actions.remove(info)} aria-label={`${REMINDERS.remove}: ${info.title}`} title={REMINDERS.remove} data-testid="reminder-remove">
            <IconBin width={15} height={15} aria-hidden="true" />
          </button>
        )}
      </div>
      {menuAt && <TimeMenu at={menuAt} info={info} onClose={() => setMenuAt(null)} onPick={(minutes) => actions.move(info, minutes)} />}
    </li>
  );
}

function useSummary(): [SchedulesSummary | null, () => void] {
  const [summary, setSummary] = useState<SchedulesSummary | null>(null);
  const read = useCallback(() => {
    void schedulesSummary().then(setSummary, () => setSummary(null));
  }, []);
  useEffect(read, [read]);
  return [summary, read];
}

/** The section; nothing while there are no reminders. */
export function RemindersSection({ book, now, focus = null }: { book: Map<string, TargetOption>; now: number; focus?: string | null }): JSX.Element | null {
  const list = usePagedList<ScheduleInfo>("schedules:yours", (request) => schedulesPage("yours", request), { pageSize: 50, keyOf: (row) => row.id });
  const [summary, readSummary] = useSummary();
  const refresh = () => {
    refreshPaged("schedules:");
    list.refresh();
    readSummary();
  };
  // 0.5.2 G30: a reminder told (a run) reads the list again: "In 1 h"
  // becomes "Told 7:45 PM" without leaving the page.
  const { refresh: reread } = list;
  useEffect(() => onSchedulesChanged(() => {
    refreshPaged("schedules:");
    reread();
    readSummary();
  }), [reread, readSummary]);
  const failed = (error: unknown) => showToast({ message: REMINDERS.failed(errorText(error)), tone: "error" });
  const actions: RowActions = {
    move: (info, minutes) => {
      const at = Date.now() + minutes * 60_000;
      void scheduleUpdate(info.id, at, info.title).then(() => {
        refresh();
        showToast({ message: REMINDERS.moved(whenWords(at)), tone: "success" });
      }, failed);
    },
    pause: (info, paused) => void schedulePause(info.id, paused).then(refresh, failed),
    remove: (info) =>
      void scheduleRemove(info.id).then(() => {
        refresh();
        const at = info.next_due_ms;
        showToast({
          id: `reminder-removed-${info.id}`,
          message: REMINDERS.removed(info.title),
          tone: "success",
          ...(at !== null && info.ended_ms === null
            ? { undo: { onUndo: () => scheduleRemind({ at_ms: at, title: info.title, about: info.about ?? null, chat_turn: null }).then(refresh), undoing: "Bringing it back…", done: "Reminder back.", failed: (cause: string) => `It couldn't be brought back: ${cause}` } }
            : {}),
          testId: "reminder-undo",
        });
      }, failed),
  };
  if (list.items.length === 0) return null;
  const active = summary?.active ?? list.items.filter((s) => s.ended_ms === null && !s.paused).length;
  const paused = summary?.paused ?? list.items.filter((s) => s.ended_ms === null && s.paused).length;
  const allPaused = active === 0 && paused > 0;
  return (
    <section aria-labelledby="watches-reminders" className="mt-1 flex flex-col gap-3" data-testid="reminders-section">
      <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
        <h3 id="watches-reminders" className="text-section font-semibold text-ink">
          {REMINDERS.title}
        </h3>
        <span className="text-meta text-muted" data-testid="reminders-line">
          {REMINDERS.line(active, paused)}
        </span>
        {active + paused > 0 && (
          <button
            type="button"
            className={GHOST + " ml-auto"}
            onClick={() => void schedulesPauseAll(!allPaused).then(refresh, failed)}
            data-testid={allPaused ? "reminders-resume-all" : "reminders-pause-all"}
          >
            {allPaused ? <IconPlay width={13} height={13} aria-hidden="true" /> : <IconPause width={14} height={14} aria-hidden="true" />}
            {allPaused ? REMINDERS.resumeAll : REMINDERS.pauseAll}
          </button>
        )}
      </div>
      <div className="rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
        <ul aria-labelledby="watches-reminders" data-testid="reminders-list">
          {list.items.map((info) => (
            <ReminderRow key={info.id} info={info} book={book} now={now} actions={actions} focused={info.id === focus} />
          ))}
        </ul>
      </div>
    </section>
  );
}
