/**
 * One member on a pool's canvas: its tool, its name, its account, its
 * state in the rail's one vocabulary (AJ-E2104), what it holds, and the
 * lead marked. What waits on you about it sits on it with its answer in
 * place (D-28's three choices; a line paused into it at the hop limit,
 * Let it continue or Don't deliver): one mark a need, no badge besides
 * (the rc3 critique, calm). Its App's logo is muted at rest, in full
 * colour when it needs you or is chosen, hovered or focused.
 *
 * The node's name is its one tab stop (the canvas moves between names
 * with the arrow keys); its answers and its ⋯ follow it.
 *
 * Semantic zoom (the 0.3 review): this card from 100 % up, where none of
 * its words is under 12 px; CompactNode below, the same box with its
 * words at screen size.
 */
import { memo, type JSX, type MouseEvent } from "react";
import { appName } from "../../lib/app-name";
import { appTone } from "../../lib/attention";

import { logoOf, MemberLogo } from "../../components/MemberLogo";
import { useActionHost } from "../../components/ItemMenu";
import { ShellOnly } from "../../components/ShellOnly";
import { IconMore } from "../../components/icons-canvas";
import { sessionState, STATE_TONE } from "../../lib/rail-items";
import { NODE_H, NODE_W, nodeForm } from "./layout";
import { CompactNode } from "./CompactNode";
import { NodeButton, RoleWords, titleOf } from "./NodeButton";
import { AskLine, NO_TASK, NodeRoom, PausedLine, nameWraps, nodeBorder, nodeNeed, sessionAlert, useNodeDescription, type NodeProps } from "./NodeParts";
import { edgeKey } from "./useCanvas";

/** Its full card from 100 % up, its compact form below (semantic zoom).
 * It never fades in: WebKit holds a hidden window's animations at their
 * first frame, and a fade's first frame is invisible — the rc2 walk's
 * nodes undrawn after Diagram → List → Diagram, and a pool's first fit
 * (100 % → its fit swaps every node's form) drew them faint until the
 * window showed again. */
export const CanvasNode = memo(function CanvasNode(props: NodeProps): JSX.Element {
  return nodeForm(props.scale).full ? <FullNode {...props} /> : <CompactNode {...props} />;
});

function FullNode({ member, at, asks, paused, hops, selected, tabStop, dragging, onSelect, onMenu, onPointerDown, onKey }: NodeProps): JSX.Element {
  const state = sessionState(member.session);
  const alert = sessionAlert(member);
  const lead = member.role === "lead";
  const title = titleOf(member);
  const waits = nodeNeed(member, asks, paused)?.waits ?? 0;
  const description = useNodeDescription(member, state.word, waits);
  // The card says "None" when it holds no task (cards.rs's NO_TASK); a
  // task that is the node's title isn't said twice.
  const task = member.card.task.value === NO_TASK || member.card.task.value === title.title ? null : member.card.task.value;
  const account = member.card.account;
  const host = useActionHost();
  const ended = state.state === "ended";
  const openMenu = (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const box = event.currentTarget.getBoundingClientRect();
    onMenu(member.id, event.detail === 0 || event.type === "click" ? { x: box.left, y: box.bottom + 4 } : { x: event.clientX, y: event.clientY });
  };
  const border = nodeBorder(selected, waits > 0, lead);
  const app = member.card.tool ? appName(member.card.tool, account) : account;
  return (
    <div
      data-testid="canvas-node"
      data-node={member.id}
      data-lead={lead || undefined}
      data-selected={selected || undefined}
      aria-current={selected ? "true" : undefined}
      aria-busy={member.pending || undefined}
      data-pending={member.pending || undefined}
      data-form="full"
      onPointerDown={(event) => onPointerDown(member.id, event)}
      onContextMenu={openMenu}
      style={{ transform: `translate(${at.x}px, ${at.y}px)`, width: NODE_W, minHeight: NODE_H }}
      className={
        "tone-host absolute left-0 top-0 flex touch-none select-none flex-col gap-2 rounded-xl border p-3 data-[pending]:opacity-70 " +
        border +
        (dragging ? " z-20 cursor-grabbing opacity-90 shadow-lg" : " cursor-grab") +
        (ended && !selected ? " opacity-75" : "")
      }
    >
      <div className="flex min-w-0 items-start gap-2.5">
        {/* Its App's own icon, a CLI's with its terminal badge: a member's
            one logo (D-126), its role in words beside it. */}
        <span className={"flex shrink-0 " + appTone(selected || waits > 0)} data-testid="node-logo">
          <MemberLogo spec={logoOf(`session:${member.session.id}`, { apps: [], sessions: [member.session] })} size={32} />
        </span>
        <div className="flex min-w-0 flex-1 flex-col">
          {/* The node is this button (P5, AJ-E3304 L4: an accessibility
              press on the node did nothing): it covers the card, so a
              click, a press or Enter anywhere on it opens it. */}
          <NodeButton member={member} title={title} selected={selected} tabStop={tabStop} description={description} onSelect={onSelect} onKey={onKey} className={"text-body leading-snug " + (nameWraps(title.title, NODE_W - 96, 14) ? "line-clamp-2" : "truncate")} />
          <span className="relative z-10 flex min-h-6 min-w-0 items-center gap-1.5 truncate text-label text-muted">
            <RoleWords role={member.role} />
            <ShellOnly tool={member.session.provider} account={member.session.account_id} project={member.session.project_id} inRow />
            <NodeRoom profileId={member.session.account_id} />
            <span aria-hidden="true">·</span>
            {/* Its App's page, one click (design review #14), a whole 24 px
                target (WCAG 2.5.8: axe measured 70 × 16.8 over the node's
                own button, which leaves it no spacing). */}
            {host.openAccountById && member.session.account_id ? (
              <button type="button" onClick={() => member.session.account_id && host.openAccountById?.(member.session.account_id)} className="min-h-6 min-w-6 truncate rounded text-left outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="node-app">
                {app}
              </button>
            ) : (
              <span className="truncate">{app}</span>
            )}
          </span>
        </div>
        <button
          type="button"
          onClick={openMenu}
          aria-label={`More for ${member.name}`}
          aria-haspopup="menu"
          className="relative z-10 flex size-6 shrink-0 items-center justify-center rounded-md text-faint outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="node-more"
        >
          <IconMore width={16} height={16} />
        </button>
      </div>
      <span className="flex min-w-0 items-center gap-1.5 text-label" data-testid="node-state">
        <span aria-hidden="true" className={"size-2 shrink-0 rounded-full " + (alert?.tone === "danger" ? "bg-danger" : STATE_TONE[state.state].dot) + (state.state === "working" ? " shadow-[0_0_0_3px_var(--color-live-soft)]" : "")} />
        <span className={"shrink-0 " + (alert?.tone === "danger" ? "text-danger" : state.tone)}>{alert?.word ?? state.word}</span>
        {task && (
          <span className="min-w-0 truncate text-muted" title={task}>
            · {task}
          </span>
        )}
        {alert ? (
          <button type="button" onClick={() => onSelect(member.id, true)} className="relative z-10 ml-auto shrink-0 rounded px-1 font-medium text-ink underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="node-alert">
            {member.session.status === "waiting_user" ? "Answer" : "See"}
            <span className="sr-only"> {member.name}</span>
          </button>
        ) : (
          title.when && (
            <span className="ml-auto shrink-0 whitespace-nowrap text-faint" data-testid="node-when">
              {title.when}
            </span>
          )
        )}
      </span>
      {asks.slice(0, 1).map((ask) => (
        <AskLine key={ask.id} ask={ask} />
      ))}
      {asks.length > 1 && (
        <button type="button" onClick={() => onSelect(member.id, true)} className="relative z-10 self-start rounded text-label text-warning outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
          {asks.length - 1} more {asks.length === 2 ? "ask" : "asks"}
        </button>
      )}
      {paused.map((one) => (
        <PausedLine key={edgeKey(one.edge.from_session_id, one.edge.to_session_id)} paused={one} hops={hops} />
      ))}
    </div>
  );
}
