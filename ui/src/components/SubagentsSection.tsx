/**
 * AJ-E1520: the subagents a session started, in the inspector — what each
 * was asked to do and where it stands, behind a disclosure (the row only
 * says how many still run, I-08). Hidden when the session started none.
 */
import { useEffect, useState, type JSX } from "react";
import { subtasksList, type SubtaskInfo, type SubtaskStateInfo, type SubtasksInfo } from "../lib/ipc/subtasks";

const STATE_WORDS: Record<SubtaskStateInfo, string> = {
  running: "running",
  done: "done",
  failed: "failed",
  unfinished: "didn't finish",
};

const STATE_TONE: Record<SubtaskStateInfo, string> = {
  running: "text-accent-hi",
  done: "text-muted",
  failed: "text-warning",
  unfinished: "text-muted",
};

/** `2 running · 4 in all`, or `4, all done`. */
export function subagentsSummary(items: SubtaskInfo[]): string {
  const running = items.filter((item) => item.state === "running").length;
  if (running === 0) return `${items.length}, none running`;
  return `${running} running · ${items.length} in all`;
}

export function SubagentsSection({ sessionId, running }: { sessionId: string; running: number | null }): JSX.Element | null {
  const [info, setInfo] = useState<SubtasksInfo | null>(null);
  // Read again whenever the row's count moves: a subagent started or ended.
  useEffect(() => {
    let current = true;
    subtasksList(sessionId)
      .then((next) => {
        if (current) setInfo(next);
      })
      .catch(() => {
        if (current) setInfo(null);
      });
    return () => {
      current = false;
    };
  }, [sessionId, running]);

  if (!info || info.items.length === 0) return null;
  return (
    <details className="group flex flex-col gap-2" data-testid="subagents-section">
      <summary className="cursor-pointer list-none text-[11px] uppercase tracking-wider text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
        <span aria-hidden="true" className="mr-1 inline-block transition-transform group-open:rotate-90">▸</span>
        Subagents <span className="normal-case tracking-normal text-faint">{subagentsSummary(info.items)}</span>
      </summary>
      <ul className="mt-1 flex flex-col gap-1 border-l border-hairline pl-2.5 text-[11px]" aria-label="Subagents">
        {info.items.map((item) => (
          <li key={item.id} className="flex items-baseline gap-2" data-testid="subagent-row">
            <span className="min-w-0 flex-1 truncate text-ink" title={item.label}>
              {item.label || "a subagent"}
            </span>
            {item.kind && <span className="shrink-0 text-faint">{item.kind}</span>}
            <span className={`shrink-0 ${STATE_TONE[item.state]}`} data-testid="subagent-state">
              {STATE_WORDS[item.state]}
            </span>
          </li>
        ))}
      </ul>
    </details>
  );
}
