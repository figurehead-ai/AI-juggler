/**
 * E19 — what about tasks waits on you, in Home's Needs you:
 * - an agent's change above its pool's level (add, take, change or give a
 *   task), as the same approval card as every agent's ask (D-28): Allow
 *   once, Always allow in this pool (from the second time), Deny; keys
 *   1–9, Enter for the first;
 * - an agent's request for a new session for a task (D-31): you pick the
 *   launch settings and start it here, one click once picked, or leave it.
 *
 * SK-14: the approval card is one component, `TaskAsk`, drawn wherever a
 * held change is named: here, on its task's page, and in its pool's Tasks
 * (the rail's and the bell's rows open the task, which offered no answer).
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { IconCheck } from "./icons-controls";
import { AskCard } from "./AskCard";
import { useActionHost } from "./ItemMenu";
import { rememberTab } from "../pages/registry";
import { focusTask, tasksAnswer, tasksDismiss, useTaskNeeds, type TaskNeedInfo } from "../lib/ipc/tasks";
import type { AskInfo } from "../lib/ipc/runtime";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { StartSession } from "../pages/tasks/StartSession";
import { NeedActions, NeedByLine, needCardClass } from "./NeedCard";
import { minutes } from "../lib/time-words";

function asAsk(need: TaskNeedInfo): AskInfo {
  return {
    id: need.id,
    session_id: "",
    kind: "approval",
    approval_kind: null,
    summary: need.title,
    reason: [need.task_title ? `«${need.task_title}»` : null, need.detail].filter(Boolean).join(" · "),
    options: [
      { id: "once", label: "Allow once", effect: "allow_once", rule: null },
      ...(need.offer_always ? [{ id: "always", label: "Always allow in this pool", effect: "allow_rule", rule: null }] : []),
      { id: "deny", label: "Deny", effect: "deny", rule: null },
    ],
    parts: [],
    asked_at_ms: need.at_ms,
  };
}

/** An agent's held change about a task, as the same approval card as every
 * agent's ask: Allow once, Always allow in this pool (from the second
 * time), Deny. */
export function TaskAsk({ need }: { need: TaskNeedInfo }): JSX.Element {
  return <AskCard ask={asAsk(need)} kindLabel="An agent asks about a task" title={need.pool_name} answer={(option) => tasksAnswer(need.id, option as "once" | "always" | "deny")} />;
}

/** The held changes that wait on you, kept current: a pool's, or the ones
 * naming one task. */
export function useTaskAsks(where: { poolId?: string; taskId?: string }): TaskNeedInfo[] {
  return useTaskNeeds().filter((need) => need.kind === "ask" && (where.poolId === undefined || need.pool_id === where.poolId) && (where.taskId === undefined || need.task_id === where.taskId));
}

/** Opens a task need's task, beside its pool's tasks. */
function useOpenTaskNeed(need: TaskNeedInfo): () => void {
  const host = useActionHost();
  return () => {
    if (need.task_id) focusTask(need.task_id);
    rememberTab("pool", "tasks");
    host.openPool?.(need.pool_id);
  };
}

/** Its card's by-line: its pool, and where it would run. */
function placeOf(need: TaskNeedInfo, projects: ProjectInfo[]): string | null {
  const project = projects.find((p) => p.id === need.project_id);
  return project ? `in ${project.name}` : null;
}

function LaunchCard({ need, projects, now }: { need: TaskNeedInfo; projects: ProjectInfo[]; now?: number }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const project = projects.find((p) => p.id === need.project_id);
  const open = useOpenTaskNeed(need);
  return (
    <li className={needCardClass(2)} data-testid="task-need-launch" data-need-key={`task:${need.id}`} data-home-item>
      <span aria-hidden="true" className="flex size-[22px] shrink-0 items-center justify-center rounded-md bg-accent-soft text-label text-accent-hi">
        ▶
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <TaskNeedText need={need} projects={projects} now={now} />
        {need.task_id && (
          <div className="mt-1.5">
            <StartSession taskId={need.task_id} needId={need.id} defaultAccountId={project?.default_account_id ?? null} compact />
          </div>
        )}
        <NeedActions>
          <Button size="sm" variant="ghost" onClick={open}>
            Read the task<span className="sr-only"> — {need.task_title}</span>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void tasksDismiss(need.id).finally(() => setBusy(false));
            }}
          >
            Not now<span className="sr-only"> — {need.title}</span>
          </Button>
        </NeedActions>
      </span>
    </li>
  );
}

/** A task need's pool, place and time, then its title and detail. */
function TaskNeedText({ need, projects, now }: { need: TaskNeedInfo; projects: ProjectInfo[]; now?: number }): JSX.Element {
  return (
    <>
      <NeedByLine author={need.pool_name} place={placeOf(need, projects)} time={now === undefined ? null : minutes(need.at_ms, now)} />
      <span className="text-body text-ink">
        <span className="font-semibold">{need.title}</span>
        {need.detail && <span className="text-muted"> — {need.detail}</span>}
      </span>
    </>
  );
}

/** AJ-E2806 R7: a task waiting on your review; it opens beside the pool's
 * tasks, where Approve and Send back are. */
function ReviewCard({ need, projects, now }: { need: TaskNeedInfo; projects: ProjectInfo[]; now?: number }): JSX.Element {
  const open = useOpenTaskNeed(need);
  return (
    <li className={needCardClass(2)} data-testid="task-need-review" data-need-key={`task:${need.id}`} data-home-item>
      <span aria-hidden="true" className="flex size-[22px] shrink-0 items-center justify-center rounded-md bg-accent-soft text-accent-hi">
        <IconCheck width={13} height={13} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <TaskNeedText need={need} projects={projects} now={now} />
        <NeedActions>
          <Button size="sm" variant="ghost" onClick={open} data-testid="task-need-review-open">
            Review it<span className="sr-only"> — {need.task_title}</span>
          </Button>
        </NeedActions>
      </span>
    </li>
  );
}

/** A task's need that isn't an approval, as its own card (D-118's mock):
 * a request for a session, or a task waiting on your review. */
export function TaskNeedCard({ need, projects, now }: { need: TaskNeedInfo; projects: ProjectInfo[]; now?: number }): JSX.Element | null {
  if (need.kind === "launch") return <LaunchCard need={need} projects={projects} now={now} />;
  if (need.kind === "review") return <ReviewCard need={need} projects={projects} now={now} />;
  return null;
}

/** Whether it is an approval (its card is the asks' card). */
export function isTaskAsk(need: TaskNeedInfo): boolean {
  return need.kind === "ask";
}

export function TaskNeeds({ needs, projects }: { needs: TaskNeedInfo[]; projects: ProjectInfo[] }): JSX.Element | null {
  if (needs.length === 0) return null;
  const asks = needs.filter(isTaskAsk);
  const rest = needs.filter((need) => !isTaskAsk(need));
  return (
    <div className="flex flex-col gap-2" data-testid="task-needs">
      {asks.map((need) => (
        <TaskAsk key={need.id} need={need} />
      ))}
      {rest.length > 0 && <ul className="flex flex-col gap-2">{rest.map((need) => <TaskNeedCard key={need.id} need={need} projects={projects} />)}</ul>}
    </div>
  );
}
