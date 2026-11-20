/**
 * A member on a pool's canvas below 100 % (semantic zoom, the 0.3
 * review): the same box in the same place as its full card, its words
 * drawn at screen size (12 px and up) — as many lines as the box has room
 * for: name, account and tool, state and task; or name and state; or the
 * name, its state the dot. What waits on you about it is its one mark,
 * its amber word in place of its state ("Asks you", "Limit reached",
 * "Paused"; "+1" when more wait), which you press to answer in place (an
 * agent's ask: Allow once, Always allow here, Deny; a paused line: Let it
 * continue, Don't deliver), the full card's answers in a small panel
 * beside it (the rc3 critique: calm, no badge on its corner).
 *
 * Keyboard and screen readers: the name is the node's one tab stop, with
 * the full card's name, description and keys; the mark follows it.
 */
import { useEffect, useRef, useState, type JSX, type MouseEvent } from "react";
import { useInertOutside } from "../../lib/inert";
import { appName } from "../../lib/app-name";
import { appTone } from "../../lib/attention";

import { AppIcon } from "../../components/AppIcon";
import { appOfSession } from "../../lib/app-of";
import { ShellOnly } from "../../components/ShellOnly";
import { IconLead, IconMore } from "../../components/icons-canvas";
import { sessionState, STATE_TONE } from "../../lib/rail-items";
import { NODE_H, NODE_W } from "./layout";
import { NodeButton, RoleWords, titleOf } from "./NodeButton";
import { AskLine, NO_TASK, NodeRoom, PausedLine, compactName, nodeBorder, nodeNeed, sessionAlert, useNodeDescription, type NodeProps } from "./NodeParts";
import { edgeKey } from "./useCanvas";

export function CompactNode({ member, at, asks, paused, hops, selected, tabStop, dragging, scale, onSelect, onMenu, onPointerDown, onKey }: NodeProps): JSX.Element {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  const badge = useRef<HTMLButtonElement | null>(null);
  const panel = useRef<HTMLDivElement | null>(null);
  // AJ-E3011 round 4: the page under its asks is inert while they're open.
  useInertOutside(panel, open);
  const state = sessionState(member.session);
  const alert = sessionAlert(member);
  const lead = member.role === "lead";
  const name = member.name;
  // D-126 §4a: what it leads with (its own name, else its task, else its
  // folder, told apart), when it started second.
  const title = titleOf(member);
  const need = nodeNeed(member, asks, paused);
  const waits = need?.waits ?? 0;
  const task = member.card.task.value === NO_TASK || member.card.task.value === title.title ? null : member.card.task.value;
  const ended = state.state === "ended";
  const description = useNodeDescription(member, state.word, waits);
  const { lines, wrap, tight, width, height, nameSize, wrapWords } = compactName(title.title, scale, lead, selected);

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
  // What waits on you leads: the dot in its tone, and its word in place
  // of the state's.
  const dot = (
    <span
      aria-hidden="true"
      className={"size-2 shrink-0 rounded-full " + (need ? (need.tone === "danger" ? "bg-danger" : "bg-warning") : STATE_TONE[state.state].dot) + (state.state === "working" && !need ? " shadow-[0_0_0_3px_var(--color-live-soft)]" : "")}
    />
  );
  // The node's one mark of what waits on you, a whole 24 px target (it
  // sits over the node's own button): its word, or (one line) its dot.
  const shown = need ? `${need.word}${waits > 1 ? ` +${waits - 1}` : ""}` : "";
  const needMark = need && (
    <button
      ref={badge}
      type="button"
      aria-expanded={open}
      aria-haspopup="dialog"
      aria-label={lines === 1 ? `${waits} waiting on you about ${name}` : `${shown}: ${waits} waiting on you about ${name}`}
      title={`${waits} waiting on you`}
      onClick={() => setOpen((now) => !now)}
      className={
        "relative z-10 -my-1 flex min-h-6 min-w-0 items-center rounded-md font-medium outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60 " +
        (need.tone === "danger" ? "text-danger" : "text-warning") +
        (lines === 1 ? " -mx-2 min-w-6 shrink-0 justify-center" : " shrink px-0.5")
      }
      data-testid="node-need"
    >
      {lines === 1 ? dot : <span className="truncate">{shown}</span>}
    </button>
  );
  const word = needMark || <span className={"shrink-0 " + state.tone}>{state.word}</span>;
  // The node is its button (P5): it covers the card, its ⋯ and badge above.
  const nameButton = (
    <NodeButton member={member} title={title} selected={selected} tabStop={tabStop} description={description} onSelect={onSelect} onKey={onKey} radius="after:rounded-lg" className={(wrapWords ? "" : "truncate ") + nameSize}>
      {/* The two-line clamp on a span: WebKit (the app's view) doesn't lay
          a button's own content out as a clamped box, and the rc12 walk saw
          a 30 % node's name run down ten lines past its box. */}
      {/* One line ends in an ellipsis inside a block span: a button's own
          text isn't cut with one where the row can't shrink it (the 38 %
          gallery fit read "Planne", its "r" under the edge). */}
      {wrapWords ? (
        <span className="line-clamp-2 leading-tight [overflow-wrap:normal]" data-node-name-text>
          {title.title}
        </span>
      ) : (
        <span className="block truncate" data-node-name-text>
          {title.title}
        </span>
      )}
    </NodeButton>
  );
  // Its ⋯ has its own slot at the name's end (design review #10: over the
  // name it hid "Checkout c…"); it shows on hover, on focus and when
  // chosen (right-click and Shift+F10 open the same menu).
  const more = (
    <button
      type="button"
      onClick={openMenu}
      aria-label={`More for ${name}`}
      aria-haspopup="menu"
      className={
        "relative z-10 flex h-5 shrink-0 items-center justify-center overflow-hidden rounded-md text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ml-0 focus-visible:w-5 focus-visible:opacity-100 focus-visible:ring-1 focus-visible:ring-accent/60 " +
        // Its slot takes room only while it shows: at rest the name has the
        // node's width (its gap too), so a long one reads whole at fit.
        // AJ-E3012: in a narrow box a selected node's name keeps its room
        // (the 38 % gallery fit read "Pl… a…"), and hovering doesn't take
        // it either (the name went to nothing under the pointer, and a
        // click on it missed); focus, right-click and ⇧F10 still open ⋯.
        (width >= 140 ? "group-hover/node:ml-0 group-hover/node:w-5 group-hover/node:opacity-100 " : "") +
        (selected && width >= 140 ? "w-5 opacity-100" : "-ml-1.5 w-0 opacity-0")
      }
      data-testid="node-more"
    >
      <IconMore width={14} height={14} />
    </button>
  );
  return (
    <div
      ref={box}
      data-testid="canvas-node"
      data-node={member.id}
      data-lead={lead || undefined}
      data-selected={selected || undefined}
      aria-current={selected ? "true" : undefined}
      aria-busy={member.pending || undefined}
      data-pending={member.pending || undefined}
      data-form="compact"
      onPointerDown={(event) => onPointerDown(member.id, event)}
      onContextMenu={openMenu}
      style={{ transform: `translate(${at.x}px, ${at.y}px)`, width: NODE_W, height: NODE_H }}
      className={"absolute left-0 top-0 touch-none select-none " + (open ? "z-30" : dragging ? "z-20" : "")}
    >
      {/* Undo the canvas's scale: what's in here is drawn at screen size. */}
      <div
        style={{ transform: `scale(${1 / scale})`, transformOrigin: "0 0", width, ...(lines === 2 && wrapWords ? { minHeight: height } : { height }) }}
        data-grows={(lines === 2 && wrapWords) || undefined}
        className={
          "tone-host group/node relative flex flex-col justify-center gap-1 rounded-lg border data-[pending]:opacity-70 " +
          (tight ? "px-1.5 py-0.5 " : "px-2 py-1 ") +
          nodeBorder(selected, waits > 0, lead) +
          (dragging ? " cursor-grabbing opacity-90 shadow-lg" : " cursor-grab") +
          (ended && !selected ? " opacity-75" : "")
        }
      >
        <div className={"flex min-w-0 gap-1.5 " + (wrap ? "items-start" : "items-center")}>
          {lines >= 2 ? (
            // Muted at rest; in full colour when it needs you or is chosen,
            // hovered or focused (calm r2).
            <span className={"flex shrink-0 " + appTone(selected || waits > 0)} data-testid="node-logo">
              <AppIcon {...appOfSession(member.session)} size={lines === 3 ? 20 : 16} />
            </span>
          ) : wrap ? (
            // On the first line of a name that wraps.
            <span className="flex h-4 shrink-0 items-center">{needMark || dot}</span>
          ) : (
            needMark || dot
          )}
          {lines === 1 && lead && <IconLead width={12} height={12} aria-hidden="true" className="shrink-0 text-accent-hi" />}
          {nameButton}
          {more}
        </div>
        {lines === 3 && (
          <span className="relative z-10 flex min-w-0 items-center gap-1.5 truncate text-label text-muted">
            <RoleWords role={member.role} />
            <ShellOnly tool={member.session.provider} account={member.session.account_id} project={member.session.project_id} inRow />
            <NodeRoom profileId={member.session.account_id} />
            <span aria-hidden="true">·</span>
            <span className="truncate">{member.card.tool ? appName(member.card.tool, member.card.account) : member.card.account}</span>
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
            {title.when && (
              <span className="ml-auto shrink-0 whitespace-nowrap text-faint" data-testid="node-when">
                {title.when}
              </span>
            )}
          </span>
        )}
        {lines === 2 && (
          <span className="flex min-w-0 items-center gap-1 text-label" data-testid="node-state">
            {dot}
            {lead && (
              <span className="inline-flex shrink-0 items-center gap-1 font-medium text-accent-hi" data-testid="node-lead">
                <IconLead width={12} height={12} aria-hidden="true" />
                Lead ·
              </span>
            )}
            {/* Its own box, so a long state ends in "…", not mid-letter. */}
            {needMark || <span className={"min-w-0 truncate " + state.tone}>{state.word}</span>}
          </span>
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
            className="absolute left-full top-0 z-40 ml-2 flex w-72 cursor-auto flex-col gap-2 rounded-xl border border-hairline bg-surface p-2.5 text-left shadow-lg"
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
            {paused.map((one) => (
              <PausedLine key={edgeKey(one.edge.from_session_id, one.edge.to_session_id)} paused={one} hops={hops} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
