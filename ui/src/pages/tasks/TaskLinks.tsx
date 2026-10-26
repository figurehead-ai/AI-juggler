/**
 * E19 — a task's links (waits on, blocks, relates to, duplicates), each
 * with the other task's state, and adding or taking one back. A task that
 * waits on an open one is blocked, whatever was set: the list says why.
 */
import { useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { tasksLink, type TaskDetail } from "../../lib/ipc/tasks";
import { TaskStatus } from "./TaskRow";
import { LINK_WORDS } from "./words";

const KINDS = ["blocked_by", "blocks", "relates_to", "duplicates"] as const;

export function TaskLinks({
  detail,
  busy,
  act,
  onOpenTask,
}: {
  detail: TaskDetail;
  busy: boolean;
  act: (run: () => Promise<TaskDetail>) => void;
  onOpenTask: (id: string) => void;
}): JSX.Element {
  const [adding, setAdding] = useState(false);
  const [kind, setKind] = useState<string>("blocked_by");
  const [other, setOther] = useState("");
  const id = detail.task.id;
  return (
    <section className="flex flex-col gap-1.5" data-testid="task-links">
      <div className="flex items-center gap-2">
        <h4 className="text-label font-semibold uppercase tracking-wider text-muted">Links</h4>
        {!adding && (
          <button type="button" className="text-label text-muted underline decoration-hairline underline-offset-2 hover:text-ink" onClick={() => setAdding(true)}>
            Add a link
          </button>
        )}
      </div>
      {detail.links.length === 0 && !adding && <p className="text-meta text-muted">None. A task that waits on another starts when that one is done.</p>}
      <ul className="flex flex-col gap-1">
        {detail.links.map((link) => (
          <li key={`${link.kind}:${link.task_id}`} className="flex flex-wrap items-center gap-2 text-meta" data-testid="task-link" data-kind={link.kind}>
            <span className="w-24 shrink-0 text-muted">{LINK_WORDS[link.kind] ?? link.kind}</span>
            <button type="button" className="min-w-0 truncate text-left text-ink hover:underline" onClick={() => onOpenTask(link.task_id)}>
              <span className="font-mono text-muted">{link.task_id}</span> {link.title}
            </button>
            <TaskStatus status={link.status} />
            <button
              type="button"
              className="text-label text-muted hover:text-danger"
              disabled={busy}
              onClick={() => act(() => tasksLink(id, link.task_id, link.kind, false))}
            >
              Remove<span className="sr-only"> the link to {link.task_id}</span>
            </button>
          </li>
        ))}
      </ul>
      {adding && (
        <form
          className="flex flex-wrap items-center gap-2"
          aria-label="Add a link"
          onSubmit={(event) => {
            event.preventDefault();
            if (!other.trim()) return;
            act(() => tasksLink(id, other.trim(), kind, true));
            setOther("");
            setAdding(false);
          }}
        >
          <select aria-label="How they're linked" value={kind} onChange={(event) => setKind(event.target.value)} className="h-8 rounded-lg border border-hairline bg-surface px-2 text-meta text-ink outline-none focus:border-accent/60">
            {KINDS.map((word) => (
              <option key={word} value={word}>
                {LINK_WORDS[word]}
              </option>
            ))}
          </select>
          <input aria-label="The other task's id" placeholder="aj-7f3c" value={other} onChange={(event) => setOther(event.target.value)} className="h-8 w-32 rounded-lg border border-hairline bg-surface px-2.5 font-mono text-meta text-ink outline-none focus:border-accent/60" />
          <Button type="submit" size="sm" variant="secondary" disabled={busy || !other.trim()}>
            Link
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setAdding(false)}>
            Cancel
          </Button>
        </form>
      )}
    </section>
  );
}
