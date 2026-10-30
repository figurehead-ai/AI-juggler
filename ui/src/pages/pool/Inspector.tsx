/**
 * The canvas's inspector (right): it follows the selection — the pool's
 * settings, one member, or one edge (the conversation between the two,
 * with what waits on it) — and, with nothing selected, the pool's live
 * activity, newest first.
 */
import { forwardRef, type JSX, type ReactNode } from "react";

import type { ProjectInfo } from "../../lib/bridge";
import type { PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import type { PoolInfo } from "../../lib/ipc/pools";
import type { TaskInfo } from "../../lib/ipc/tasks";
import { levelLabel } from "../../lib/pools";
import { agoWords } from "../../lib/time-words";
import { lazyNamed } from "../../lib/lazy";
import { PendingChip, edgeWords } from "./CanvasEdges";
import { InspectNode } from "./InspectNode";
import { InspectPool, Section, deliveryWords } from "./InspectPool";
import type { Selection } from "./Canvas";
import { edgeKey, type MemberNode } from "./useCanvas";

// The conversation loads only when an edge is opened.
const ThreadView = lazyNamed(() => import("../messages/ThreadView"), "ThreadView");

function Fact({ label, children }: { label: string; children: ReactNode }): JSX.Element {
  return (
    <div className="flex items-baseline justify-between gap-3 text-meta">
      <span className="text-muted">{label}</span>
      <span className="min-w-0 truncate text-right text-ink">{children}</span>
    </div>
  );
}

function Activity({ pool, canvas, members, now, onSelect }: { pool: PoolInfo; canvas: PoolCanvasInfo | null; members: MemberNode[]; now: number; onSelect: (selection: Selection) => void }): JSX.Element {
  const lead = members.find((member) => member.role === "lead");
  const asks = canvas?.asks.length ?? 0;
  return (
    <div className="flex flex-col" data-testid="inspect-activity">
      <Section title="This pool">
        <Fact label="Members">
          {pool.members}
          {lead ? ` · lead ${lead.card.name}` : ""}
        </Fact>
        <Fact label="Agents may">{levelLabel(pool.access.level)}</Fact>
        <Fact label="Delivery">{`${deliveryWords(pool)}, ${pool.access.hop_limit} in a row`}</Fact>
        <Fact label="Always allowed">{pool.rules.length === 0 ? "Nothing yet" : `${pool.rules.length} ${pool.rules.length === 1 ? "rule" : "rules"}`}</Fact>
        {asks > 0 && <Fact label="Waiting on you">{`${asks} ${asks === 1 ? "ask" : "asks"}`}</Fact>}
        <button type="button" onClick={() => onSelect({ kind: "pool" })} className="self-start rounded text-meta text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="inspect-open-settings">
          Change what agents may do…
        </button>
      </Section>
      <Section title="Live" testId="inspect-stream">
        {canvas === null ? (
          <span className="h-16 rounded-lg bg-surface-3 motion-safe:animate-pulse" role="status" aria-label="Reading the pool's activity" />
        ) : canvas.stream.length === 0 ? (
          <p className="text-label text-muted">Nothing yet. Messages and task changes between its agents show here as they happen.</p>
        ) : (
          <ol className="flex flex-col gap-2" aria-live="polite" aria-relevant="additions">
            {canvas.stream.map((line, index) => (
              <li key={`${line.at_ms}-${index}`} className="flex gap-2 text-meta">
                <span className="w-[4.75rem] shrink-0 whitespace-nowrap text-label tabular-nums text-faint">{agoWords(line.at_ms, now)}</span>
                {line.session_id ? (
                  <button
                    type="button"
                    onClick={() => onSelect(line.other_session_id ? { kind: "edge", key: edgeKey(line.session_id!, line.other_session_id) } : { kind: "node", id: line.session_id! })}
                    className="min-w-0 rounded text-left text-ink outline-none hover:text-accent-hi focus-visible:ring-1 focus-visible:ring-accent/60"
                  >
                    {line.words}
                  </button>
                ) : (
                  <span className="min-w-0 text-ink">{line.words}</span>
                )}
              </li>
            ))}
          </ol>
        )}
        <p className="text-label text-faint">Select a member or an edge and this panel shows its tasks, messages, usage and rules.</p>
      </Section>
    </div>
  );
}

function InspectEdge({ pool, canvas, edgeId, names, now }: { pool: PoolInfo; canvas: PoolCanvasInfo | null; edgeId: string; names: ReadonlyMap<string, string>; now: number }): JSX.Element {
  const edge = canvas?.edges.find((row) => edgeKey(row.from_session_id, row.to_session_id) === edgeId);
  if (!edge) return <p className="px-4 py-3 text-meta text-muted">Nothing passes between these two this week.</p>;
  const from = names.get(edge.from_session_id) ?? "A member";
  const to = names.get(edge.to_session_id) ?? "a member";
  const members = [
    { session_id: edge.from_session_id, name: from },
    { session_id: edge.to_session_id, name: to },
  ];
  return (
    <div className="flex flex-col" data-testid="inspect-edge">
      <div className="flex flex-col gap-1 px-4 pb-3 pt-1">
        <span className="text-section text-ink">
          {from} → {to}
        </span>
        <span className="text-meta text-muted">{edgeWords(edge, from, to).count} this week · last {agoWords(edge.last_at_ms, now)}</span>
      </div>
      {edge.pending.length > 0 && (
        <Section title="Waiting on you">
          <ul className="flex flex-col gap-1.5">
            {edge.pending.map((pending) => (
              <li key={pending.message_id} className="flex flex-col gap-1 rounded-lg border border-hairline/70 px-2.5 py-2">
                <span className="truncate text-meta text-ink">«{pending.subject}»</span>
                <span className="text-label text-muted">
                  {pending.kind === "hop_limit" ? "Paused: its chain reached the hop limit." : pending.kind === "asked" ? `Above what the pool allows: answer it on ${from}.` : `Waits for ${to} to read it, or for your Deliver.`}
                </span>
              </li>
            ))}
          </ul>
          <PendingChip edge={edge} />
        </Section>
      )}
      <Section title="The conversation">
        {edge.thread_id ? (
          <div className="-mx-2 min-w-0" data-canvas-scroll>
            <ThreadView threadId={edge.thread_id} poolId={pool.id} members={members} now={now} />
          </div>
        ) : (
          <p className="text-label text-muted">Only task hand-offs so far: {from} gave {to} {edge.tasks === 1 ? "a task" : `${edge.tasks} tasks`}.</p>
        )}
      </Section>
    </div>
  );
}

export const Inspector = forwardRef<HTMLHeadingElement, {
  pool: PoolInfo;
  project: ProjectInfo | undefined;
  canvas: PoolCanvasInfo | null;
  members: MemberNode[];
  names: ReadonlyMap<string, string>;
  selection: Selection;
  now: number;
  onSelect: (selection: Selection) => void;
  onOpenTask: (task: TaskInfo) => void;
  onDeleted: () => void;
}>(function Inspector({ pool, project, canvas, members, names, selection, now, onSelect, onOpenTask, onDeleted }, heading): JSX.Element {
  const member = selection.kind === "node" ? members.find((row) => row.id === selection.id) : undefined;
  const title = selection.kind === "pool" ? "Pool settings" : member ? "Member" : selection.kind === "edge" ? "Conversation" : "Activity";
  return (
    <aside aria-labelledby="pool-inspector-title" className="flex h-full min-h-0 w-[340px] shrink-0 flex-col border-l border-hairline bg-surface" data-testid="pool-inspector">
      <div className="flex h-11 shrink-0 items-center gap-2 px-4">
        <h3 id="pool-inspector-title" ref={heading} tabIndex={-1} className="flex-1 text-label font-semibold uppercase tracking-[0.08em] text-faint outline-none">
          {title}
        </h3>
        {selection.kind !== "none" && (
          <button type="button" onClick={() => onSelect({ kind: "none" })} className="rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="inspector-back">
            Activity
          </button>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto pb-6" data-canvas-scroll>
        {selection.kind === "pool" ? (
          <InspectPool pool={pool} project={project} now={now} onDeleted={onDeleted} />
        ) : member ? (
          <InspectNode
            pool={pool}
            member={member}
            members={members}
            asks={(canvas?.asks ?? []).filter((ask) => ask.session_id === member.id)}
            edges={canvas?.edges ?? []}
            names={names}
            now={now}
            onSelectEdge={(key) => onSelect({ kind: "edge", key })}
            onSelectPool={() => onSelect({ kind: "pool" })}
            onOpenTask={onOpenTask}
          />
        ) : selection.kind === "edge" ? (
          <InspectEdge pool={pool} canvas={canvas} edgeId={selection.key} names={names} now={now} />
        ) : (
          <Activity pool={pool} canvas={canvas} members={members} now={now} onSelect={onSelect} />
        )}
      </div>
    </aside>
  );
});
