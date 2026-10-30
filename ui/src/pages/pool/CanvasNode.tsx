/**
 * One member on a pool's canvas: its tool, its name, its account, its
 * state in the rail's one vocabulary (AJ-E2104), what it holds, and the
 * lead marked. What waits on you about it sits on it with its answer in
 * place (D-28's three choices), and a badge counts it.
 *
 * The node's name is its one tab stop (the canvas moves between names
 * with the arrow keys); its answers and its ⋯ follow it.
 *
 * Semantic zoom (the 0.3 review): this card from 100 % up, where none of
 * its words is under 12 px; CompactNode below, the same box with its
 * words at screen size.
 */
import { memo, type JSX, type MouseEvent } from "react";

import { ToolMark } from "../../components/ToolMark";
import { IconLead, IconMore } from "../../components/icons-canvas";
import { sessionState, STATE_TONE } from "../../lib/rail-items";
import { NODE_H, NODE_W, nodeForm } from "./layout";
import { CompactNode } from "./CompactNode";
import { nameOf } from "./actions";
import { AskLine, NO_TASK, nodeBorder, sessionAlert, type NodeProps } from "./NodeParts";

/** Its full card from 100 % up, its compact form below (semantic zoom). */
export const CanvasNode = memo(function CanvasNode(props: NodeProps): JSX.Element {
  return nodeForm(props.scale).full ? <FullNode {...props} /> : <CompactNode {...props} />;
});

function FullNode({ member, at, asks, held, selected, tabStop, dragging, onSelect, onMenu, onPointerDown, onKey }: NodeProps): JSX.Element {
  const state = sessionState(member.session);
  const alert = sessionAlert(member);
  const lead = member.role === "lead";
  const name = nameOf(member.session);
  const waits = asks.length + held + (alert ? 1 : 0);
  // The card says "None" when it holds no task (cards.rs's NO_TASK).
  const task = member.card.task.value === NO_TASK ? null : member.card.task.value;
  const account = member.card.account;
  const ended = state.state === "ended";
  const openMenu = (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const box = event.currentTarget.getBoundingClientRect();
    onMenu(member.id, event.detail === 0 || event.type === "click" ? { x: box.left, y: box.bottom + 4 } : { x: event.clientX, y: event.clientY });
  };
  const border = nodeBorder(selected, waits > 0, lead);
  return (
    <div
      role="group"
      aria-label={`${name}${lead ? ", lead" : ""}`}
      data-testid="canvas-node"
      data-node={member.id}
      data-lead={lead || undefined}
      data-form="full"
      onPointerDown={(event) => onPointerDown(member.id, event)}
      onContextMenu={openMenu}
      style={{ transform: `translate(${at.x}px, ${at.y}px)`, width: NODE_W, minHeight: NODE_H }}
      className={
        "absolute left-0 top-0 flex touch-none select-none flex-col gap-1.5 rounded-xl border bg-surface p-2.5 shadow-card motion-safe:animate-[aij-fade-in_140ms_ease-out] " +
        border +
        (dragging ? " z-20 cursor-grabbing opacity-90 shadow-lg" : " cursor-grab") +
        (ended && !selected ? " opacity-75" : "")
      }
    >
      {waits > 0 && (
        <span className="absolute -right-2 -top-2 rounded-full bg-warning px-1.5 font-mono text-label font-semibold leading-5 text-canvas" aria-hidden="true" data-testid="node-badge">
          {waits}
        </span>
      )}
      <div className="flex min-w-0 items-center gap-2">
        <span className="flex size-6 shrink-0 items-center justify-center rounded-md bg-surface-2">
          <ToolMark tool={member.session.provider} size={14} />
        </span>
        <button
          type="button"
          tabIndex={tabStop ? 0 : -1}
          data-node-name={member.id}
          aria-pressed={selected}
          aria-keyshortcuts="Enter Delete ArrowUp ArrowDown ArrowLeft ArrowRight"
          aria-description={`${state.word}${waits > 0 ? `, ${waits} waiting on you` : ""}`}
          onClick={(event) => onSelect(member.id, event.detail === 0)}
          onKeyDown={(event) => onKey(member.id, event)}
          className="min-w-0 flex-1 truncate rounded text-left text-sm font-semibold text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
          title={name}
        >
          {name}
        </button>
        <button
          type="button"
          onClick={openMenu}
          aria-label={`More for ${name}`}
          aria-haspopup="menu"
          className="flex size-6 shrink-0 items-center justify-center rounded-md text-faint outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="node-more"
        >
          <IconMore width={16} height={16} />
        </button>
      </div>
      <span className="flex min-w-0 items-center gap-1.5 truncate text-label text-muted">
        {lead && (
          <span className="inline-flex shrink-0 items-center gap-1 rounded-md bg-accent-soft px-1.5 font-medium text-accent-hi" data-testid="node-lead">
            <IconLead width={12} height={12} aria-hidden="true" />
            Lead
          </span>
        )}
        {member.role === "observer" && <span className="shrink-0 rounded-md bg-surface-2 px-1.5">Observer</span>}
        <span className="truncate">{[account, member.card.tool].filter(Boolean).join(" · ")}</span>
      </span>
      <span className="flex min-w-0 items-center gap-1.5 text-label" data-testid="node-state">
        <span aria-hidden="true" className={"size-2 shrink-0 rounded-full " + (alert?.tone === "danger" ? "bg-danger" : STATE_TONE[state.state].dot) + (state.state === "working" ? " shadow-[0_0_0_3px_var(--color-live-soft)]" : "")} />
        <span className={"shrink-0 " + (alert?.tone === "danger" ? "text-danger" : state.tone)}>{alert?.word ?? state.word}</span>
        {task && (
          <span className="min-w-0 truncate text-muted" title={task}>
            · {task}
          </span>
        )}
        {alert && (
          <button type="button" onClick={() => onSelect(member.id, true)} className="ml-auto shrink-0 rounded px-1 font-medium text-ink underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="node-alert">
            {member.session.status === "waiting_user" ? "Answer" : "See"}
            <span className="sr-only"> {name}</span>
          </button>
        )}
      </span>
      {asks.slice(0, 1).map((ask) => (
        <AskLine key={ask.id} ask={ask} />
      ))}
      {asks.length > 1 && (
        <button type="button" onClick={() => onSelect(member.id, true)} className="self-start rounded text-label text-warning outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
          {asks.length - 1} more {asks.length === 2 ? "ask" : "asks"}
        </button>
      )}
    </div>
  );
}
