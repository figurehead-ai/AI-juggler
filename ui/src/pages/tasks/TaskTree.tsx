/**
 * E19 (AJ-E1903) — the plan behind a fold: each epic, its stories, and
 * their tasks, read a page at a time when a fold opens (never the whole
 * tree at once).
 * The same rows as the list, indented; a fold says how many are done.
 */
import { useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { usePagedList } from "../../lib/paged";
import { tasksChildren, type TaskInfo } from "../../lib/ipc/tasks";
import { TaskRow } from "./TaskRow";

function Fold({ item, depth, plan, poolId, open, now, onOpen }: { item: TaskInfo; depth: number; plan: TaskInfo[]; poolId: string; open: string | null; now: number; onOpen: (id: string) => void }): JSX.Element {
  const [shown, setShown] = useState(depth === 0);
  const stories = plan.filter((p) => p.parent_id === item.id);
  // A fold's items are read while it's open (its stories come with the plan).
  const under = usePagedList(shown ? `tasks-children:${poolId}:${item.id}` : null, (page) => tasksChildren(poolId, item.id, page), { keyOf: (task) => task.id });
  const tasks = under.items.filter((t) => t.kind === "task");
  const toggle = () => setShown((was) => !was);
  return (
    <>
      <li className="flex items-start">
        <button
          type="button"
          aria-expanded={shown}
          aria-label={`${shown ? "Fold" : "Unfold"} ${item.title}`}
          onClick={toggle}
          style={{ marginLeft: `${depth * 18}px` }}
          className="mt-2.5 flex size-6 shrink-0 items-center justify-center rounded-md text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="tree-fold"
        >
          <svg aria-hidden="true" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
            <path d={shown ? "M6 9l6 6 6-6" : "M9 6l6 6-6 6"} />
          </svg>
        </button>
        <ul className="min-w-0 flex-1">
          <TaskRow task={item} now={now} selected={open === item.id} onOpen={() => onOpen(item.id)} />
        </ul>
      </li>
      {shown && (
        <>
          {stories.map((story) => (
            <Fold key={story.id} item={story} depth={depth + 1} plan={plan} poolId={poolId} open={open} now={now} onOpen={onOpen} />
          ))}
          {tasks.map((task) => (
            <li key={task.id} style={{ marginLeft: `${(depth + 1) * 18 + 24}px` }}>
              <ul>
                <TaskRow task={task} now={now} selected={open === task.id} onOpen={() => onOpen(task.id)} />
              </ul>
            </li>
          ))}
          {under.hasMore && (
            <li style={{ marginLeft: `${(depth + 1) * 18 + 24}px` }}>
              <Button size="sm" variant="ghost" onClick={under.loadMore} disabled={under.loadingMore} data-testid="tree-more">
                {under.loadingMore ? "Reading more…" : `Show more under ${item.title}`}
              </Button>
            </li>
          )}
        </>
      )}
    </>
  );
}

export function TaskTree({ plan, poolId, open, now, onOpen }: { plan: TaskInfo[]; poolId: string; open: string | null; now: number; onOpen: (id: string) => void }): JSX.Element {
  const roots = plan.filter((item) => item.kind === "epic" || !plan.some((p) => p.id === item.parent_id));
  if (roots.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-hairline px-3 py-4 text-meta text-ink-2" data-testid="tree-empty">
        No epics or stories yet. Add an epic to plan bigger work, then its stories, then their tasks.
      </p>
    );
  }
  return (
    <ul className="flex flex-col gap-0.5" data-testid="task-tree">
      {roots.map((item) => (
        <Fold key={item.id} item={item} depth={0} plan={plan} poolId={poolId} open={open} now={now} onOpen={onOpen} />
      ))}
    </ul>
  );
}
