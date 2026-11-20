/**
 * The inspector on one member: what waits on you about it (its tool's
 * own asks with the tool's options; its agent's asks with the pool's
 * three; its limit), what it says it's doing (marked as its own words),
 * its tasks, who it talks to, its usage, and the rules it works under,
 * with its own level here (D-62 decision 12).
 */
import { useEffect, useState, type JSX } from "react";
import { appName } from "../../lib/app-name";

import { AskCard } from "../../components/AskCard";
import { Button } from "../../components/primitives";
import { ShellOnly, useShellOnly } from "../../components/ShellOnly";
import { useActionHost } from "../../components/ItemMenu";
import { formatTokens } from "../../components/indicator";
import { bytesWords } from "../../lib/machine";
import { useOpenAsks, type AskInfo } from "../../lib/ipc/runtime";
import { onTasksChanged, tasksOfSession, type TaskInfo } from "../../lib/ipc/tasks";
import type { CanvasEdgeInfo, NodeAskInfo } from "../../lib/ipc/pool-canvas";
import type { PoolInfo } from "../../lib/ipc/pools";
import { TaskRow } from "../tasks/TaskRow";
import { answerAsk, askChoices, sessionAlert } from "./NodeParts";
import type { HeldAnswer } from "../messages/words";
import { edgeWords } from "./CanvasEdges";
import { Section } from "./Section";
import { MemberRole } from "./MemberRole";
import { MemberLevel } from "./MemberLevel";
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
    options: askChoices(ask).map(({ id, label }) => ({ id, label, effect: id === "always" ? "allow_rule" : id === "deny" ? "deny" : "allow_once", rule: null })),
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
  const alert = sessionAlert(member);
  const mine = edges.filter((edge) => edge.from_session_id === member.id || edge.to_session_id === member.id);
  const tokens = member.session.tokens;
  const shell = useShellOnly(member.session.provider, member.session.account_id, member.session.project_id);
  return (
    <div className="flex flex-col" data-testid="inspect-node">
      <div className="flex flex-col gap-2 px-4 pb-3 pt-1">
        {/* AJ-E3012: its name, App, role and state are the inspector's
            title, subtitle and pill; here its App's page and its doors. */}
        <span className="flex flex-wrap items-center gap-2 text-meta text-muted">
          <button type="button" onClick={() => member.session.account_id && host.openAccountById?.(member.session.account_id)} disabled={!host.openAccountById || !member.session.account_id} className="min-w-0 truncate rounded text-left outline-none enabled:hover:text-ink enabled:hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="inspect-node-app">
            {member.card.tool ? appName(member.card.tool, member.card.account) : member.card.account}
          </button>
          <ShellOnly tool={member.session.provider} account={member.session.account_id} project={member.session.project_id} />
        </span>

        {shell && <p className="text-label text-muted">{shell}</p>}
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => host.openSessionById?.(member.id)} disabled={!host.openSessionById} data-testid="inspect-node-open">
            Open session
          </Button>
        </div>
        {/* SK-13: its role, Lead too, as its card's ⋯ sets it. */}
        <MemberRole pool={pool} member={member} members={members} testId="inspect-node-role" />
      </div>

      {(toolAsks.length > 0 || asks.length > 0 || alert) && (
        <Section title="Waiting on you" testId="inspect-node-waiting">
          {alert && !toolAsks.length && <p className={"text-meta " + (alert.tone === "danger" ? "text-danger" : "text-warning")}>{alert.word}</p>}
          {toolAsks.map((ask) => (
            <AskCard key={ask.id} ask={ask} kindLabel="Its tool asks" plain />
          ))}
          {asks.map((ask) => (
            <AskCard key={ask.id} ask={asCard(ask, member.id)} kindLabel="Its agent asks" answer={(option) => answerAsk(ask, option as HeldAnswer)} plain />
          ))}
        </Section>
      )}

      {/* Its own choice here, near the top (the rc2 walk P5: the walker
          didn't find it at the foot, under Usage). */}
      <MemberLevel pool={pool} sessionId={member.id} memberRole={member.role} />

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
          <span className="h-10 rounded-lg bg-skeleton motion-safe:animate-pulse" role="status" aria-label="Reading its tasks" />
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
