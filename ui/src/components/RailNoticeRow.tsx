/**
 * D-49 — a Needs you line in the rail that isn't a session's (a sign-in,
 * This Mac, a cap, an agent's message or task, an account almost out of
 * room): so the rail's Needs you holds every item the bell counts. Its
 * mark and words are the notification centre's; a click is the notice's
 * own primary (it opens what it is about; nothing runs by itself).
 */
import { useLayoutEffect, useRef, useState, type JSX, type RefObject } from "react";
import { NoticeMark } from "./NoticeRow";
import { needLine, primaryWords } from "../lib/notice-words";
import type { NoticeInfo } from "../lib/ipc/notices";

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

export function RailNoticeRow({ notice, now, onOpen, canHandOver }: { notice: NoticeInfo; now: number; onOpen: (notice: NoticeInfo) => void; canHandOver: boolean }): JSX.Element {
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
  const showAction = Boolean(action) && !(long && plainOpen && line);
  return (
    <button
      type="button"
      data-rail-item={`notice:${notice.subject}`}
      data-testid="rail-notice"
      data-kind={notice.kind}
      onClick={() => onOpen(notice)}
      title={line ? `${notice.title} — ${line}` : undefined}
      className="flex h-[62px] w-full min-w-0 items-start gap-2.5 rounded-lg pb-2.5 pl-2 pr-2 pt-2 text-left outline-none transition-colors hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <span className="sr-only">{[notice.title, line, action].filter(Boolean).join(", ")}</span>
      <span aria-hidden="true" className="mt-0.5 flex shrink-0">
        <NoticeMark kind={notice.kind} small />
      </span>
      <span aria-hidden="true" className="flex min-w-0 flex-1 flex-col">
        <span ref={titleRef} className="line-clamp-2 text-meta font-semibold leading-[18px] text-ink">{notice.title}</span>
        {showLine && (
          <span className="truncate text-label leading-4 text-muted" data-testid="rail-notice-line">
            {line}
          </span>
        )}
        {showAction && <span className="truncate text-label font-semibold leading-4 text-warning">{action}</span>}
      </span>
    </button>
  );
}
