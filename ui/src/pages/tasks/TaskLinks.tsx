/**
 * E19 — a task's links (waits on, blocks, relates to, duplicates), each
 * with the other task's state; a click opens it. Read only (D-44): the
 * agents link their tasks. A task that waits on an open one is blocked,
 * whatever was set: the list says why. Nothing shows when it has none.
 */
import type { JSX } from "react";
import type { TaskDetail } from "../../lib/ipc/tasks";
import { TaskStatus } from "./TaskRow";
import { LINK_WORDS } from "./words";

export function TaskLinks({ detail, onOpenTask }: { detail: TaskDetail; onOpenTask: (id: string) => void }): JSX.Element | null {
  if (detail.links.length === 0) return null;
  return (
    <section className="flex flex-col gap-1.5" data-testid="task-links">
      <h4 className="text-label font-semibold uppercase tracking-wider text-muted">Links</h4>
      <ul className="flex flex-col gap-1">
        {detail.links.map((link) => (
          <li key={`${link.kind}:${link.task_id}`} className="flex flex-wrap items-center gap-2 text-meta" data-testid="task-link" data-kind={link.kind}>
            <span className="w-24 shrink-0 text-muted">{LINK_WORDS[link.kind] ?? link.kind}</span>
            <button type="button" className="min-w-0 truncate text-left text-ink hover:underline" onClick={() => onOpenTask(link.task_id)}>
              {link.title}
            </button>
            <TaskStatus status={link.status} />
          </li>
        ))}
      </ul>
    </section>
  );
}
