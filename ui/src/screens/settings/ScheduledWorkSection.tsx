/**
 * AJ-E2027 (ADR-0039 §6) — Settings › Advanced › Scheduled work: what AI
 * Juggler does on its own and when, read-only, in the same plain words as
 * the Reminders list: what, when next, when it last ran and how. Nothing
 * here runs while AI Juggler is closed; what fell due then runs once on
 * the next launch. The user's own reminders are on the Watches page (a
 * link), and change there.
 */
import { useEffect, useState, type JSX } from "react";
import { LoadingState } from "../../components/LoadingState";
import { IconClockFace, IconMoon } from "../../components/watches/watch-icons";
import { schedulesPage, type ScheduleInfo } from "../../lib/ipc/schedules";
import { useRetriedRead } from "../../lib/loading";
import { clock, whenWords } from "../../lib/time-words";
import { openWatches } from "../../lib/watches-open";
import { PaneSection, SettingRow } from "./parts";

const SCHEDULED = {
  title: "Scheduled work",
  what: "the scheduled work",
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
  const { state, slow, setFailed, round, retry } = useRetriedRead(rows, SCHEDULED.what);
  useEffect(() => {
    let on = true;
    void schedulesPage("app", { cursor: null, limit: 50 }).then(
      // A watch's timer is the watch's own: it lives on the Watches page.
      (page) => {
        if (!on) return;
        setFailed(undefined);
        setRows(page.items.filter((s) => s.ended_ms === null && s.handler !== "watch.deadline"));
      },
      // A read that failed says so, with Retry: never "Nothing is scheduled".
      (cause: unknown) => on && setFailed(cause),
    );
    return () => {
      on = false;
    };
    // `round` is a Retry: it reads again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [round, setFailed]);
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
      {state === "ready" && rows ? (
        rows.map((info) => <SettingRow key={info.id} icon={info.trigger.kind === "when_idle" ? IconMoon : IconClockFace} on={!info.paused} title={info.title} description={whenOf(info, now)} testId="scheduled-row" />)
      ) : (
        // The section's rows sit in a list: its other states take one item.
        <li>
          <LoadingState
            state={state}
            slow={slow}
            what={SCHEDULED.what}
            skeleton={<div className="h-[52px] rounded-[10px] bg-skeleton/60 motion-safe:animate-pulse" aria-hidden="true" />}
            empty={<p className="px-3 py-3 text-meta text-muted">{SCHEDULED.none}</p>}
            onRetry={retry}
          >
            {null}
          </LoadingState>
        </li>
      )}
    </PaneSection>
  );
}
