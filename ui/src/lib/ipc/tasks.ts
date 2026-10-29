/**
 * E19 — the tasks a pool shares (one IPC module per feature,
 * ENGINEERING-GUIDE U3). Only the app's window calls these: an agent
 * reaches tasks through its own tools, never here.
 *
 * D-44: agents write and run the tasks; you see and track them here.
 * Nothing here makes, edits, assigns or links a task. Your only writes:
 * approve or send back a task in review, free one whose holder's session
 * ended, cancel one, answer an agent's held change or request, start a
 * session for a task (on the launch settings you pick, D-31), look an
 * issue up.
 *
 * The preview keeps its own tasks, seeded by `__AIJ_TASKS__`, and records
 * the calls that change them in `__AIJ_TASKS_CALLS__`.
 */
import { useEffect, useState } from "react";
import { invoke, listen } from "./transport";
import { hasLiveBackend, refused } from "../bridge";
import type { TaskInfo } from "../../types/generated/TaskInfo";
import type { TaskCounts } from "../../types/generated/TaskCounts";
import type { Page, PageRequest } from "../paged";
import type { TaskDetail } from "../../types/generated/TaskDetail";
import type { TaskActions } from "../../types/generated/TaskActions";
import type { TaskNeedInfo } from "../../types/generated/TaskNeedInfo";
import type { TaskStartedInfo } from "../../types/generated/TaskStartedInfo";

/** AJ-E1904: what a switch-history epoch adds for tasks: its session's
 * task (`aj-7f3c · title`), from the backend's `task` field. */
export type EpochTask = { task?: string | null };

export type { TaskInfo, TaskCounts, TaskDetail, TaskActions, TaskNeedInfo, TaskStartedInfo };

/** The event the backend sends when tasks change (the pools' ids). */
export const TASKS_CHANGED = "tasks://changed";

/** A task's full view in the preview, beyond its row (its actions follow
 * from the row, as the backend's do). */
export type PreviewDetail = Partial<Omit<TaskDetail, "task" | "actions">>;

/** What the preview starts with. */
export interface TasksSeed {
  tasks?: TaskInfo[];
  details?: Record<string, PreviewDetail>;
  needs?: TaskNeedInfo[];
  /** Each session's tasks (the session page's tab). */
  sessions?: Record<string, string[]>;
}
interface Seams {
  __AIJ_TASKS__?: TasksSeed;
  __AIJ_TASKS_CALLS__?: string[];
}
const seams = () => window as unknown as Seams;
const record = (call: string) => (seams().__AIJ_TASKS_CALLS__ ??= []).push(call);
const RANK = ["ready", "in_progress", "in_review", "blocked", "todo", "done", "cancelled"];
const PRIORITY = ["urgent", "high", "normal", "low"];

let store: Required<TasksSeed> | null = null;
const preview = (): Required<TasksSeed> => {
  if (store === null) {
    const seed = seams().__AIJ_TASKS__ ?? {};
    store = {
      tasks: (seed.tasks ?? []).map((t) => ({ ...t })),
      details: { ...(seed.details ?? {}) },
      needs: [...(seed.needs ?? [])],
      sessions: { ...(seed.sessions ?? {}) },
    };
  }
  return store;
};
/** A preview row as the backend answers it: what it waits on, by title
 * (none for a task that is gone). */
const answered = (task: TaskInfo): TaskInfo => ({
  ...task,
  waits_on: task.blocked_by.map((id) => ({ id, title: preview().tasks.find((t) => t.id === id)?.title ?? null })),
});
const listeners = new Set<() => void>();
const changed = () => listeners.forEach((fn) => fn());

/** The preview's card line for a session's task (`aj-7f3c · title (in
 * progress)`), or `None`, as the backend's card says it. */
export function previewHeldBy(sessionId: string): string {
  const task = preview().tasks.find((t) => t.holder_session_id === sessionId && ["in_progress", "in_review", "blocked"].includes(t.status));
  return task ? `${task.id} · ${task.title} (${task.status.replace("_", " ")})` : "None";
}

/** Tests: forget the preview's tasks between cases. */
export function resetPreviewTasks(): void {
  store = null;
}

const order = (a: TaskInfo, b: TaskInfo) =>
  RANK.indexOf(a.status) - RANK.indexOf(b.status) || PRIORITY.indexOf(a.priority) - PRIORITY.indexOf(b.priority) || b.updated_at_ms - a.updated_at_ms;

function previewDetail(taskId: string): TaskDetail {
  const data = preview();
  const task = data.tasks.find((t) => t.id === taskId);
  if (!task) throw refused("That task isn't there any more");
  const extra = data.details[taskId] ?? {};
  return {
    task: answered(task),
    pool_name: extra.pool_name ?? "",
    body: extra.body ?? "",
    acceptance: extra.acceptance ?? "",
    parent_title: data.tasks.find((t) => t.id === task.parent_id)?.title ?? null,
    children: data.tasks.filter((t) => t.parent_id === taskId).sort(order).map(answered),
    links: extra.links ?? [],
    sessions: extra.sessions ?? [],
    threads: extra.threads ?? [],
    history: extra.history ?? [],
    history_next: null,
    issue: extra.issue ?? null,
    session_asked: data.needs.find((n) => n.kind === "launch" && n.task_id === taskId)?.id ?? null,
    actions: previewActions(task),
  };
}

/** The preview's rows as one keyset page (the backend's cursor is
 * opaque; the preview's names the last row's index). */
function previewPage(rows: TaskInfo[], page: PageRequest): Page<TaskInfo> {
  const from = page.cursor ? Number(page.cursor.slice(1)) : 0;
  const limit = Math.min(Math.max(page.limit ?? 50, 1), 200);
  const items = rows.slice(from, from + limit).map(answered);
  return { items, next_cursor: rows.length > from + limit ? `p${from + limit}` : null, total: rows.length };
}

/** The preview's tasks, their order and its pages, for the reads across
 * pools (tasks-across.ts). */
export const previewTasks = { rows: (): TaskInfo[] => preview().tasks, order, page: previewPage, answered };

/** How many of a pool's tasks (the work) are in each status. */
export async function tasksCounts(poolId: string): Promise<TaskCounts> {
  if (!hasLiveBackend()) {
    const all = preview().tasks.filter((t) => t.pool_id === poolId && t.kind === "task");
    const n = (status: string) => all.filter((t) => t.status === status).length;
    return { ready: n("ready"), in_progress: n("in_progress"), in_review: n("in_review"), blocked: n("blocked"), todo: n("todo"), done: n("done"), cancelled: n("cancelled") };
  }
  return invoke<TaskCounts>("tasks_counts", { poolId });
}

/** One page of a pool's tasks (the work: epics and stories are the
 * plan), ready work first. `filter`: `open` (the default), `closed`,
 * `mine`, `all`, or one status. */
export async function tasksList(poolId: string, filter: string | null, words: string | null, page: PageRequest): Promise<Page<TaskInfo>> {
  if (!hasLiveBackend()) {
    const want = (t: TaskInfo) => {
      switch (filter ?? "open") {
        case "open":
          return t.status !== "done" && t.status !== "cancelled";
        case "closed":
          return t.status === "done" || t.status === "cancelled";
        case "mine":
          return t.holder === "You";
        case "all":
          return true;
        default:
          return t.status === filter;
      }
    };
    const needle = (words ?? "").trim().toLowerCase();
    const rows = preview()
      .tasks.filter((t) => t.pool_id === poolId && t.kind === "task" && want(t) && (!needle || t.title.toLowerCase().includes(needle)))
      .sort(order);
    return previewPage(rows, page);
  }
  return invoke<Page<TaskInfo>>("tasks_list", { poolId, filter, words, page });
}

/** One page of a pool's epics and stories (the plan's folds). */
export async function tasksPlan(poolId: string, page: PageRequest): Promise<Page<TaskInfo>> {
  if (!hasLiveBackend()) return previewPage(preview().tasks.filter((t) => t.pool_id === poolId && t.kind !== "task").sort(order), page);
  return invoke<Page<TaskInfo>>("tasks_plan", { poolId, page });
}

/** One page of the items under an epic or story (`""`: tasks with none). */
export async function tasksChildren(poolId: string, parent: string, page: PageRequest): Promise<Page<TaskInfo>> {
  if (!hasLiveBackend()) {
    const rows = preview()
      .tasks.filter((t) => t.pool_id === poolId && (parent ? t.parent_id === parent : !t.parent_id && t.kind === "task"))
      .sort(order);
    return previewPage(rows, page);
  }
  return invoke<Page<TaskInfo>>("tasks_children", { poolId, parent, page });
}

/** One page of a session's tasks (what it holds, what it worked). */
export async function tasksOfSession(sessionId: string, page: PageRequest): Promise<Page<TaskInfo>> {
  if (!hasLiveBackend()) {
    const data = preview();
    const ids = data.sessions[sessionId] ?? [];
    return previewPage(data.tasks.filter((t) => ids.includes(t.id) || t.holder_session_id === sessionId).sort(order), page);
  }
  return invoke<Page<TaskInfo>>("tasks_session", { sessionId, page });
}

/** One task in full (`cursor`: older history). */
export async function tasksDetail(taskId: string, cursor: string | null = null): Promise<TaskDetail> {
  if (!hasLiveBackend()) return previewDetail(taskId);
  return invoke<TaskDetail>("tasks_detail", { taskId, cursor });
}

/** What you may do to a preview task now (the backend's `may_user`). */
function previewActions(task: TaskInfo): TaskActions {
  const open = task.status !== "done" && task.status !== "cancelled";
  return {
    review: (task.stored_status ?? task.status) === "in_review",
    free: task.holder !== null && task.holder_session_id === null,
    cancel: open,
  };
}

/** The preview's take on one of your four actions: the same rules as the
 * backend, the change, and a line in the task's history. */
function previewAct(taskId: string, call: string, may: keyof TaskActions, change: (task: TaskInfo) => void, what: string): TaskDetail {
  record(call);
  const task = preview().tasks.find((t) => t.id === taskId);
  if (!task) throw refused("That task isn't there any more");
  if (!previewActions(task)[may]) throw refused("That task has moved on: look again");
  change(task);
  task.stored_status = null;
  task.updated_at_ms = Date.now();
  const extra = (preview().details[taskId] ??= {});
  const history = extra.history ?? [];
  extra.history = [{ id: (history[0]?.id ?? 0) + 1, at_ms: Date.now(), who: "You", by_agent: false, what }, ...history];
  changed();
  return previewDetail(taskId);
}

/** You approve a task its holder marked in review: it's done (D-44). */
export async function tasksApprove(taskId: string): Promise<TaskDetail> {
  if (!hasLiveBackend()) return previewAct(taskId, `approve:${taskId}`, "review", (t) => (t.status = "done"), "approved it: done");
  return invoke<TaskDetail>("tasks_approve", { taskId });
}

/** You send a task in review back to in progress, with a note its holder
 * gets as your message (D-44). */
export async function tasksSendBack(taskId: string, note: string): Promise<TaskDetail> {
  if (!hasLiveBackend()) {
    if (!note.trim()) throw refused("Say what should change: a short note");
    return previewAct(taskId, `send-back:${taskId}:${note.trim()}`, "review", (t) => (t.status = t.holder ? "in_progress" : "ready"), `sent it back: ${note.trim()}`);
  }
  return invoke<TaskDetail>("tasks_send_back", { taskId, note });
}

/** You free a task whose holder's session ended or crashed, so another
 * agent can take it (D-44). */
export async function tasksFree(taskId: string): Promise<TaskDetail> {
  if (!hasLiveBackend()) {
    return previewAct(
      taskId,
      `free:${taskId}`,
      "free",
      (t) => {
        if (t.status === "in_progress") t.status = "ready";
        t.holder = null;
        t.holder_session_id = null;
      },
      "freed it: its session had ended",
    );
  }
  return invoke<TaskDetail>("tasks_free", { taskId });
}

/** You cancel an open task (D-44). */
export async function tasksCancel(taskId: string): Promise<TaskDetail> {
  if (!hasLiveBackend()) return previewAct(taskId, `cancel:${taskId}`, "cancel", (t) => (t.status = "cancelled"), "cancelled it");
  return invoke<TaskDetail>("tasks_cancel", { taskId });
}

/** What about tasks waits on you (Needs you). */
export async function tasksNeeds(): Promise<TaskNeedInfo[]> {
  if (!hasLiveBackend()) return preview().needs.map((n) => ({ ...n }));
  return invoke<TaskNeedInfo[]>("tasks_needs");
}

/** Your answer to an agent's held change: once, always (a rule in this
 * pool) or deny. */
export async function tasksAnswer(needId: string, answer: "once" | "always" | "deny"): Promise<void> {
  if (!hasLiveBackend()) {
    record(`answer:${needId}:${answer}`);
    preview().needs = preview().needs.filter((n) => n.id !== needId);
    changed();
    return;
  }
  return invoke<void>("tasks_answer", { needId, answer });
}

/** You dismiss an agent's request for a new session. */
export async function tasksDismiss(needId: string): Promise<void> {
  if (!hasLiveBackend()) {
    record(`dismiss:${needId}`);
    preview().needs = preview().needs.filter((n) => n.id !== needId);
    changed();
    return;
  }
  return invoke<void>("tasks_dismiss", { needId });
}

/** You start a session for a task, on the launch settings you picked. */
export async function tasksStart(taskId: string, profileId: string, needId: string | null = null): Promise<TaskStartedInfo> {
  if (!hasLiveBackend()) {
    record(`start:${taskId}:${profileId}:${needId ?? ""}`);
    const data = preview();
    data.needs = data.needs.filter((n) => n.id !== needId);
    const task = data.tasks.find((t) => t.id === taskId);
    if (!task) throw refused("That task isn't there any more");
    Object.assign(task, { status: "in_progress", holder: "New session", holder_session_id: "sess-preview-new" });
    changed();
    return { session_id: "sess-preview-new", task: answered(task) };
  }
  return invoke<TaskStartedInfo>("tasks_start", { taskId, profileId, needId });
}

/** You look a task's issue up (through your own `gh`). */
export async function tasksLookUp(taskId: string): Promise<TaskDetail> {
  if (!hasLiveBackend()) {
    record(`look-up:${taskId}`);
    return previewDetail(taskId);
  }
  return invoke<TaskDetail>("tasks_look_up", { taskId });
}

/** Calls `handler` when tasks change (the backend's event, or the
 * preview's own writes). */
export function onTasksChanged(handler: () => void): () => void {
  if (!hasLiveBackend()) {
    listeners.add(handler);
    return () => listeners.delete(handler);
  }
  let unlisten: (() => void) | null = null;
  let gone = false;
  void listen<string[]>(TASKS_CHANGED, () => handler()).then((off) => {
    if (gone) off();
    else unlisten = off;
  });
  return () => {
    gone = true;
    unlisten?.();
  };
}

/** What waits on you about tasks, kept current. */
export function useTaskNeeds(): TaskNeedInfo[] {
  const [needs, setNeeds] = useState<TaskNeedInfo[]>([]);
  useEffect(() => {
    let alive = true;
    const read = () => void tasksNeeds().then((next) => alive && setNeeds(next), () => undefined);
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, []);
  return needs;
}

let focused: string | null = null;
/** The task a pool's Tasks tab opens on next (from Needs you or a
 * session's page). */
export function focusTask(taskId: string | null): void {
  focused = taskId;
}
/** …taken once. */
export function takeFocusedTask(): string | null {
  const task = focused;
  focused = null;
  return task;
}
