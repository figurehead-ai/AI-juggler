/**
 * E19 (AJ-E1903) — the plan behind a fold: each epic, its stories, and
 * their tasks, read a page at a time when a fold opens (never the whole
 * tree at once).
 * The same rows as the list, indented; a fold says how many are done.
 * AJ-E2806 R3 (amended): an outline list, each item with its level and
 * each fold with aria-expanded, moved as a treegrid is: ↑ ↓ move, →
 * unfolds or goes in, ← folds or goes to its parent (native lists, not
 * ARIA rows: the app's lint keeps new code on native elements).
 */
import { useState, type JSX, type KeyboardEvent } from "react";
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
      <li aria-level={depth + 1} data-plan-row data-folded={shown ? "false" : "true"} className="flex">
        <div className="flex min-w-0 flex-1 items-start">
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
        <div className="min-w-0 flex-1">
          <TaskRow bare task={item} now={now} selected={open === item.id} onOpen={() => onOpen(item.id)} onKeyDown={moveInPlan} />
        </div>
        </div>
      </li>
      {shown && (
        <>
          {stories.map((story) => (
            <Fold key={story.id} item={story} depth={depth + 1} plan={plan} poolId={poolId} open={open} now={now} onOpen={onOpen} />
          ))}
          {tasks.map((task) => (
            <li key={task.id} aria-level={depth + 2} data-plan-row style={{ marginLeft: `${(depth + 1) * 18 + 24}px` }}>
              <TaskRow bare task={task} now={now} selected={open === task.id} onOpen={() => onOpen(task.id)} onKeyDown={moveInPlan} />
            </li>
          ))}
          {under.hasMore && (
            <li aria-level={depth + 2} data-plan-row style={{ marginLeft: `${(depth + 1) * 18 + 24}px` }}>
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

/** The plan from the keyboard (WAI-ARIA treegrid): ↑ ↓ to the next row,
 * → unfolds a folded row or goes to its first item, ← folds an open one
 * or goes to its parent. */
function moveInPlan(event: KeyboardEvent<HTMLElement>): void {
  if (!["ArrowDown", "ArrowUp", "ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) return;
  const list = event.currentTarget.closest<HTMLElement>("[data-testid='task-tree']");
  if (!list) return;
  const rows = [...list.querySelectorAll<HTMLElement>(":scope > [data-plan-row]")];
  const row = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>("[data-plan-row]") : null;
  const at = row ? rows.indexOf(row) : -1;
  if (at < 0 || !row) return;
  const level = Number(row.getAttribute("aria-level") ?? 1);
  const focusRow = (target: HTMLElement | undefined) => target?.querySelector<HTMLElement>("[data-testid='task-row'], [data-testid='tree-more']")?.focus();
  const fold = () => row.querySelector<HTMLButtonElement>("[data-testid='tree-fold']")?.click();
  event.preventDefault();
  if (event.key === "ArrowDown") focusRow(rows[at + 1]);
  else if (event.key === "ArrowUp") focusRow(rows[at - 1]);
  else if (event.key === "Home") focusRow(rows[0]);
  else if (event.key === "End") focusRow(rows[rows.length - 1]);
  else if (event.key === "ArrowRight") {
    if (row.dataset.folded === "true") fold();
    else if (row.dataset.folded === "false") focusRow(rows[at + 1]);
  } else if (row.dataset.folded === "false") fold();
  else focusRow(rows.slice(0, at).reverse().find((up) => Number(up.getAttribute("aria-level") ?? 1) < level));
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
    <ul aria-label="The plan" className="flex flex-col gap-0.5" data-testid="task-tree">
      {roots.map((item) => (
        <Fold key={item.id} item={item} depth={0} plan={plan} poolId={poolId} open={open} now={now} onOpen={onOpen} />
      ))}
    </ul>
  );
}
