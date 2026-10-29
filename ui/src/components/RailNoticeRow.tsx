/**
 * D-49 — a Needs you line in the rail that isn't a session's (a sign-in,
 * This Mac, a cap, an agent's message or task, an account almost out of
 * room): so the rail's Needs you holds every item the bell counts. Its
 * mark and words are the notification centre's; a click is the notice's
 * own primary (it opens what it is about; nothing runs by itself).
 */
import type { JSX } from "react";
import { NoticeMark } from "./NoticeRow";
import { needLine, primaryLabel } from "../lib/notice-words";
import type { NoticeInfo } from "../lib/ipc/notices";

export function RailNoticeRow({ notice, now, onOpen }: { notice: NoticeInfo; now: number; onOpen: (notice: NoticeInfo) => void }): JSX.Element {
  const line = needLine(notice, now);
  const action = primaryLabel(notice);
  return (
    <button
      type="button"
      data-rail-item={`notice:${notice.subject}`}
      data-testid="rail-notice"
      data-kind={notice.kind}
      onClick={() => onOpen(notice)}
      className="flex h-[62px] w-full min-w-0 items-start gap-2.5 rounded-lg pb-2.5 pl-2 pr-2 pt-2 text-left outline-none transition-colors hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      <span className="sr-only">{[notice.title, line, action].filter(Boolean).join(", ")}</span>
      <span aria-hidden="true" className="mt-0.5 flex shrink-0">
        <NoticeMark kind={notice.kind} small />
      </span>
      <span aria-hidden="true" className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-meta font-semibold leading-[18px] text-ink">{notice.title}</span>
        {line && <span className="truncate text-label leading-4 text-muted">{line}</span>}
        {action && <span className="truncate text-label font-semibold leading-4 text-warning">{action}</span>}
      </span>
    </button>
  );
}
