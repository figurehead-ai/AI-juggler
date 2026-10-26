/**
 * E19 — the tasks a pool shares (one IPC module per feature,
 * ENGINEERING-GUIDE U3). Only the app's window calls these: an agent
 * reaches tasks through its own tools, never here.
 *
 * Every write is the user's: a task, a change, an assignment, a link, an
 * answer to an agent's held change, a session started for a task (on the
 * launch settings the user picks, D-31), an issue looked up.
 *
 * The preview keeps its own tasks, seeded by `__AIJ_TASKS__`, and records
 * the calls that change them in `__AIJ_TASKS_CALLS__`.
 */
import { useEffect, useState } from "react";
import { invoke, listen } from "./transport";
import { hasLiveBackend, refused } from "../bridge";
import type { TaskInfo } from "../../types/generated/TaskInfo";
import type { TasksPage } from "../../types/generated/TasksPage";
import type { TaskDetail } from "../../types/generated/TaskDetail";
import type { TaskInput } from "../../types/generated/TaskInput";
import type { TaskEdit } from "../../types/generated/TaskEdit";
import type { TaskNeedInfo } from "../../types/generated/TaskNeedInfo";
import type { TaskStartedInfo } from "../../types/generated/TaskStartedInfo";

export type { TaskInfo, TasksPage, TaskDetail, TaskInput, TaskEdit, TaskNeedInfo, TaskStartedInfo };

/** The event the backend sends when tasks change (the pools' ids). */
export const TASKS_CHANGED = "tasks://changed";

/** A task's full view in the preview, beyond its row. */
export type PreviewDetail = Partial<Omit<TaskDetail, "task">>;

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
const PAGE = 40;
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
    task: { ...task },
    pool_name: extra.pool_name ?? "",
    body: extra.body ?? "",
    acceptance: extra.acceptance ?? "",
    parent_title: data.tasks.find((t) => t.id === task.parent_id)?.title ?? null,
    children: data.tasks.filter((t) => t.parent_id === taskId).sort(order),
    links: extra.links ?? [],
    sessions: extra.sessions ?? [],
    threads: extra.threads ?? [],
    history: extra.history ?? [],
    history_next: null,
    issue: extra.issue ?? null,
    session_asked: data.needs.find((n) => n.kind === "launch" && n.task_id === taskId)?.id ?? null,
  };
}

function counts(poolId: string): TasksPage["counts"] {
  const all = preview().tasks.filter((t) => t.pool_id === poolId && t.kind === "task");
  return RANK.map((status) => ({ status, count: all.filter((t) => t.status === status).length })).filter((c) => c.count > 0);
}

/** One page of a pool's tasks (the work: epics and stories are the
 * plan), ready work first. `filter`: `open` (the default), `closed`,
 * `mine`, `all`, or one status. */
export async function tasksList(poolId: string, filter: string | null = null, words: string | null = null, cursor: string | null = null): Promise<TasksPage> {
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
    const from = cursor ? Number(cursor.slice(1)) : 0;
    return { tasks: rows.slice(from, from + PAGE).map((t) => ({ ...t })), next: rows.length > from + PAGE ? `o${from + PAGE}` : null, counts: counts(poolId) };
  }
  return invoke<TasksPage>("tasks_list", { poolId, filter, words, cursor });
}

/** A pool's epics and stories (the plan's folds). */
export async function tasksPlan(poolId: string): Promise<TaskInfo[]> {
  if (!hasLiveBackend()) return preview().tasks.filter((t) => t.pool_id === poolId && t.kind !== "task").sort(order);
  return invoke<TaskInfo[]>("tasks_plan", { poolId });
}

/** One page of the items under an epic or story (`""`: tasks with none). */
export async function tasksChildren(poolId: string, parent: string, cursor: string | null = null): Promise<TasksPage> {
  if (!hasLiveBackend()) {
    const rows = preview()
      .tasks.filter((t) => t.pool_id === poolId && (parent ? t.parent_id === parent : !t.parent_id && t.kind === "task"))
      .sort(order);
    return { tasks: rows.map((t) => ({ ...t })), next: null, counts: [] };
  }
  return invoke<TasksPage>("tasks_children", { poolId, parent, cursor });
}

/** A session's tasks (what it holds, what it worked). */
export async function tasksOfSession(sessionId: string): Promise<TaskInfo[]> {
  if (!hasLiveBackend()) {
    const data = preview();
    const ids = data.sessions[sessionId] ?? [];
    return data.tasks.filter((t) => ids.includes(t.id) || t.holder_session_id === sessionId).sort(order);
  }
  return invoke<TaskInfo[]>("tasks_session", { sessionId });
}

/** One task in full (`cursor`: older history). */
export async function tasksDetail(taskId: string, cursor: string | null = null): Promise<TaskDetail> {
  if (!hasLiveBackend()) return previewDetail(taskId);
  return invoke<TaskDetail>("tasks_detail", { taskId, cursor });
}

/** You add a task. */
export async function tasksCreate(input: TaskInput): Promise<TaskDetail> {
  if (!hasLiveBackend()) {
    const title = input.title.trim();
    if (!title) throw refused("a task needs a title");
    record(`create:${input.kind}:${title}`);
    const data = preview();
    const id = `aj-${(0x1000 + data.tasks.length).toString(16)}`;
    data.tasks.push({
      id,
      pool_id: input.pool_id,
      kind: input.kind,
      parent_id: input.parent_id,
      title,
      status: input.status,
      stored_status: null,
      priority: input.priority,
      holder: null,
      holder_session_id: null,
      blocked_by: input.blocked_by,
      items: 0,
      items_done: 0,
      written_by: "You",
      by_agent: false,
      updated_at_ms: Date.now(),
      issue_label: null,
    });
    data.details[id] = { body: input.body, acceptance: input.acceptance, history: [{ id: 1, at_ms: Date.now(), who: "You", by_agent: false, what: "added it" }] };
    changed();
    return previewDetail(id);
  }
  return invoke<TaskDetail>("tasks_create", { input });
}

/** You change a task (any field, any status); what isn't said stays. */
export async function tasksEdit(change: Partial<TaskEdit> & { task_id: string }): Promise<TaskDetail> {
  const edit: TaskEdit = { title: null, body: null, acceptance: null, priority: null, status: null, parent_id: null, url: null, note: null, ...change };
  if (!hasLiveBackend()) {
    record(`edit:${edit.task_id}:${edit.status ?? ""}`);
    const task = preview().tasks.find((t) => t.id === edit.task_id);
    if (!task) throw refused("That task isn't there any more");
    if (edit.status) task.status = edit.status;
    if (edit.title) task.title = edit.title;
    if (edit.priority) task.priority = edit.priority;
    const extra = (preview().details[edit.task_id] ??= {});
    if (edit.body !== null && edit.body !== undefined) extra.body = edit.body;
    if (edit.acceptance !== null && edit.acceptance !== undefined) extra.acceptance = edit.acceptance;
    task.updated_at_ms = Date.now();
    changed();
    return previewDetail(edit.task_id);
  }
  return invoke<TaskDetail>("tasks_edit", { edit });
}

/** You give a task to a member (its session), to yourself (`you`) or to
 * no one (`nobody`). */
export async function tasksAssign(taskId: string, to: string): Promise<TaskDetail> {
  if (!hasLiveBackend()) {
    record(`assign:${taskId}:${to}`);
    const task = preview().tasks.find((t) => t.id === taskId);
    if (!task) throw refused("That task isn't there any more");
    task.holder = to === "nobody" ? null : to === "you" ? "You" : to;
    task.holder_session_id = to === "nobody" || to === "you" ? null : to;
    changed();
    return previewDetail(taskId);
  }
  return invoke<TaskDetail>("tasks_assign", { taskId, to });
}

/** You link two tasks (`blocked_by`, `blocks`, `relates_to`,
 * `duplicates`), or take the link back. */
export async function tasksLink(taskId: string, other: string, kind: string, on: boolean): Promise<TaskDetail> {
  if (!hasLiveBackend()) {
    record(`link:${taskId}:${kind}:${other}:${on}`);
    const extra = (preview().details[taskId] ??= {});
    const target = preview().tasks.find((t) => t.id === other);
    extra.links = (extra.links ?? []).filter((l) => !(l.task_id === other && l.kind === kind));
    if (on && target) extra.links.push({ kind, task_id: other, title: target.title, status: target.status });
    changed();
    return previewDetail(taskId);
  }
  return invoke<TaskDetail>("tasks_link", { taskId, other, kind, on });
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
    return { session_id: "sess-preview-new", task: { ...task } };
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
