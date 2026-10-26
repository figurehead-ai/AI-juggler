/**
 * E19 — what about tasks waits on you, in Home's Needs you:
 * - an agent's change above its pool's level (add, take, change or give a
 *   task), as the same approval card as every agent's ask (D-28): Allow
 *   once, Always allow in this pool (from the second time), Deny; keys
 *   1–9, Enter for the first;
 * - an agent's request for a new session for a task (D-31): you pick the
 *   launch settings and start it here, one click once picked, or leave it.
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { AskCard } from "./AskCard";
import { useActionHost } from "./ItemMenu";
import { rememberTab } from "../pages/registry";
import { focusTask, tasksAnswer, tasksDismiss, type TaskNeedInfo } from "../lib/ipc/tasks";
import type { AskInfo } from "../lib/ipc/runtime";
import type { ProjectInfo } from "../lib/bridge";
import { StartSession } from "../pages/tasks/StartSession";

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

function LaunchRow({ need, projects }: { need: TaskNeedInfo; projects: ProjectInfo[] }): JSX.Element {
  const host = useActionHost();
  const [busy, setBusy] = useState(false);
  const project = projects.find((p) => p.id === need.project_id);
  const open = () => {
    if (need.task_id) focusTask(need.task_id);
    rememberTab("pool", "tasks");
    host.openPool?.(need.pool_id);
  };
  return (
    <li className="flex min-w-0 flex-col gap-2 border-b border-hairline/40 py-2.5 last:border-b-0" data-testid="task-need-launch" data-home-item>
      <div className="flex min-w-0 items-start gap-2.5">
        <span aria-hidden="true" className="flex size-[22px] shrink-0 items-center justify-center rounded-lg bg-accent-soft text-[12px] text-accent-hi">
          ▶
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="text-[13px] font-medium text-ink">{need.title}</span>
          <span className="text-[12px] text-muted">
            {need.pool_name}
            {project ? ` · in ${project.name}` : ""} · {need.detail}
          </span>
        </span>
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
      </div>
      {need.task_id && (
        <div className="pl-8">
          <StartSession taskId={need.task_id} needId={need.id} defaultAccountId={project?.default_account_id ?? null} compact />
        </div>
      )}
    </li>
  );
}

export function TaskNeeds({ needs, projects }: { needs: TaskNeedInfo[]; projects: ProjectInfo[] }): JSX.Element | null {
  if (needs.length === 0) return null;
  const asks = needs.filter((need) => need.kind === "ask");
  const launches = needs.filter((need) => need.kind === "launch");
  return (
    <div className="flex flex-col gap-2" data-testid="task-needs">
      {asks.map((need) => (
        <AskCard key={need.id} ask={asAsk(need)} kindLabel="An agent asks about a task" title={need.pool_name} answer={(option) => tasksAnswer(need.id, option as "once" | "always" | "deny")} />
      ))}
      {launches.length > 0 && <ul className="flex flex-col">{launches.map((need) => <LaunchRow key={need.id} need={need} projects={projects} />)}</ul>}
    </div>
  );
}
