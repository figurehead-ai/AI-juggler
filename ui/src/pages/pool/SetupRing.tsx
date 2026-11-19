/**
 * 0.4.6 (D-126 §3): the setup's progress as a ring beside the checklist's
 * title, "5/6" in it: the accent for what's done on a quiet track. One
 * image to a screen reader, named in words ("5 of 6 done"); "Can't work
 * yet" while something keeps the pool from working, its arc then as far
 * as it got, never full.
 */
import type { JSX } from "react";

const R = 16;
const ROUND = 2 * Math.PI * R;

/** `decorative`: its words are said beside it (the checklist's count). */
export function SetupRing({ done, total, blocked = false, size = 40, decorative = false, testId }: { done: number; total: number; blocked?: boolean; size?: 40 | 32; decorative?: boolean; testId?: string }): JSX.Element {
  const share = total > 0 ? Math.min(1, done / total) : 0;
  // Something can't work: never a full ring.
  const arc = blocked ? Math.min(share, (total - 1) / Math.max(1, total)) : share;
  return (
    <svg role={decorative ? undefined : "img"} aria-hidden={decorative || undefined} aria-label={decorative ? undefined : blocked ? "Can't work yet" : `${done} of ${total} done`} width={size} height={size} viewBox="0 0 40 40" className="shrink-0" data-testid={testId} data-done={done} data-total={total}>
      <circle cx="20" cy="20" r={R} fill="none" strokeWidth="4" className="stroke-surface-3" />
      {arc > 0 && (
        <circle cx="20" cy="20" r={R} fill="none" strokeWidth="4" strokeLinecap="round" strokeDasharray={`${arc * ROUND} ${ROUND}`} transform="rotate(-90 20 20)" className={blocked ? "stroke-warning" : "stroke-accent"} />
      )}
      <text x="20" y="24.5" textAnchor="middle" fontSize="12" fontWeight="600" className="fill-ink" aria-hidden="true">
        {done}/{total}
      </text>
    </svg>
  );
}
