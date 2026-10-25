/**
 * AJ-E1520: the subagents a session started, in the inspector — what each
 * was asked to do and where it stands, behind a disclosure (the row only
 * says how many still run, I-08). Hidden when the session started none.
 * Read again when the row's count moves, and every 5 s while the session
 * runs (one can end as another starts, leaving the count as it was).
 */
import { useEffect, useState, type JSX } from "react";
import { subtasksList, type SubtaskInfo, type SubtaskStateInfo, type SubtasksInfo } from "../lib/ipc/subtasks";

/** How often a live session's list is read again. */
const REFRESH_MS = 5_000;

const STATE_WORDS: Record<SubtaskStateInfo, string> = {
  running: "running",
  background: "in the background",
  done: "done",
  failed: "failed",
  unfinished: "didn't finish",
};

const STATE_TONE: Record<SubtaskStateInfo, string> = {
  running: "text-accent-hi",
  background: "text-muted",
  done: "text-muted",
  failed: "text-warning",
  unfinished: "text-muted",
};

/** `2 running · 4 in all`, or `4 · none running`. */
export function subagentsSummary(items: SubtaskInfo[]): string {
  const running = items.filter((item) => item.state === "running").length;
  if (running === 0) return `${items.length} · none running`;
  return `${running} running · ${items.length} in all`;
}

export function SubagentsSection({
  sessionId,
  running,
  live,
}: {
  sessionId: string;
  running: number | null;
  live: boolean;
}): JSX.Element | null {
  const [info, setInfo] = useState<SubtasksInfo | null>(null);
  // Read again whenever the row's count moves (a subagent started or
  // ended), and every few seconds while the session runs.
  useEffect(() => {
    let current = true;
    const load = () => {
      subtasksList(sessionId)
        .then((next) => {
          if (current) setInfo(next);
        })
        .catch(() => {
          if (current) setInfo(null);
        });
    };
    load();
    const timer = live ? window.setInterval(load, REFRESH_MS) : undefined;
    return () => {
      current = false;
      if (timer !== undefined) window.clearInterval(timer);
    };
  }, [sessionId, running, live]);

  if (!info || info.items.length === 0) return null;
  return (
    <details className="group flex flex-col gap-2" data-testid="subagents-section">
      <summary className="cursor-pointer list-none text-section font-semibold text-ink outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
        <span aria-hidden="true" className="mr-1 inline-block transition-transform group-open:rotate-90">▸</span>
        Subagents <span className="normal-case tracking-normal text-muted">{subagentsSummary(info.items)}</span>
      </summary>
      <ul className="mt-1 flex flex-col gap-1 border-l border-hairline pl-2.5 text-label" aria-label="Subagents">
        {info.items.map((item) => (
          <li key={item.id} className="flex items-baseline gap-2" data-testid="subagent-row">
            <span className="min-w-0 flex-1 truncate text-ink" title={item.label}>
              {item.label || "a subagent"}
            </span>
            {item.kind && <span className="shrink-0 text-muted">{item.kind}</span>}
            <span className={`shrink-0 ${STATE_TONE[item.state]}`} data-testid="subagent-state">
              {STATE_WORDS[item.state]}
            </span>
          </li>
        ))}
      </ul>
    </details>
  );
}
