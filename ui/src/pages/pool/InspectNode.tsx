/**
 * The inspector on one member: what waits on you about it (its tool's
 * own asks with the tool's options; its agent's asks with the pool's
 * three; its limit), what it says it's doing (marked as its own words),
 * its tasks, who it talks to, its usage, and the rules it works under.
 */
import { useEffect, useState, type JSX } from "react";

import { AskCard } from "../../components/AskCard";
import { Button } from "../../components/primitives";
import { ToolMark } from "../../components/ToolMark";
import { useActionHost } from "../../components/ItemMenu";
import { formatTokens } from "../../components/indicator";
import { bytesWords } from "../../lib/machine";
import { useOpenAsks, type AskInfo } from "../../lib/ipc/runtime";
import { onTasksChanged, tasksOfSession, type TaskInfo } from "../../lib/ipc/tasks";
import type { CanvasEdgeInfo, NodeAskInfo } from "../../lib/ipc/pool-canvas";
import type { PoolInfo } from "../../lib/ipc/pools";
import { sessionState } from "../../lib/rail-items";
import { TaskRow } from "../tasks/TaskRow";
import { answerAsk, sessionAlert } from "./CanvasNode";
import { edgeWords } from "./CanvasEdges";
import { Section } from "./InspectPool";
import { makeLead, nameOf } from "./actions";
import type { MemberNode } from "./useCanvas";
import { edgeKey } from "./useCanvas";

const TASKS_SHOWN = 5;

/** An agent's ask as the approval card shows it. */
function asCard(ask: NodeAskInfo, sessionId: string): AskInfo {
  return {
    id: ask.id,
    session_id: sessionId,
    kind: "approval",
    approval_kind: null,
    summary: ask.title,
    reason: ask.source === "task" ? "A change to a task, above what this pool lets agents do." : "A message, above what this pool lets agents do.",
    options: [
      { id: "once", label: "Allow once", effect: "allow_once", rule: null },
      ...(ask.offer_always ? [{ id: "always", label: "Always allow in this pool", effect: "allow_rule", rule: null }] : []),
      { id: "deny", label: "Deny", effect: "deny", rule: null },
    ],
    parts: [],
    asked_at_ms: ask.at_ms,
  };
}

function useMemberTasks(sessionId: string): TaskInfo[] | null {
  const [tasks, setTasks] = useState<TaskInfo[] | null>(null);
  useEffect(() => {
    let alive = true;
    setTasks(null);
    const read = () => void tasksOfSession(sessionId, { cursor: null, limit: TASKS_SHOWN }).then((page) => alive && setTasks(page.items), () => alive && setTasks([]));
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [sessionId]);
  return tasks;
}

export function InspectNode({
  pool,
  member,
  members,
  asks,
  edges,
  names,
  now,
  onSelectEdge,
  onSelectPool,
  onOpenTask,
}: {
  pool: PoolInfo;
  member: MemberNode;
  members: MemberNode[];
  asks: NodeAskInfo[];
  edges: CanvasEdgeInfo[];
  names: ReadonlyMap<string, string>;
  now: number;
  onSelectEdge: (key: string) => void;
  onSelectPool: () => void;
  onOpenTask: (task: TaskInfo) => void;
}): JSX.Element {
  const host = useActionHost();
  const toolAsks = useOpenAsks().filter((ask) => ask.session_id === member.id);
  const tasks = useMemberTasks(member.id);
  const state = sessionState(member.session);
  const alert = sessionAlert(member);
  const name = nameOf(member.session);
  const mine = edges.filter((edge) => edge.from_session_id === member.id || edge.to_session_id === member.id);
  const tokens = member.session.tokens;
  return (
    <div className="flex flex-col" data-testid="inspect-node">
      <div className="flex flex-col gap-2 px-4 pb-3 pt-1">
        <div className="flex items-center gap-2">
          <span className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-surface-2">
            <ToolMark tool={member.session.provider} size={16} />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-section text-ink" title={name}>
              {name}
            </span>
            <span className="truncate text-label text-muted">{[member.role === "lead" ? "Lead" : member.role === "observer" ? "Observer" : null, member.card.account, member.card.tool].filter(Boolean).join(" · ")}</span>
          </span>
        </div>
        <span className={"text-meta " + state.tone}>{state.word}</span>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => host.openSessionById?.(member.id)} disabled={!host.openSessionById} data-testid="inspect-node-open">
            Open session
          </Button>
          {member.role !== "lead" && (
            <Button size="sm" variant="secondary" onClick={() => void makeLead(member, members)} data-testid="inspect-node-lead">
              Make lead
            </Button>
          )}
        </div>
      </div>

      {(toolAsks.length > 0 || asks.length > 0 || alert) && (
        <Section title="Waiting on you" testId="inspect-node-waiting">
          {alert && !toolAsks.length && <p className={"text-meta " + (alert.tone === "danger" ? "text-danger" : "text-warning")}>{alert.word}</p>}
          {toolAsks.map((ask) => (
            <AskCard key={ask.id} ask={ask} kindLabel="Its tool asks" />
          ))}
          {asks.map((ask) => (
            <AskCard key={ask.id} ask={asCard(ask, member.id)} kindLabel="Its agent asks" answer={(option) => answerAsk(ask, option as "once" | "always" | "deny")} />
          ))}
        </Section>
      )}

      <Section title="What it's doing">
        {member.card.said.value ? (
          <p className="text-body text-ink">
            “{member.card.said.value}” <span className="text-label text-faint">said by the agent</span>
          </p>
        ) : (
          <p className="text-label text-faint">{member.card.said.unavailable ?? "Unavailable"}</p>
        )}
        {member.card.files.length > 0 && (
          <ul className="flex flex-col gap-0.5 font-mono text-label text-muted" aria-label="Files it changed">
            {member.card.files.map((file) => (
              <li key={file} className="truncate">
                {file}
              </li>
            ))}
          </ul>
        )}
      </Section>

      <Section title="Tasks" testId="inspect-node-tasks">
        {tasks === null ? (
          <span className="h-10 rounded-lg bg-surface-3 motion-safe:animate-pulse" role="status" aria-label="Reading its tasks" />
        ) : tasks.length === 0 ? (
          <p className="text-label text-muted">It hasn't held or worked a task here.</p>
        ) : (
          <ul className="-mx-2 flex flex-col">
            {tasks.map((task) => (
              <li key={task.id}>
                <TaskRow task={task} selected={false} now={now} onOpen={() => onOpenTask(task)} />
              </li>
            ))}
          </ul>
        )}
      </Section>

      <Section title="Talks to" testId="inspect-node-edges">
        {mine.length === 0 ? (
          <p className="text-label text-muted">No messages or hand-offs with the others this week.</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {mine.map((edge) => {
              const out = edge.from_session_id === member.id;
              const other = names.get(out ? edge.to_session_id : edge.from_session_id) ?? "A member";
              const words = edgeWords(edge, names.get(edge.from_session_id) ?? "", names.get(edge.to_session_id) ?? "");
              return (
                <li key={edgeKey(edge.from_session_id, edge.to_session_id)}>
                  <button type="button" onClick={() => onSelectEdge(edgeKey(edge.from_session_id, edge.to_session_id))} className="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60">
                    <span className="min-w-0 flex-1 truncate text-meta text-ink">
                      {out ? "To" : "From"} {other}
                    </span>
                    <span className="shrink-0 text-label text-muted">{words.count}</span>
                    {edge.pending.length > 0 && <span className="size-2 shrink-0 rounded-full bg-warning" aria-label="something waits" />}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </Section>

      <Section title="Usage">
        <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-meta">
          <dt className="text-muted">Tokens</dt>
          <dd className="text-ink">{tokens ? `${formatTokens(tokens.input + tokens.cache_read + tokens.cache_write)} in · ${formatTokens(tokens.output)} out` : <span className="text-faint">Unavailable — not measured yet</span>}</dd>
          <dt className="text-muted">Memory</dt>
          <dd className="text-ink">{member.session.rss_bytes === null ? <span className="text-faint">Unavailable — not running here</span> : bytesWords(member.session.rss_bytes)}</dd>
        </dl>
      </Section>

      <Section title="Rules">
        <p className="text-label text-muted">
          It works under the pool's rules: {pool.rules.length === 0 ? "nothing always allowed yet" : `${pool.rules.length} always allowed`}.{" "}
          <button type="button" onClick={onSelectPool} className="rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
            See the pool's settings
          </button>
        </p>
      </Section>
    </div>
  );
}
