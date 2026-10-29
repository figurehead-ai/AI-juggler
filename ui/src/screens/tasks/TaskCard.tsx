/**
 * 0.3 — a task on the Tasks screen's board: where it sits (its epic ›
 * story), its title, and under it its project (a link to the project's
 * page) and who has it, or what it waits on. A card opens the task beside
 * the board, where its actions are (Approve, Send back…, Free it, Cancel
 * task, Start a session for it…: the pool's own task view, never a copy).
 */
import type { JSX } from "react";
import { PRIORITY_WORDS } from "../../pages/tasks/words";
import type { TaskBoardRow } from "../../lib/ipc/tasks-across";

/** Who has it, in a few words: its holder, "Yours", or no one yet. */
export function holderLine(row: TaskBoardRow): string {
  const task = row.task;
  if (task.holder) return task.holder === "You" ? "Yours" : task.holder;
  return task.status === "ready" ? "No one yet" : "No one";
}

export function TaskCard({ row, projectName, selected, onOpen, onOpenProject }: { row: TaskBoardRow; projectName: string | null; selected: boolean; onOpen: () => void; onOpenProject: (id: string) => void }): JSX.Element {
  const task = row.task;
  const urgent = task.priority === "urgent" || task.priority === "high";
  const review = task.status === "in_review";
  return (
    <li
      className={
        "relative flex min-w-0 flex-col gap-1 rounded-lg border px-3 py-2.5 transition-colors focus-within:ring-2 focus-within:ring-accent/60 " +
        (selected ? "border-accent/50 bg-surface-3" : "border-hairline bg-surface hover:bg-surface-2")
      }
      data-testid="task-card"
      data-status={task.status}
    >
      {row.path.length > 0 && <span className="truncate text-label text-muted">{row.path.join(" › ")}</span>}
      <button
        type="button"
        onClick={onOpen}
        aria-current={selected ? "true" : undefined}
        className="text-left text-body font-medium leading-snug text-ink outline-none after:absolute after:inset-0 after:content-['']"
        data-testid="task-card-open"
      >
        {task.title}
      </button>
      <span className="flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5 text-meta text-muted">
        {projectName && row.project_id ? (
          <button
            type="button"
            onClick={() => onOpenProject(row.project_id!)}
            className="relative z-10 truncate rounded text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-current focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="task-card-project"
          >
            {projectName}
          </button>
        ) : (
          <span className="truncate">{row.pool_name}</span>
        )}
        <span aria-hidden="true">·</span>
        {task.blocked_by.length > 0 ? <span className="truncate text-danger">waits on {task.blocked_by.join(", ")}</span> : <span className="truncate">{holderLine(row)}</span>}
        {urgent && <span className="font-semibold text-warning">{PRIORITY_WORDS[task.priority]}</span>}
      </span>
      {review && (
        <span className="text-meta font-medium text-accent-hi" data-testid="task-card-review">
          Waits on your review: open it to approve or send back
        </span>
      )}
    </li>
  );
}
