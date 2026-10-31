/**
 * E19 (AJ-E1903) — a pool's Tasks tab: its tasks on the left, ready work
 * first, read a page at a time (`usePagedList`, the backend's keyset
 * pages), with how many are in each state; the epic and story plan behind
 * its own fold; the open task on the right. Loaded only when the tab is
 * first shown.
 *
 * D-44: you see and track the agents' work here; the agents write the
 * tasks (no New task, no editing). A task offers only your four actions
 * (`TaskView`).
 *
 * Also a session's Tasks tab: what it holds and what it worked.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { refreshPaged, usePagedList } from "../../lib/paged";
import { usePoolPlace, usePools } from "../../lib/pools";
import { rememberTab } from "../registry";
import { useActionHost } from "../../components/ItemMenu";
import { focusTask, onTasksChanged, takeFocusedTask, tasksCounts, tasksList, tasksOfSession, tasksPlan, type TaskCounts, type TaskInfo } from "../../lib/ipc/tasks";
import { TaskRow } from "./TaskRow";
import { TaskTree } from "./TaskTree";
import { TaskView } from "./TaskView";
import { STATUS_WORDS } from "./words";

const FILTERS = ["open", "ready", "in_progress", "in_review", "blocked", "closed"] as const;
const FILTER_WORDS: Record<string, string> = { ...STATUS_WORDS, open: "Open", closed: "Done" };
/** Rows a page of the list holds. */
const LIST_PAGE = 40;
/** Epics and stories a page of the plan holds (the backend's ceiling). */
const PLAN_PAGE = 200;

function countOf(counts: TaskCounts | null, filter: string): number | null {
  if (!counts) return null;
  if (filter === "open") return counts.ready + counts.in_progress + counts.in_review + counts.blocked + counts.todo;
  if (filter === "closed") return counts.done + counts.cancelled;
  return (counts as Record<string, number>)[filter] ?? 0;
}

/** Every task list shown reads again when tasks change (the backend says
 * so once a burst). Their keys all start with `tasks`. */
function useTasksRefresh(): void {
  useEffect(() => onTasksChanged(() => refreshPaged("tasks")), []);
}

/** A list's skeleton while its first page is read. */
function Skeleton({ label, rows }: { label: string; rows: number }): JSX.Element {
  return (
    <div className="flex flex-col gap-2" role="status" aria-label={label}>
      {Array.from({ length: rows }, (_, n) => (
        <span key={n} className="h-16 rounded-lg bg-skeleton motion-safe:animate-pulse" />
      ))}
    </div>
  );
}

export function PoolTasks({ poolId, sessions, projects, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const pool = usePools()?.pools.find((p) => p.id === poolId);
  const defaultAccount = projects.find((p) => p.id === pool?.project_id)?.default_account_id ?? null;
  const [filter, setFilter] = useState<string>("open");
  const [view, setView] = useState<"list" | "plan">("list");
  const [words, setWords] = useState("");
  const [query, setQuery] = useState("");
  const [counts, setCounts] = useState<TaskCounts | null>(null);
  const [open, setOpen] = useState<string | null>(takeFocusedTask);
  useTasksRefresh();
  // What's typed is looked for once typing pauses.
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(words.trim()), words ? 200 : 0);
    return () => window.clearTimeout(timer);
  }, [words]);
  useEffect(() => {
    let alive = true;
    const read = () => void tasksCounts(poolId).then((next) => alive && setCounts(next), () => undefined);
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [poolId]);
  const list = usePagedList(`tasks:${poolId}:${filter}:${query}`, (page) => tasksList(poolId, filter, query || null, page), { pageSize: LIST_PAGE, keyOf: (task) => task.id });
  // The plan's folds.
  const plan = usePagedList(`tasks-plan:${poolId}`, (page) => tasksPlan(poolId, page), { pageSize: PLAN_PAGE, keyOf: (task) => task.id });
  const first = list.items[0]?.id ?? null;
  useEffect(() => {
    if (first) setOpen((was) => was ?? first);
  }, [first]);

  const rows = list.items;
  const read = !list.loading;
  const total = counts ? countOf(counts, "open")! + countOf(counts, "closed")! : null;
  const error = list.error ? errorText(list.error) : null;
  return (
    <div className="flex w-full min-w-0 max-w-content flex-col gap-4" data-testid="pool-tasks">
      <div className="flex flex-wrap items-center gap-2">
        {view === "plan" ? (
          <p className="text-meta text-ink-2">The plan: each epic, its stories and their tasks. Open a fold to see what's under it.</p>
        ) : (
          <div role="radiogroup" aria-label="Which tasks" className="flex flex-wrap items-center gap-1">
            {FILTERS.map((word) => {
              const n = countOf(counts, word);
              const on = filter === word;
              return (
                <button
                  key={word}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  onClick={() => setFilter(word)}
                  className={"h-8 rounded-lg px-2.5 text-meta font-medium outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (on ? "bg-accent-soft text-accent-hi" : "text-muted hover:bg-surface-2 hover:text-ink")}
                  data-testid={`tasks-filter-${word}`}
                >
                  {FILTER_WORDS[word]}
                  {n !== null && n > 0 && <span className="ml-1.5 tabular-nums opacity-80">{n}</span>}
                </button>
              );
            })}
          </div>
        )}
        <div className="ml-auto flex flex-wrap items-center gap-2">
          <div role="radiogroup" aria-label="Show" className="flex rounded-lg border border-hairline p-0.5">
            {(["list", "plan"] as const).map((v) => (
              <button key={v} type="button" role="radio" aria-checked={view === v} onClick={() => setView(v)} className={"h-7 rounded-md px-2.5 text-meta " + (view === v ? "bg-surface-3 text-ink" : "text-muted hover:text-ink")} data-testid={`tasks-view-${v}`}>
                {v === "list" ? "List" : "Plan"}
              </button>
            ))}
          </div>
          {view === "list" && <input type="search" aria-label="Find tasks" placeholder="Find tasks" value={words} onChange={(event) => setWords(event.target.value)} className="h-8 w-44 rounded-lg border border-hairline bg-surface-2 px-3 text-meta text-ink outline-none focus:border-accent/60" />}
        </div>
      </div>
      <div className="grid min-w-0 gap-4 lg:grid-cols-[minmax(300px,420px)_minmax(0,1fr)]">
        <aside className="flex min-w-0 flex-col gap-2" aria-label="Tasks">
          {view === "plan" ? (
            plan.loading ? (
              <Skeleton label="Loading the plan" rows={3} />
            ) : (
              <TaskTree plan={plan.items} poolId={poolId} open={open} now={now} onOpen={setOpen} />
            )
          ) : !read ? (
            <Skeleton label="Loading the tasks" rows={4} />
          ) : rows.length === 0 ? (
            <p className="rounded-lg border border-dashed border-hairline px-3 py-4 text-meta text-ink-2" data-testid="tasks-empty">
              {query ? "No task has those words." : total ? `No ${FILTER_WORDS[filter]!.toLowerCase()} tasks.` : "No tasks yet. The pool's agents add them and take ready work with their own tools: ask one to plan the work."}
            </p>
          ) : (
            <ul className="flex flex-col gap-0.5" data-testid="task-list">
              {rows.map((task) => (
                <TaskRow key={task.id} task={task} now={now} selected={task.id === open} onOpen={() => setOpen(task.id)} />
              ))}
            </ul>
          )}
          {view === "list" && list.hasMore && (
            <Button size="sm" variant="ghost" onClick={list.loadMore} disabled={list.loadingMore} data-testid="tasks-more">
              {list.loadingMore ? "Reading more tasks…" : "Show more tasks"}
            </Button>
          )}
          {error && (
            <p role="alert" className="text-meta text-danger">
              {error}
            </p>
          )}
        </aside>
        <div className="min-w-0 rounded-xl border border-hairline bg-surface p-4">
          {open ? (
            <TaskView key={open} taskId={open} sessions={sessions} defaultAccountId={defaultAccount} now={now} onOpenTask={setOpen} />
          ) : read ? (
            <p className="text-meta text-ink-2" data-testid="task-none-open">
              The tasks the pool's agents write and work show here, with who has each and its history. You follow them; you approve or send back what's in review.
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}

/** A session's Tasks tab: what it holds and what it worked, a page at a
 * time; each opens in its pool's Tasks tab. */
export function SessionTasks({ session, now }: { session: InstanceInfo; now: number }): JSX.Element {
  const host = useActionHost();
  const poolId = usePoolPlace(session.id)?.pool_id;
  useTasksRefresh();
  const tasks = usePagedList(`tasks-session:${session.id}`, (page) => tasksOfSession(session.id, page), { keyOf: (task) => task.id });
  const open = (task: TaskInfo) => {
    focusTask(task.id);
    rememberTab("pool", "tasks");
    host.openPool?.(task.pool_id || poolId || "");
  };
  return (
    <div className="flex w-full min-w-0 max-w-[960px] flex-col gap-3" data-testid="session-tasks">
      <p className="text-meta text-ink-2">The tasks this session holds or worked, in its pool. A switch keeps them: its next session holds them too.</p>
      {tasks.loading ? (
        <span className="h-16 rounded-lg bg-skeleton motion-safe:animate-pulse" role="status" aria-label="Loading its tasks" />
      ) : (
        <ul className="flex flex-col gap-0.5">
          {tasks.items.map((task) => (
            <TaskRow key={task.id} task={task} now={now} selected={false} onOpen={() => open(task)} />
          ))}
        </ul>
      )}
      {tasks.hasMore && (
        <Button size="sm" variant="ghost" onClick={tasks.loadMore} disabled={tasks.loadingMore} className="self-start">
          {tasks.loadingMore ? "Reading more tasks…" : "Show more tasks"}
        </Button>
      )}
    </div>
  );
}
