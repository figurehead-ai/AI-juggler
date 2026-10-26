/**
 * E19 (AJ-E1903) — a pool's Tasks tab: its tasks on the left, ready work
 * first, a page at a time, with how many are in each state; the epic and
 * story plan behind its own fold; the open task (or a new one) on the
 * right. Loaded only when the tab is first shown.
 *
 * Also a session's Tasks tab: what it holds and what it worked.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { usePools } from "../../lib/pools";
import { rememberTab } from "../registry";
import { useActionHost } from "../../components/ItemMenu";
import { focusTask, onTasksChanged, takeFocusedTask, tasksList, tasksOfSession, tasksPlan, type TaskInfo, type TasksPage } from "../../lib/ipc/tasks";
import { TaskForm } from "./TaskForm";
import { TaskRow } from "./TaskRow";
import { TaskTree } from "./TaskTree";
import { TaskView } from "./TaskView";
import { STATUS_WORDS } from "./words";

const FILTERS = ["open", "ready", "in_progress", "in_review", "blocked", "closed"] as const;
const FILTER_WORDS: Record<string, string> = { ...STATUS_WORDS, open: "Open", closed: "Done" };

function countOf(page: TasksPage | null, filter: string): number | null {
  if (!page) return null;
  const by = (status: string) => page.counts.find((c) => c.status === status)?.count ?? 0;
  if (filter === "open") return page.counts.filter((c) => c.status !== "done" && c.status !== "cancelled").reduce((n, c) => n + c.count, 0);
  if (filter === "closed") return by("done") + by("cancelled");
  return by(filter);
}

export function PoolTasks({ poolId, members, sessions, projects, now }: { poolId: string; members: { session_id: string; name: string }[]; sessions: InstanceInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const pool = usePools()?.pools.find((p) => p.id === poolId);
  const defaultAccount = projects.find((p) => p.id === pool?.project_id)?.default_account_id ?? null;
  const [filter, setFilter] = useState<string>("open");
  const [view, setView] = useState<"list" | "plan">("list");
  const [words, setWords] = useState("");
  const [page, setPage] = useState<TasksPage | null>(null);
  const [more, setMore] = useState<TaskInfo[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [plan, setPlan] = useState<TaskInfo[]>([]);
  const [open, setOpen] = useState<string | null>(takeFocusedTask);
  const [writing, setWriting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    const read = () => {
      void tasksList(poolId, filter, words.trim() || null).then(
        (first) => {
          if (!alive) return;
          setPage(first);
          setMore([]);
          setNext(first.next);
          setOpen((was) => was ?? first.tasks[0]?.id ?? null);
        },
        (cause) => alive && setError(errorText(cause)),
      );
      void tasksPlan(poolId).then((rows) => alive && setPlan(rows), () => undefined);
    };
    const timer = window.setTimeout(read, words ? 200 : 0);
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      window.clearTimeout(timer);
      off();
    };
  }, [poolId, filter, words]);

  const rows = [...(page?.tasks ?? []), ...more];
  const total = page?.counts.reduce((n, c) => n + c.count, 0) ?? null;
  return (
    <div className="flex w-full min-w-0 max-w-[1440px] flex-col gap-4" data-testid="pool-tasks">
      <div className="flex flex-wrap items-center gap-2">
        {view === "plan" ? (
          <p className="text-[13px] text-muted">The plan: each epic, its stories and their tasks. Open a fold to see what's under it.</p>
        ) : (
        <div role="radiogroup" aria-label="Which tasks" className="flex flex-wrap items-center gap-1">
          {FILTERS.map((word) => {
            const n = countOf(page, word);
            const on = filter === word;
            return (
              <button
                key={word}
                type="button"
                role="radio"
                aria-checked={on}
                onClick={() => setFilter(word)}
                className={"h-8 rounded-lg px-2.5 text-[13px] font-medium outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (on ? "bg-accent-soft text-accent-hi" : "text-muted hover:bg-surface-2 hover:text-ink")}
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
              <button key={v} type="button" role="radio" aria-checked={view === v} onClick={() => setView(v)} className={"h-7 rounded-md px-2.5 text-[13px] " + (view === v ? "bg-surface-3 text-ink" : "text-muted hover:text-ink")} data-testid={`tasks-view-${v}`}>
                {v === "list" ? "List" : "Plan"}
              </button>
            ))}
          </div>
          {view === "list" && <input type="search" aria-label="Find tasks" placeholder="Find tasks" value={words} onChange={(event) => setWords(event.target.value)} className="h-8 w-44 rounded-lg border border-hairline bg-surface-2 px-3 text-[13px] text-ink outline-none focus:border-accent/60" />}
          <Button size="md" variant="primary" onClick={() => setWriting(true)} data-testid="tasks-new">
            New task
          </Button>
        </div>
      </div>
      <div className="grid min-w-0 gap-4 lg:grid-cols-[minmax(300px,420px)_minmax(0,1fr)]">
        <aside className="flex min-w-0 flex-col gap-2" aria-label="Tasks">
          {view === "plan" ? (
            <TaskTree plan={plan} poolId={poolId} open={writing ? null : open} now={now} onOpen={(id) => (setOpen(id), setWriting(false))} />
          ) : page === null ? (
            <div className="flex flex-col gap-2" role="status" aria-label="Loading the tasks">
              {[0, 1, 2, 3].map((n) => (
                <span key={n} className="h-16 rounded-lg bg-surface-3 motion-safe:animate-pulse" />
              ))}
            </div>
          ) : rows.length === 0 ? (
            <p className="rounded-lg border border-dashed border-hairline px-3 py-4 text-[13px] text-muted" data-testid="tasks-empty">
              {words ? "No task has those words." : total ? `No ${FILTER_WORDS[filter]!.toLowerCase()} tasks.` : "No tasks yet. Add one, or let the pool's agents add theirs: they take ready work with their own tools."}
            </p>
          ) : (
            <ul className="flex flex-col gap-0.5" data-testid="task-list">
              {rows.map((task) => (
                <TaskRow key={task.id} task={task} now={now} selected={!writing && task.id === open} onOpen={() => (setOpen(task.id), setWriting(false))} />
              ))}
            </ul>
          )}
          {view === "list" && next && (
            <Button size="sm" variant="ghost" onClick={() => void tasksList(poolId, filter, words.trim() || null, next).then((older) => (setMore((was) => [...was, ...older.tasks]), setNext(older.next)))} data-testid="tasks-more">
              Show more tasks
            </Button>
          )}
          {error && (
            <p role="alert" className="text-[13px] text-danger">
              {error}
            </p>
          )}
        </aside>
        <div className="min-w-0 rounded-xl border border-hairline bg-surface p-4">
          {writing || (open === null && page !== null) ? (
            <div className="flex flex-col gap-3">
              <h3 className="text-[16px] font-semibold text-ink">New task</h3>
              <TaskForm poolId={poolId} plan={plan} onMade={(detail) => (setWriting(false), setOpen(detail.task.id))} onCancel={open ? () => setWriting(false) : undefined} />
            </div>
          ) : open ? (
            <TaskView key={open} taskId={open} members={members} sessions={sessions} defaultAccountId={defaultAccount} now={now} onOpenTask={(id) => (setOpen(id), setWriting(false))} />
          ) : null}
        </div>
      </div>
    </div>
  );
}

/** A session's Tasks tab: what it holds and what it worked; each opens
 * in its pool's Tasks tab. */
export function SessionTasks({ session, now }: { session: InstanceInfo; now: number }): JSX.Element {
  const host = useActionHost();
  const poolId = usePools()?.members.find((row) => row.session_id === session.id)?.pool_id;
  const [tasks, setTasks] = useState<TaskInfo[] | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () => void tasksOfSession(session.id).then((rows) => alive && setTasks(rows), () => undefined);
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [session.id]);
  const open = (task: TaskInfo) => {
    focusTask(task.id);
    rememberTab("pool", "tasks");
    host.openPool?.(task.pool_id || poolId || "");
  };
  return (
    <div className="flex w-full min-w-0 max-w-[960px] flex-col gap-3" data-testid="session-tasks">
      <p className="text-[13px] text-muted">The tasks this session holds or worked, in its pool. A switch keeps them: its next session holds them too.</p>
      {tasks === null ? (
        <span className="h-16 rounded-lg bg-surface-3 motion-safe:animate-pulse" role="status" aria-label="Loading its tasks" />
      ) : (
        <ul className="flex flex-col gap-0.5">
          {tasks.map((task) => (
            <TaskRow key={task.id} task={task} now={now} selected={false} onOpen={() => open(task)} />
          ))}
        </ul>
      )}
    </div>
  );
}
