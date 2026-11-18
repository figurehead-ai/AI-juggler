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
import { useInertOutside } from "../../lib/inert";
import { appName } from "../../lib/app-name";

import { AppIcon } from "../../components/AppIcon";
import { appOfSession } from "../../lib/app-of";
import { ShellOnly } from "../../components/ShellOnly";
import { IconLead, IconMore } from "../../components/icons-canvas";
import { sessionState, STATE_TONE } from "../../lib/rail-items";
import { NODE_H, NODE_W, nodeForm } from "./layout";
import { NodeButton, RoleWords, titleOf } from "./NodeButton";
import { AskLine, NO_TASK, NodeRoom, nameWraps, nodeBorder, sessionAlert, useNodeDescription, type NodeProps } from "./NodeParts";

export function CompactNode({ member, at, asks, held, selected, tabStop, dragging, scale, onSelect, onMenu, onPointerDown, onKey }: NodeProps): JSX.Element {
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
  const waits = asks.length + held + (alert ? 1 : 0);
  const task = member.card.task.value === NO_TASK || member.card.task.value === title.title ? null : member.card.task.value;
  const ended = state.state === "ended";
  const description = useNodeDescription(member, state.word, waits);
  const form = nodeForm(scale);
  const width = NODE_W * scale;
  const height = NODE_H * scale;
  const lines = form.full ? 3 : form.lines;
  // Room for the name on two lines above its state (a long name read
  // whole at fit, not cut at "Checkout cop…"); at the smallest, the name
  // on two lines with no state line (rc7 walk: at a 30 % fit every node
  // read "we…").
  // The coordinator's critique (D-126): at a 53 % fit every title read
  // "Checkout …" on one line. A two-line box always wraps its title over
  // two lines (12 px, tight) before it cuts it; the box grows into the
  // room the layout keeps under each node (GAP_Y) when the two lines and
  // the state need more than its height.
  const wrap = lines === 2 || (lines === 1 && height >= 32);
  const tight = lines === 1;
  // A narrow box (fit in a small window, ~130 px): the name at 12 px, so
  // "Spike: payment sandbox" reads whole on its two lines, not "Spike:
  // payment…" at 13 px.
  const nameSize = lines === 3 ? "text-body" : lines === 2 && !(wrap && width < 150) ? "text-meta" : "text-label";
  // The name's room: the box less its padding, the state dot, the lead's
  // crown and the ⋯ slot when it shows; words break only between them.
  const fontPx = nameSize === "text-body" ? 14 : nameSize === "text-meta" ? 13 : 12;
  // Every gap of the row counts (dot, crown, the ⋯ slot even when it's
  // folded), the border too, and a tenth to spare for the font's own
  // widths: a word that only just fits is cut by the edge (the gallery's
  // "Planne").
  const gaps = 6 * (lines === 1 && lead ? 3 : 2);
  // Its mark: the App's logo from two lines up (20 or 16 px), else the dot.
  const mark = lines === 3 ? 20 : lines === 2 ? 16 : 8;
  const space = width - (tight ? 12 : 16) - 2 - mark - (lines === 1 && lead ? 12 : 0) - gaps - (selected && width >= 140 ? 20 : 0);
  // The two-line form measures its words in the page's own font, so 2 px
  // to spare is enough (a tenth more cut "Checkout" at a 45 % fit, the
  // coordinator's critique); elsewhere a tenth, as the gallery's "Planne".
  const room = lines === 2 ? space - 2 : space / 1.1;
  const wrapWords = wrap && nameWraps(title.title, room, fontPx);

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
        "relative z-10 flex h-5 shrink-0 items-center justify-center overflow-hidden rounded-md text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:w-5 focus-visible:opacity-100 focus-visible:ring-1 focus-visible:ring-accent/60 " +
        // Its slot takes room only while it shows: at rest the name has the
        // node's width, so a long one reads whole at fit.
        // AJ-E3012: in a narrow box a selected node's name keeps its room
        // (the 38 % gallery fit read "Pl… a…"), and hovering doesn't take
        // it either (the name went to nothing under the pointer, and a
        // click on it missed); focus, right-click and ⇧F10 still open ⋯.
        (width >= 140 ? "group-hover/node:w-5 group-hover/node:opacity-100 " : "") +
        (selected && width >= 140 ? "w-5 opacity-100" : "w-0 opacity-0")
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
          "group/node relative flex flex-col justify-center gap-1 rounded-lg border data-[pending]:opacity-70 " +
          (tight ? "px-1.5 py-0.5 " : "px-2 py-1 ") +
          nodeBorder(selected, waits > 0, lead) +
          (dragging ? " cursor-grabbing opacity-90 shadow-lg" : " cursor-grab") +
          (ended && !selected ? " opacity-75" : "")
        }
      >
        <div className={"flex min-w-0 gap-1.5 " + (wrap ? "items-start" : "items-center")}>
          {lines >= 2 ? (
            <AppIcon {...appOfSession(member.session)} size={lines === 3 ? 20 : 16} />
          ) : wrap ? (
            // On the first line of a name that wraps.
            <span className="flex h-4 shrink-0 items-center">{dot}</span>
          ) : (
            dot
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
          <span className="flex min-w-0 items-center gap-1 truncate text-label" data-testid="node-state">
            {dot}
            {lead && (
              <span className="inline-flex shrink-0 items-center gap-1 font-medium text-accent-hi" data-testid="node-lead">
                <IconLead width={12} height={12} aria-hidden="true" />
                Lead ·
              </span>
            )}
            {/* Its own box, so a long state ends in "…", not mid-letter. */}
            <span className="min-w-0 truncate">{word}</span>
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
            className="absolute -right-2 -top-2 z-10 flex h-5 min-w-5 items-center justify-center rounded-full bg-warning px-1.5 font-mono text-label font-semibold text-canvas shadow-card outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-accent/60"
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
            {held > 0 && (
              <span className="text-label text-muted">
                {held === 1 ? "A message to it is" : `${held} messages to it are`} paused after too many replies in a row: answer on its edge.
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
