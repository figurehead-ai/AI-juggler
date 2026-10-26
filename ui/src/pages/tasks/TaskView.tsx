/**
 * E19 (AJ-E1903) — one task in full: what it asks and when it's done,
 * where it is and who has it (their session, one click away), the
 * sessions that worked it, the threads that name it, what it waits on,
 * its items, the issue it comes from, and its history. You change any of
 * it: its status, who has it, its links; or start a session for it, on
 * the launch settings you pick (D-31). An agent's request for one shows
 * here and in Needs you.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Pill } from "../../components/Pill";
import { errorText, type InstanceInfo } from "../../lib/bridge";
import { focusThread } from "../../lib/ipc/messages";
import { onTasksChanged, tasksAssign, tasksDetail, tasksEdit, tasksLookUp, type TaskDetail } from "../../lib/ipc/tasks";
import { processesLinkFor } from "../../lib/task-links";
import { ago } from "../../lib/time-words";
import { useActionHost } from "../../components/ItemMenu";
import { PlaceMark, markPlace } from "../../components/PlaceMarks";
import { showTab } from "../PageShell";
import { StartSession } from "./StartSession";
import { TaskLinks } from "./TaskLinks";
import { TaskRow, TaskStatus } from "./TaskRow";
import { HOW_WORDS, KIND_WORDS, PRIORITY_WORDS, STATUSES, STATUS_WORDS } from "./words";

/** Tasks are worked by the sessions AI Juggler runs: its own place. */
const OWN = markPlace(undefined, true);

const SELECT = "h-8 rounded-lg border border-hairline bg-surface px-2 text-meta text-ink outline-none focus:border-accent/60";

function Section({ title, children, testId }: { title: string; children: React.ReactNode; testId?: string }): JSX.Element {
  return (
    <section className="flex flex-col gap-1.5" data-testid={testId}>
      <h4 className="text-label font-semibold uppercase tracking-wider text-muted">{title}</h4>
      {children}
    </section>
  );
}

export function TaskView({
  taskId,
  members,
  sessions,
  defaultAccountId,
  now,
  onOpenTask,
}: {
  taskId: string;
  members: { session_id: string; name: string }[];
  sessions: InstanceInfo[];
  defaultAccountId: string | null;
  now: number;
  onOpenTask: (id: string) => void;
}): JSX.Element {
  const host = useActionHost();
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [starting, setStarting] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setDetail(null);
    setStarting(false);
    const read = () => void tasksDetail(taskId).then((next) => alive && setDetail(next), (cause) => alive && setError(errorText(cause)));
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [taskId]);
  const act = (run: () => Promise<TaskDetail>) => {
    setBusy(true);
    setError(null);
    void run().then(
      (next) => (setBusy(false), setDetail(next)),
      (cause) => (setBusy(false), setError(errorText(cause))),
    );
  };
  const profileOf = (id: string | null) => sessions.find((s) => s.id === id)?.account_label ?? null;
  const openSession = (id: string | null) => {
    const row = sessions.find((s) => s.id === id);
    if (row) host.openSession?.(row);
  };
  if (!detail) {
    return (
      <div className="flex flex-col gap-3 p-1" role="status" aria-label="Loading the task">
        <span className="h-6 w-2/3 rounded-lg bg-surface-3 motion-safe:animate-pulse" />
        <span className="h-16 rounded-lg bg-surface-3 motion-safe:animate-pulse" />
        {error && <p className="text-meta text-danger">{error}</p>}
      </div>
    );
  }
  const { task } = detail;
  const open = task.status !== "done" && task.status !== "cancelled";
  const processes = task.holder_session_id ? processesLinkFor(task.holder_session_id) : null;
  return (
    <article className="flex min-w-0 flex-col gap-5" aria-labelledby={`task-${task.id}`} data-testid="task-view">
      <header className="flex flex-col gap-2 border-b border-hairline/60 pb-3">
        <div className="flex flex-wrap items-center gap-2 text-label text-muted">
          <span className="font-semibold uppercase tracking-wider">{KIND_WORDS[task.kind]}</span>
          <span className="font-mono">{task.id}</span>
          {detail.parent_title && task.parent_id && (
            <button type="button" className="truncate underline decoration-hairline underline-offset-2 hover:text-ink" onClick={() => onOpenTask(task.parent_id!)}>
              in «{detail.parent_title}»
            </button>
          )}
        </div>
        <h3 id={`task-${task.id}`} className="text-[16px] font-semibold leading-snug text-ink">
          {task.title}
        </h3>
        <div className="flex flex-wrap items-center gap-2">
          <TaskStatus status={task.status} />
          {task.priority !== "normal" && <Pill tone={task.priority === "low" ? "neutral" : "warning"}>{PRIORITY_WORDS[task.priority] ?? task.priority}</Pill>}
          {task.stored_status && <span className="text-meta text-danger">Set {STATUS_WORDS[task.stored_status]?.toLowerCase()}, but it waits on {task.blocked_by.join(", ")}</span>}
          <span className="text-meta text-muted">
            Written by {task.written_by}
            {task.by_agent ? " (an agent: its words, not yours)" : ""} · changed {ago(task.updated_at_ms, now)}
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-2 pt-1">
          <label className="flex items-center gap-1.5 text-meta text-muted">
            Status
            <select aria-label="Status" value={task.stored_status ?? task.status} disabled={busy} className={SELECT} onChange={(event) => act(() => tasksEdit({ task_id: task.id, status: event.target.value }))} data-testid="task-set-status">
              {STATUSES.map((word) => (
                <option key={word} value={word} disabled={word === "in_progress" && !task.holder && task.kind === "task"}>
                  {STATUS_WORDS[word]}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-1.5 text-meta text-muted">
            With
            <select aria-label="Who has it" value={task.holder_session_id ?? (task.holder === "You" ? "you" : task.holder ? "held" : "nobody")} disabled={busy} className={SELECT} onChange={(event) => act(() => tasksAssign(task.id, event.target.value))} data-testid="task-set-holder">
              <option value="nobody">No one (anyone can take it)</option>
              <option value="you">You</option>
              {task.holder && !task.holder_session_id && task.holder !== "You" && <option value="held">{task.holder} (not running)</option>}
              {members.map((member) => (
                <option key={member.session_id} value={member.session_id}>
                  {member.name}
                </option>
              ))}
            </select>
          </label>
          {open && !starting && !task.holder_session_id && (
            <Button size="sm" variant={detail.session_asked ? "primary" : "secondary"} onClick={() => setStarting(true)} data-testid="task-start-open">
              Start a session for it…
            </Button>
          )}
        </div>
        {detail.session_asked && !starting && <p className="text-meta text-warning">An agent asked you for a new session for it. You pick the account and start it, or leave it.</p>}
        {starting && <StartSession taskId={task.id} needId={detail.session_asked} defaultAccountId={defaultAccountId} onCancel={() => setStarting(false)} onStarted={() => setStarting(false)} />}
      </header>
      {task.holder_session_id && (
        <div className="flex flex-wrap items-center gap-2.5 rounded-lg border border-hairline bg-surface-2/60 px-3 py-2 text-meta" data-testid="task-holder">
          {/* D-38: the app it runs in (AI Juggler's own), with its profile. */}
          <PlaceMark place={OWN} profile={profileOf(task.holder_session_id)} size={20} ring="var(--color-surface-2)" />
          <span className="min-w-0 flex-1 text-ink">
            {STATUS_WORDS[task.status]} with <span className="font-medium">{task.holder}</span>, running now
          </span>
          <Button size="sm" variant="secondary" onClick={() => openSession(task.holder_session_id)}>
            Open its session
          </Button>
          {processes && (
            <Button size="sm" variant="ghost" onClick={() => (processes.open(), openSession(task.holder_session_id))} data-testid="task-processes">
              {processes.label}
            </Button>
          )}
        </div>
      )}
      {detail.body && (
        <Section title="What it asks">
          <p className="whitespace-pre-wrap break-words text-body leading-relaxed text-ink">{detail.body}</p>
        </Section>
      )}
      {detail.acceptance && (
        <Section title="Done when">
          <p className="whitespace-pre-wrap break-words text-body leading-relaxed text-ink" data-testid="task-acceptance">
            {detail.acceptance}
          </p>
        </Section>
      )}
      {detail.children.length > 0 && (
        <Section title={`Items · ${task.items_done} of ${task.items} done`} testId="task-children">
          <ul className="flex flex-col gap-0.5">
            {detail.children.map((child) => (
              <TaskRow key={child.id} task={child} now={now} selected={false} onOpen={() => onOpenTask(child.id)} />
            ))}
          </ul>
        </Section>
      )}
      <TaskLinks detail={detail} busy={busy} act={act} onOpenTask={onOpenTask} />
      {detail.issue && (
        <Section title="Issue" testId="task-issue">
          <div className="flex flex-wrap items-center gap-2 text-meta">
            <a href={detail.issue.url} target="_blank" rel="noreferrer" className="font-mono text-accent-hi underline underline-offset-2">
              {detail.issue.label ?? detail.issue.url}
            </a>
            {detail.issue.state && <Pill tone={detail.issue.state === "open" ? "accent" : "success"}>{detail.issue.state}</Pill>}
            {detail.issue.title && <span className="min-w-0 truncate text-ink">{detail.issue.title}</span>}
            {detail.issue.checked_at_ms !== null && <span className="text-muted">looked up {ago(detail.issue.checked_at_ms, now)}</span>}
            {detail.issue.can_look_up && (
              <Button size="sm" variant="ghost" disabled={busy} onClick={() => act(() => tasksLookUp(task.id))} data-testid="task-look-up">
                {detail.issue.checked_at_ms === null ? "Look up its title" : "Look it up again"}
              </Button>
            )}
          </div>
        </Section>
      )}
      {detail.sessions.length > 0 && (
        <Section title="Sessions" testId="task-sessions">
          <ul className="flex flex-col gap-1">
            {detail.sessions.map((s) => (
              <li key={s.session_id} className="flex items-center gap-2 text-meta">
                {s.live ? <PlaceMark place={OWN} profile={profileOf(s.session_id)} size={16} ring="var(--color-surface)" /> : <span className="size-4 shrink-0 rounded-full border border-hairline" aria-hidden="true" />}
                <span className="min-w-0 flex-1 truncate text-ink">
                  {s.name} <span className="text-muted">{HOW_WORDS[s.how] ?? s.how} · {ago(s.at_ms, now)}{s.live ? " · running" : ""}</span>
                </span>
                {s.live && (
                  <Button size="sm" variant="ghost" onClick={() => openSession(s.session_id)}>
                    Open<span className="sr-only"> {s.name}</span>
                  </Button>
                )}
              </li>
            ))}
          </ul>
        </Section>
      )}
      {detail.threads.length > 0 && (
        <Section title="Messages about it" testId="task-threads">
          <ul className="flex flex-col gap-1">
            {detail.threads.map((thread) => (
              <li key={thread.thread_id}>
                <button type="button" className="flex w-full items-baseline gap-2 rounded-lg px-2 py-1.5 text-left text-meta hover:bg-surface-2" onClick={() => (focusThread(thread.thread_id), showTab("messages"))}>
                  <span className="min-w-0 flex-1 truncate text-ink">{thread.subject}</span>
                  <span className="shrink-0 text-muted">
                    {thread.count === 1 ? "1 message" : `${thread.count} messages`} · {ago(thread.at_ms, now)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </Section>
      )}
      <Section title="History" testId="task-history">
        <ol className="flex flex-col gap-1">
          {detail.history.map((event) => (
            <li key={event.id} className="flex gap-2 text-meta">
              <span className="w-16 shrink-0 text-muted">{ago(event.at_ms, now)}</span>
              <span className="min-w-0 flex-1 text-ink">
                <span className="font-medium">{event.who}</span> {event.what}
              </span>
            </li>
          ))}
        </ol>
      </Section>
      {error && (
        <p role="alert" className="text-meta text-danger">
          {error}
        </p>
      )}
    </article>
  );
}
