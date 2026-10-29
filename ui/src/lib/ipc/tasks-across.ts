/**
 * 0.3 — the Tasks screen's reads: every pool's tasks at once, by state
 * (the board's columns) or as the plan, filtered by project, pool, holder
 * and words, a page at a time (the backend's `features/tasks/across.rs`).
 * Reads only (D-44): the actions on a task stay lib/ipc/tasks.ts's.
 *
 * The preview reads its own tasks (`__AIJ_TASKS__`) and pools.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { Page, PageRequest } from "../paged";
import { poolsList } from "./pools";
import { previewTasks, type TaskCounts, type TaskInfo } from "./tasks";
import type { TaskBoardRow } from "../../types/generated/TaskBoardRow";
import type { TasksFilterInput } from "../../types/generated/TasksFilterInput";
import type { ProjectTaskCounts } from "../../types/generated/ProjectTaskCounts";

export type { TaskBoardRow, TasksFilterInput, ProjectTaskCounts };

const OPEN = ["ready", "in_progress", "in_review", "blocked", "todo"];

/** The preview's take on the filter (the backend's rules). */
async function previewRows(filter: TasksFilterInput, withStatus: boolean): Promise<TaskBoardRow[]> {
  const pools = (await poolsList()).pools;
  const named = new Map(pools.map((pool) => [pool.id, pool]));
  const all = previewTasks.rows();
  const byId = new Map(all.map((task) => [task.id, task]));
  const plan = filter.kind === "plan";
  const needle = (filter.words ?? "").trim().toLowerCase();
  const status = filter.status ?? "open";
  const want = (task: TaskInfo) => {
    const pool = named.get(task.pool_id);
    if (filter.pool_id && task.pool_id !== filter.pool_id) return false;
    if (filter.project_id && pool?.project_id !== filter.project_id) return false;
    if (plan ? task.kind === "task" : task.kind !== "task") return false;
    if (filter.holder === "you" && task.holder !== "You") return false;
    if (filter.holder === "nobody" && task.holder !== null) return false;
    if (needle && !task.title.toLowerCase().includes(needle)) return false;
    if (!withStatus) return true;
    if (status === "open") return OPEN.includes(task.status);
    if (status === "closed") return task.status === "done" || task.status === "cancelled";
    return task.status === status;
  };
  const path = (task: TaskInfo) => {
    const out: string[] = [];
    let up = task.parent_id ? byId.get(task.parent_id) : undefined;
    while (up && out.length < 2) {
      out.unshift(up.title);
      up = up.parent_id ? byId.get(up.parent_id) : undefined;
    }
    return out;
  };
  return all
    .filter(want)
    .sort(previewTasks.order)
    .map((task) => ({ task: { ...task }, pool_name: named.get(task.pool_id)?.name ?? "", project_id: named.get(task.pool_id)?.project_id ?? null, path: path(task) }));
}

/** One page of the tasks the filter names, across pools, ready work first. */
export async function tasksAcross(filter: TasksFilterInput, page: PageRequest): Promise<Page<TaskBoardRow>> {
  if (!hasLiveBackend()) {
    const rows = await previewRows(filter, true);
    const from = page.cursor ? Number(page.cursor.slice(1)) : 0;
    const limit = Math.min(Math.max(page.limit ?? 50, 1), 200);
    return { items: rows.slice(from, from + limit), next_cursor: rows.length > from + limit ? `p${from + limit}` : null, total: rows.length };
  }
  return invoke<Page<TaskBoardRow>>("tasks_across", { filter, page });
}

/** How many of the tasks the filter names are in each status (its own
 * status aside): the board's column counts. */
export async function tasksAcrossCounts(filter: TasksFilterInput): Promise<TaskCounts> {
  if (!hasLiveBackend()) {
    const rows = await previewRows(filter, false);
    const n = (status: string) => rows.filter((row) => row.task.status === status).length;
    return { ready: n("ready"), in_progress: n("in_progress"), in_review: n("in_review"), blocked: n("blocked"), todo: n("todo"), done: n("done"), cancelled: n("cancelled") };
  }
  return invoke<TaskCounts>("tasks_across_counts", { filter });
}

/** Each project's tasks by status: a page of the Projects screen, in one
 * call. */
export async function tasksProjectCounts(projectIds: string[]): Promise<ProjectTaskCounts[]> {
  if (!hasLiveBackend()) {
    return Promise.all(projectIds.map(async (id) => ({ project_id: id, counts: await tasksAcrossCounts({ project_id: id }) })));
  }
  return invoke<ProjectTaskCounts[]>("tasks_project_counts", { projectIds });
}
