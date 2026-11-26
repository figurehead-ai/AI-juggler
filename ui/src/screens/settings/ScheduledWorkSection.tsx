/**
 * AJ-E2027 (ADR-0039 §6) — Settings › Advanced › Scheduled work: what AI
 * Juggler does on its own and when, read-only, in the same plain words as
 * the Reminders list: what, when next, when it last ran and how. Nothing
 * here runs while AI Juggler is closed; what fell due then runs once on
 * the next launch. The user's own reminders are on the Watches page (a
 * link), and change there.
 */
import { useEffect, useState, type JSX } from "react";
import { IconClockFace, IconMoon } from "../../components/watches/watch-icons";
import { schedulesPage, type ScheduleInfo } from "../../lib/ipc/schedules";
import { clock, whenWords } from "../../lib/time-words";
import { openWatches } from "../../lib/watches-open";
import { PaneSection, SettingRow } from "./parts";

const SCHEDULED = {
  title: "Scheduled work",
  footer: "What AI Juggler does on its own, and when. Nothing runs while it's closed; what fell due then runs once when it opens.",
  yours: "Your reminders are on the Watches page.",
  open: "Open reminders",
  none: "Nothing is scheduled.",
  idle: "When this computer is quiet, about hourly",
  every: (minutes: number) => (minutes % 60 === 0 ? (minutes === 60 ? "Every hour" : `Every ${minutes / 60} hours`) : `Every ${minutes} minutes`),
  next: (ms: number, now: number) => `next ${whenWords(ms, now)}`,
  last: (ms: number, outcome: string | null) => (outcome === "ok" || outcome === null ? `last ran ${clock(ms)}` : `last try ${clock(ms)} didn't finish`),
  paused: "Paused",
} as const;

/** A schedule's when, in words. */
export function whenOf(info: ScheduleInfo, now: number): string {
  const t = info.trigger;
  const how = t.kind === "when_idle" ? SCHEDULED.idle : t.kind === "every" ? SCHEDULED.every(Math.round(t.interval_ms / 60_000)) : null;
  const parts = [how, info.paused ? SCHEDULED.paused : info.next_due_ms !== null && t.kind !== "when_idle" ? SCHEDULED.next(info.next_due_ms, now) : null, info.last_run_ms !== null ? SCHEDULED.last(info.last_run_ms, info.last_outcome) : null];
  return parts.filter(Boolean).join(" · ");
}

export function ScheduledWorkSection(): JSX.Element {
  const [rows, setRows] = useState<ScheduleInfo[] | null>(null);
  useEffect(() => {
    let on = true;
    void schedulesPage("app", { cursor: null, limit: 50 }).then(
      (page) => on && setRows(page.items.filter((s) => s.ended_ms === null)),
      () => on && setRows([]),
    );
    return () => {
      on = false;
    };
  }, []);
  const [now] = useState(() => Date.now());
  return (
    <PaneSection
      id="scheduled"
      title={SCHEDULED.title}
      footer={
        <>
          {SCHEDULED.footer} {SCHEDULED.yours}{" "}
          <button type="button" className="font-medium text-accent-hi underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => openWatches({ kind: "list" })} data-testid="scheduled-open-reminders">
            {SCHEDULED.open}
          </button>
        </>
      }
    >
      {rows === null ? (
        <li className="h-[52px] animate-pulse rounded-[10px] bg-skeleton/60" aria-hidden="true" />
      ) : rows.length === 0 ? (
        <li className="px-3 py-3 text-meta text-muted">{SCHEDULED.none}</li>
      ) : (
        rows.map((info) => <SettingRow key={info.id} icon={info.trigger.kind === "when_idle" ? IconMoon : IconClockFace} on={!info.paused} title={info.title} description={whenOf(info, now)} testId="scheduled-row" />)
      )}
    </PaneSection>
  );
}
