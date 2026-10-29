/**
 * 0.3 — the Tasks screen's two views. Board: Ready · In progress · In
 * review · Blocked, each column its own paged list with its count. Plan:
 * each pool's epics → stories → tasks, the pool's own tree (TaskTree).
 * Both read across pools (lib/ipc/tasks-across.ts), a page at a time.
 */
import { useMemo, type JSX, type KeyboardEvent } from "react";
import { Button } from "../../components/primitives";
import { usePagedList } from "../../lib/paged";
import { tasksAcross, type TaskBoardRow, type TasksFilterInput } from "../../lib/ipc/tasks-across";
import type { TaskCounts } from "../../lib/ipc/tasks";
import { TaskTree } from "../../pages/tasks/TaskTree";
import { STATUS_WORDS } from "../../pages/tasks/words";
import { TaskCard } from "./TaskCard";

export const BOARD_COLUMNS = ["ready", "in_progress", "in_review", "blocked"] as const;
/** Cards a column reads at a time. */
const COLUMN_PAGE = 20;
/** Epics and stories the plan reads at a time (the backend's ceiling). */
const PLAN_PAGE = 200;

/** What a column says when it's empty. */
const EMPTY: Record<string, string> = {
  ready: "Nothing ready to take.",
  in_progress: "No one is working a task.",
  in_review: "Nothing waits on your review.",
  blocked: "Nothing is blocked.",
};

export type OpenTask = { id: string; projectId: string | null };

type Shared = {
  filter: TasksFilterInput;
  filterKey: string;
  open: OpenTask | null;
  projectName: (id: string | null) => string | null;
  onOpen: (task: OpenTask) => void;
  onOpenProject: (id: string) => void;
};

function Skeleton({ rows }: { rows: number }): JSX.Element {
  return (
    <>
      {Array.from({ length: rows }, (_, n) => (
        <li key={n} aria-hidden="true" className="h-[4.5rem] rounded-lg bg-surface-3 motion-safe:animate-pulse" />
      ))}
    </>
  );
}

function Column({ status, count, ...shared }: Shared & { status: string; count: number | null }): JSX.Element {
  const list = usePagedList(`tasks-across:${status}:${shared.filterKey}`, (page) => tasksAcross({ ...shared.filter, status }, page), { pageSize: COLUMN_PAGE, keyOf: (row) => row.task.id });
  const id = `tasks-column-${status}`;
  return (
    <section aria-labelledby={id} className="flex min-w-0 flex-col gap-2" data-testid={`tasks-column-${status}`}>
      <h3 id={id} className="flex items-baseline gap-2 px-1 text-section font-semibold text-ink">
        {STATUS_WORDS[status]}
        <span className="text-meta font-normal tabular-nums text-muted">{count ?? ""}</span>
      </h3>
      <ul className="flex flex-col gap-2" aria-label={STATUS_WORDS[status]} aria-busy={list.loading}>
        {list.loading ? (
          <Skeleton rows={2} />
        ) : list.items.length === 0 ? (
          <li className="rounded-lg border border-dashed border-hairline px-3 py-3 text-meta text-muted">{EMPTY[status]}</li>
        ) : (
          list.items.map((row) => (
            <TaskCard
              key={row.task.id}
              row={row}
              projectName={shared.projectName(row.project_id)}
              selected={shared.open?.id === row.task.id}
              onOpen={() => shared.onOpen({ id: row.task.id, projectId: row.project_id })}
              onOpenProject={shared.onOpenProject}
            />
          ))
        )}
      </ul>
      {list.hasMore && (
        <Button size="sm" variant="ghost" onClick={list.loadMore} disabled={list.loadingMore} className="self-start">
          {list.loadingMore ? "Reading more…" : "Show more"}
        </Button>
      )}
    </section>
  );
}

/** The board from the keyboard: ↑ ↓ within a column, ← → to the next
 * column's card at the same height. */
function moveOnBoard(event: KeyboardEvent<HTMLDivElement>): void {
  const step = { ArrowDown: [0, 1], ArrowUp: [0, -1], ArrowRight: [1, 0], ArrowLeft: [-1, 0] }[event.key];
  const at = (event.target as HTMLElement).closest<HTMLElement>("[data-testid^='tasks-column-']");
  if (!step || !at || !(event.target as HTMLElement).matches("[data-testid='task-card-open']")) return;
  const columns = [...event.currentTarget.querySelectorAll<HTMLElement>("[data-testid^='tasks-column-']")];
  const cards = (column: HTMLElement | undefined) => [...(column?.querySelectorAll<HTMLElement>("[data-testid='task-card-open']") ?? [])];
  const col = columns.indexOf(at);
  const row = cards(at).indexOf(event.target as HTMLElement);
  const next = cards(columns[col + step[0]!])[Math.max(0, step[0] ? Math.min(row, cards(columns[col + step[0]!]).length - 1) : row + step[1]!)];
  if (!next) return;
  event.preventDefault();
  next.focus();
}

export function Board({ counts, narrow, ...shared }: Shared & { counts: TaskCounts | null; narrow: boolean }): JSX.Element {
  return (
    <div className="@container min-w-0" data-testid="tasks-board" onKeyDown={moveOnBoard}>
      <div className={"grid gap-4 " + (narrow ? "@min-[520px]:grid-cols-2" : "@min-[520px]:grid-cols-2 @min-[960px]:grid-cols-4")}>
        {BOARD_COLUMNS.map((status) => (
          <Column key={status} status={status} count={counts ? (counts as Record<string, number>)[status]! : null} {...shared} />
        ))}
      </div>
    </div>
  );
}

/** The plan: each pool's tree, the pools in the order their items come. */
export function Plan({ now, ...shared }: Shared & { now: number }): JSX.Element {
  const list = usePagedList(`tasks-across-plan:${shared.filterKey}`, (page) => tasksAcross({ ...shared.filter, kind: "plan", status: "open" }, page), { pageSize: PLAN_PAGE, keyOf: (row) => row.task.id });
  const pools = useMemo(() => {
    const byPool = new Map<string, TaskBoardRow[]>();
    for (const row of list.items) byPool.set(row.task.pool_id, [...(byPool.get(row.task.pool_id) ?? []), row]);
    return [...byPool.values()];
  }, [list.items]);
  if (list.loading) {
    return (
      <ul className="flex flex-col gap-2" aria-busy="true" aria-label="Loading the plan">
        <Skeleton rows={3} />
      </ul>
    );
  }
  if (pools.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-hairline px-4 py-6 text-center text-body text-muted" data-testid="tasks-plan-empty">
        No epics or stories yet. When a pool's lead agent plans bigger work, its epics, their stories and their tasks show here.
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-6" data-testid="tasks-plan">
      {pools.map((rows) => {
        const first = rows[0]!;
        const project = shared.projectName(first.project_id);
        return (
          <section key={first.task.pool_id} aria-label={first.pool_name} className="flex flex-col gap-2">
            <h3 className="flex flex-wrap items-baseline gap-x-2 px-1 text-section font-semibold text-ink">
              {first.pool_name}
              {project && first.project_id && !first.pool_name.startsWith(project) && (
                <button type="button" onClick={() => shared.onOpenProject(first.project_id!)} className="text-meta font-normal text-muted underline decoration-hairline underline-offset-2 outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60">
                  {project}
                </button>
              )}
            </h3>
            <TaskTree plan={rows.map((row) => row.task)} poolId={first.task.pool_id} open={shared.open?.id ?? null} now={now} onOpen={(id) => shared.onOpen({ id, projectId: first.project_id })} />
          </section>
        );
      })}
      {list.hasMore && (
        <Button size="sm" variant="ghost" onClick={list.loadMore} disabled={list.loadingMore} className="self-start">
          {list.loadingMore ? "Reading more…" : "Show more of the plan"}
        </Button>
      )}
    </div>
  );
}
