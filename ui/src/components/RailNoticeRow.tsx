/**
 * D-49 — a Needs you line in the rail that isn't a session's (a sign-in,
 * This Mac, a cap, an agent's message or task, an account almost out of
 * room): so the rail's Needs you holds every item the bell counts. Its
 * mark and words are the notification centre's; a click is the notice's
 * own primary (it opens what it is about; nothing runs by itself).
 */
import { levelTone, noticeLevel } from "../lib/attention";
import { useLayoutEffect, useRef, useState, type JSX, type RefObject } from "react";
import { NoticeMark } from "./NoticeRow";
import { needLine, primaryWords } from "../lib/notice-words";
import type { NoticeInfo } from "../lib/ipc/notices";

/** Kinds whose title is a session's own (its state said apart from it):
 * every other notice's title says the need itself ("builder-1 asks you",
 * "Work needs its sign-in"). */
const SESSION_TITLED = new Set(["asks", "stuck"]);

/** One title line's height (`leading-[18px]`); a taller title wrapped. */
const TITLE_LINE = 18;

/** Whether the title wraps onto a second line, measured as drawn (the
 * rail's width and the font decide it, not a count of characters: "Planner
 * has a message waiting" fits). */
function useWraps(): [RefObject<HTMLSpanElement | null>, boolean] {
  const ref = useRef<HTMLSpanElement | null>(null);
  const [wraps, setWraps] = useState(false);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setWraps(el.getBoundingClientRect().height > TITLE_LINE * 1.5);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  return [ref, wraps];
}

/** An agent's message, subject first (D-118's mock, round 3), as a
 * session's need row reads: "Export flow", then "builder-1 asks you" in
 * its level's colour, then "Release pool". */
function AboutRow({ notice, about, onOpen }: { notice: NoticeInfo; about: { subject: string; context: string }; onOpen: (notice: NoticeInfo) => void }): JSX.Element {
  const level = noticeLevel(notice);
  const tone = level >= 2 ? "font-semibold " + levelTone(level).text : "text-muted";
  return (
    <button
      type="button"
      data-rail-item={`notice:${notice.subject}`}
      data-testid="rail-notice"
      data-kind={notice.kind}
      data-level={level}
      data-about="subject"
      onClick={() => onOpen(notice)}
      title={`${about.subject} — ${notice.title} — ${about.context}`}
      className="flex h-[62px] w-full min-w-0 items-start gap-2.5 rounded-lg pb-2.5 pl-2.5 pr-2 pt-2 text-left outline-none transition-colors hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <span className="sr-only">{[notice.title, about.subject, about.context].join(", ")}</span>
      {/* The row's mark: a neutral 6 px dot where a session's row has its App's (an agent has no App). */}
      <span aria-hidden="true" className="mt-[6px] size-1.5 shrink-0 rounded-full bg-surface-3 ring-1 ring-hairline" />
      <span aria-hidden="true" className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-meta font-semibold leading-[18px] text-ink" data-testid="rail-notice-title">{about.subject}</span>
        <span className={"truncate text-label leading-4 " + tone} data-testid="rail-notice-action">{notice.title}</span>
        <span className="truncate text-label leading-4 text-muted" data-testid="rail-notice-line">{about.context}</span>
      </span>
    </button>
  );
}

export function RailNoticeRow({ notice, now, onOpen, canHandOver, about = null }: { notice: NoticeInfo; now: number; onOpen: (notice: NoticeInfo) => void; canHandOver: boolean; about?: { subject: string; context: string } | null }): JSX.Element {
  if (about) return <AboutRow notice={notice} about={about} onOpen={onOpen} />;
  return <TitledRow notice={notice} now={now} onOpen={onOpen} canHandOver={canHandOver} />;
}

function TitledRow({ notice, now, onOpen, canHandOver }: { notice: NoticeInfo; now: number; onOpen: (notice: NoticeInfo) => void; canHandOver: boolean }): JSX.Element {
  const line = needLine(notice, now);
  const action = primaryWords(notice, canHandOver);
  // A long title takes the detail's line (the row's height is fixed:
  // the rail's list is drawn by it); the detail stays in its name and
  // its tooltip (the 0.4 review: "Claude Max 20x is almost out…").
  const [titleRef, long] = useWraps();
  // Unless its action is only "Open" (the row itself opens it): then the
  // long title takes that line and the detail, which says why it waits
  // on you, stays (the rc19 walk: "Ask first in «Team»" was out of sight).
  const plainOpen = action === "Open";
  const showLine = Boolean(line) && (!long || plainOpen);
  // D-118's mock: a bare "Open" is the row itself (a click opens it), so
  // it isn't written under a title that says the need ("builder-1 asks
  // you"): that title is the row's mark instead. A session's title keeps
  // its word, its state said apart from it.
  const titleMarks = plainOpen && !SESSION_TITLED.has(notice.kind);
  const showAction = Boolean(action) && !titleMarks && !(long && plainOpen && line);
  // D-117: its one quiet mark in its level's colour (amber L2, red L3; no
  // tinted row in the rail, D-118's one quiet mark per need): the action
  // word, else the title; a note's (L1: low room nothing live is on,
  // leftovers) muted.
  const level = noticeLevel(notice);
  const tone = level >= 2 ? levelTone(level).text : "text-ink";
  return (
    <button
      type="button"
      data-rail-item={`notice:${notice.subject}`}
      data-testid="rail-notice"
      data-kind={notice.kind}
      data-level={level}
      onClick={() => onOpen(notice)}
      title={line ? `${notice.title} — ${line}` : undefined}
      className={"flex h-[62px] w-full min-w-0 items-start gap-2.5 rounded-lg pb-2.5 pl-2 pr-2 pt-2 text-left outline-none transition-colors hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"}
    >
      <span className="sr-only">{[notice.title, line, showAction ? action : null].filter(Boolean).join(", ")}</span>
      <span aria-hidden="true" className="mt-0.5 flex shrink-0">
        <NoticeMark kind={notice.kind} small />
      </span>
      <span aria-hidden="true" className="flex min-w-0 flex-1 flex-col">
        <span ref={titleRef} className={"line-clamp-2 text-meta font-semibold leading-[18px] " + (titleMarks ? tone : "text-ink")} data-testid="rail-notice-title">{notice.title}</span>
        {showLine && (
          <span className="truncate text-label leading-4 text-muted" data-testid="rail-notice-line">
            {line}
          </span>
        )}
        {showAction && <span className={"truncate text-label leading-4 " + (level >= 2 ? "font-semibold " + tone : "text-muted")} data-testid="rail-notice-action">{action}</span>}
      </span>
    </button>
  );
}
