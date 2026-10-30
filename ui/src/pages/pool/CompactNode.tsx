/**
 * A member on a pool's canvas below 100 % (semantic zoom, the 0.3
 * review): the same box in the same place as its full card, its words
 * drawn at screen size (12 px and up) — as many lines as the box has room
 * for: name, account and tool, state and task; or name and state; or the
 * name, its state the dot. What waits on it is a badge you press to
 * answer in place (an agent's ask: Allow once, Always allow here, Deny),
 * the full card's answers in a small panel under it.
 *
 * Keyboard and screen readers: the name is the node's one tab stop, with
 * the full card's name, description and keys; the badge follows it.
 */
import { useEffect, useRef, useState, type JSX, type MouseEvent } from "react";

import { ToolMark } from "../../components/ToolMark";
import { IconLead, IconMore } from "../../components/icons-canvas";
import { sessionState, STATE_TONE } from "../../lib/rail-items";
import { NODE_H, NODE_W, nodeForm } from "./layout";
import { nameOf } from "./actions";
import { AskLine, NO_TASK, nodeBorder, sessionAlert, type NodeProps } from "./NodeParts";

export function CompactNode({ member, at, asks, held, selected, tabStop, dragging, scale, onSelect, onMenu, onPointerDown, onKey }: NodeProps): JSX.Element {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  const badge = useRef<HTMLButtonElement | null>(null);
  const panel = useRef<HTMLDivElement | null>(null);
  const state = sessionState(member.session);
  const alert = sessionAlert(member);
  const lead = member.role === "lead";
  const name = nameOf(member.session);
  const waits = asks.length + held + (alert ? 1 : 0);
  const task = member.card.task.value === NO_TASK ? null : member.card.task.value;
  const ended = state.state === "ended";
  const form = nodeForm(scale);
  const width = NODE_W * scale;
  const height = NODE_H * scale;
  const lines = form.full ? 3 : form.lines;
  // Room for the name on two lines above its state (a long name read
  // whole at fit, not cut at "Checkout cop…").
  const wrap = lines === 2 && height >= 62;

  useEffect(() => {
    if (!open) return;
    panel.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const away = (event: PointerEvent) => {
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", away);
    return () => window.removeEventListener("pointerdown", away);
  }, [open]);
  // Nothing left to answer: the panel goes, the keyboard back on the node.
  useEffect(() => {
    if (open && waits === 0) setOpen(false);
  }, [open, waits]);

  const openMenu = (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const rect = event.currentTarget.getBoundingClientRect();
    onMenu(member.id, event.detail === 0 || event.type === "click" ? { x: rect.left, y: rect.bottom + 4 } : { x: event.clientX, y: event.clientY });
  };
  const dot = (
    <span
      aria-hidden="true"
      className={"size-2 shrink-0 rounded-full " + (alert?.tone === "danger" ? "bg-danger" : STATE_TONE[state.state].dot) + (state.state === "working" ? " shadow-[0_0_0_3px_var(--color-live-soft)]" : "")}
    />
  );
  const word = <span className={"shrink-0 " + (alert?.tone === "danger" ? "text-danger" : state.tone)}>{alert?.word ?? state.word}</span>;
  const nameButton = (
    <button
      type="button"
      tabIndex={tabStop ? 0 : -1}
      data-node-name={member.id}
      aria-pressed={selected}
      aria-keyshortcuts="Enter Delete ArrowUp ArrowDown ArrowLeft ArrowRight"
      aria-description={`${state.word}${waits > 0 ? `, ${waits} waiting on you` : ""}`}
      onClick={(event) => onSelect(member.id, event.detail === 0)}
      onKeyDown={(event) => onKey(member.id, event)}
      className={
        "min-w-0 flex-1 rounded text-left font-semibold text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " +
        (wrap ? "line-clamp-2 break-words leading-tight " : "truncate ") +
        (lines === 3 ? "text-body" : lines === 2 ? "text-meta" : "text-label")
      }
      title={name}
    >
      {name}
    </button>
  );
  // Its ⋯ shows on hover, on focus and when chosen, over the name's end:
  // at rest the name has the node's width (right-click and Shift+F10
  // open the same menu).
  const more = (
    <button
      type="button"
      onClick={openMenu}
      aria-label={`More for ${name}`}
      aria-haspopup="menu"
      className={
        "absolute right-1 top-1/2 flex size-6 -translate-y-1/2 items-center justify-center rounded-md bg-surface text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:opacity-100 focus-visible:ring-1 focus-visible:ring-accent/60 group-hover/node:opacity-100 " +
        (selected ? "opacity-100" : "opacity-0")
      }
      data-testid="node-more"
    >
      <IconMore width={16} height={16} />
    </button>
  );
  return (
    <div
      ref={box}
      role="group"
      aria-label={`${name}${lead ? ", lead" : ""}`}
      data-testid="canvas-node"
      data-node={member.id}
      data-lead={lead || undefined}
      data-form="compact"
      onPointerDown={(event) => onPointerDown(member.id, event)}
      onContextMenu={openMenu}
      style={{ transform: `translate(${at.x}px, ${at.y}px)`, width: NODE_W, height: NODE_H }}
      className={"absolute left-0 top-0 touch-none select-none " + (open ? "z-30" : dragging ? "z-20" : "")}
    >
      {/* Undo the canvas's scale: what's in here is drawn at screen size. */}
      <div
        style={{ transform: `scale(${1 / scale})`, transformOrigin: "0 0", width, height }}
        className={
          "group/node relative flex flex-col justify-center gap-1 rounded-lg border bg-surface px-2 py-1 shadow-card motion-safe:animate-[aij-fade-in_140ms_ease-out] " +
          nodeBorder(selected, waits > 0, lead) +
          (dragging ? " cursor-grabbing opacity-90 shadow-lg" : " cursor-grab") +
          (ended && !selected ? " opacity-75" : "")
        }
      >
        <div className={"flex min-w-0 gap-1.5 " + (wrap ? "items-start" : "items-center")}>
          {lines === 3 ? (
            <span className="flex size-5 shrink-0 items-center justify-center rounded bg-surface-2">
              <ToolMark tool={member.session.provider} size={12} />
            </span>
          ) : wrap ? (
            // On the first line of a name that wraps.
            <span className="flex h-4 shrink-0 items-center">{dot}</span>
          ) : (
            dot
          )}
          {lines === 1 && lead && <IconLead width={12} height={12} aria-hidden="true" className="shrink-0 text-accent-hi" />}
          {nameButton}
        </div>
        {more}
        {lines === 3 && (
          <span className="flex min-w-0 items-center gap-1.5 truncate text-label text-muted">
            {lead && (
              <span className="inline-flex shrink-0 items-center gap-1 rounded bg-accent-soft px-1 font-medium text-accent-hi" data-testid="node-lead">
                <IconLead width={12} height={12} aria-hidden="true" />
                Lead
              </span>
            )}
            {member.role === "observer" && <span className="shrink-0 rounded bg-surface-2 px-1">Observer</span>}
            <span className="truncate">{[member.card.account, member.card.tool].filter(Boolean).join(" · ")}</span>
          </span>
        )}
        {lines === 3 && (
          <span className="flex min-w-0 items-center gap-1.5 text-label" data-testid="node-state">
            {dot}
            {word}
            {task && (
              <span className="min-w-0 truncate text-muted" title={task}>
                · {task}
              </span>
            )}
          </span>
        )}
        {lines === 2 && (
          <span className="flex min-w-0 items-center gap-1 truncate text-label" data-testid="node-state">
            {lead && (
              <span className="inline-flex shrink-0 items-center gap-1 font-medium text-accent-hi" data-testid="node-lead">
                <IconLead width={12} height={12} aria-hidden="true" />
                Lead ·
              </span>
            )}
            {word}
          </span>
        )}
        {waits > 0 && (
          <button
            ref={badge}
            type="button"
            aria-expanded={open}
            aria-haspopup="dialog"
            aria-label={`${waits} waiting on you about ${name}`}
            title="What waits on you"
            onClick={() => setOpen((now) => !now)}
            className="absolute -right-2 -top-2 flex h-5 min-w-5 items-center justify-center rounded-full bg-warning px-1.5 font-mono text-label font-semibold text-canvas shadow-card outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="node-badge"
          >
            {waits}
          </button>
        )}
        {open && (
          <div
            ref={panel}
            role="dialog"
            aria-label={`What waits on you about ${name}`}
            data-node-popover
            onKeyDown={(event) => {
              if (event.key !== "Escape") return;
              event.preventDefault();
              event.stopPropagation();
              setOpen(false);
              badge.current?.focus();
            }}
            className="absolute left-0 top-full z-40 mt-2 flex w-72 cursor-auto flex-col gap-2 rounded-xl border border-hairline bg-surface p-2.5 text-left shadow-lg"
            data-testid="node-popover"
          >
            {alert && (
              <span className="flex items-center gap-2 text-meta">
                {dot}
                <span className={"flex-1 " + (alert.tone === "danger" ? "text-danger" : "text-warning")}>{alert.word}</span>
                <button type="button" onClick={() => onSelect(member.id, true)} className="rounded px-1 font-medium text-ink underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="node-alert">
                  {member.session.status === "waiting_user" ? "Answer" : "See"}
                  <span className="sr-only"> {name}</span>
                </button>
              </span>
            )}
            {asks.slice(0, 2).map((ask) => (
              <AskLine key={ask.id} ask={ask} />
            ))}
            {asks.length > 2 && (
              <button type="button" onClick={() => onSelect(member.id, true)} className="self-start rounded text-label text-warning outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
                {asks.length - 2} more in the inspector
              </button>
            )}
            {held > 0 && (
              <span className="text-label text-muted">
                {held === 1 ? "A message to it is" : `${held} messages to it are`} paused at the hop limit: answer on its edge.
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
