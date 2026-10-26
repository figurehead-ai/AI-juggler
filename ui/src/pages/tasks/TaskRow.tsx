/**
 * E19 — one task as a row, in the same shape as a thread's row and the
 * rail's: what it is first, where it is and who has it under it, its
 * status as a word and a mark (never colour alone).
 */
import type { JSX } from "react";
import { Pill } from "../../components/Pill";
import { ago } from "../../lib/time-words";
import type { TaskInfo } from "../../lib/ipc/tasks";
import { KIND_WORDS, PRIORITY_WORDS, STATUS_TONE, STATUS_WORDS, itemsWords } from "./words";

export function TaskStatus({ status }: { status: string }): JSX.Element {
  return (
    <Pill tone={STATUS_TONE[status] ?? "neutral"} testId="task-status">
      {STATUS_WORDS[status] ?? status}
    </Pill>
  );
}

export function TaskRow({ task, selected, now, onOpen, indent = 0 }: { task: TaskInfo; selected: boolean; now: number; onOpen: () => void; indent?: number }): JSX.Element {
  const items = itemsWords(task.items, task.items_done);
  const urgent = task.priority === "urgent" || task.priority === "high";
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        aria-current={selected ? "true" : undefined}
        style={indent ? { paddingLeft: `${12 + indent * 18}px` } : undefined}
        className={
          "flex w-full min-w-0 flex-col gap-1 rounded-lg px-3 py-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface-3" : "hover:bg-surface-2")
        }
        data-testid="task-row"
        data-status={task.status}
        data-kind={task.kind}
      >
        <span className="flex min-w-0 items-baseline gap-2">
          {task.kind !== "task" && <span className="shrink-0 text-[12px] font-semibold uppercase tracking-wider text-muted">{KIND_WORDS[task.kind]}</span>}
          <span className={"min-w-0 flex-1 truncate text-[14px] text-ink " + (task.status === "done" || task.status === "cancelled" ? "text-muted line-through decoration-hairline" : "font-medium")}>{task.title}</span>
          <span className="shrink-0 text-[12px] text-muted">{ago(task.updated_at_ms, now)}</span>
        </span>
        <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <TaskStatus status={task.status} />
          <span className="font-mono text-[12px] text-muted">{task.id}</span>
          {task.holder ? (
            <span className="min-w-0 truncate text-[13px] text-muted">
              {task.holder === "You" ? "Yours" : task.holder}
            </span>
          ) : (
            task.status === "ready" && task.kind === "task" && <span className="text-[13px] text-muted">Anyone can take it</span>
          )}
          {task.blocked_by.length > 0 && <span className="truncate text-[13px] text-danger">Waits on {task.blocked_by.join(", ")}</span>}
          {items && <span className="text-[13px] text-muted">{items}</span>}
          {urgent && <span className="text-[12px] font-semibold text-warning">{PRIORITY_WORDS[task.priority]}</span>}
          {task.issue_label && <span className="truncate font-mono text-[12px] text-muted">{task.issue_label}</span>}
        </span>
      </button>
    </li>
  );
}
