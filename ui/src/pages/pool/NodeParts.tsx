/**
 * What a pool canvas's node shows in both its forms (the full card and
 * the compact one): an agent's ask with its answers, what else waits on
 * you about it, its edge, and the props both take.
 *
 * Calm (the rc3 critique, D-117): what waits on you about a member is
 * one mark, on its node (`nodeNeed`): its own alert, its agent's ask, or
 * a line paused into it at the hop limit, answered there (`PausedLine`);
 * its line shows nothing at rest. Its App's logo is muted at rest and in
 * full colour when it needs you or is chosen, hovered or focused
 * (`appTone`).
 */
import { RoomChip, useRoomMark } from "../../components/RoomChip";
import { useState, type JSX, type KeyboardEvent, type PointerEvent } from "react";

import { useShellOnly } from "../../components/ShellOnly";
import { errorText } from "../../lib/bridge";
import { messagesAnswer } from "../../lib/ipc/messages";
import { heldChoices, type HeldAnswer } from "../messages/words";
import { tasksAnswer } from "../../lib/ipc/tasks";
import type { CanvasEdgeInfo, NodeAskInfo } from "../../lib/ipc/pool-canvas";
import { sessionState } from "../../lib/rail-items";
import { PendingChip, pendingWords } from "./CanvasEdges";
import { SELECTED } from "./selection";
import { NODE_H, NODE_W, nodeForm, type Point } from "./layout";
import type { MemberNode } from "./useCanvas";

export const NO_TASK = "None";

/** Answers an agent's ask: a task's (Allow once, Always allow in this
 * pool, Deny), or a held message's (`heldChoices`, D-139). */
export function answerAsk(ask: NodeAskInfo, answer: HeldAnswer): Promise<void> {
  if (ask.source !== "task") return messagesAnswer(ask.id, answer);
  return tasksAnswer(ask.id, answer === "deliver" ? "once" : answer);
}

/** An ask's answers: a held message's as every surface offers them
 * (`heldChoices`), a task change's Allow once, Always allow, Deny. */
export function askChoices(ask: NodeAskInfo, always = "Always allow in this pool"): { id: HeldAnswer; label: string }[] {
  if (ask.source !== "task") return heldChoices(ask, always);
  return [{ id: "once", label: "Allow once" }, ...(ask.offer_always ? [{ id: "always" as const, label: always }] : []), { id: "deny", label: "Deny" }];
}

/** Each answer's test id on a node. */
const NODE_TEST_ID: Record<HeldAnswer, string> = {
  deliver: "node-ask-deliver",
  once: "node-ask-once",
  always: "node-ask-always",
  deny: "node-ask-deny",
};

/** How an answer's button looks on a node: the first is the one to
 * press, Deny the quietest. */
function nodeTone(id: HeldAnswer, first: boolean): string {
  if (first) return "border-warning/60 bg-warning/15 font-medium text-ink hover:bg-warning/25";
  return id === "deny" ? "border-hairline text-muted hover:bg-surface-2 hover:text-ink" : "border-hairline text-ink hover:bg-surface-2";
}

/** What about a member waits on you besides its agent's asks: its tool
 * asks you, it's at its limit, or it stopped by itself. */
export function sessionAlert(member: MemberNode): { word: string; tone: "warning" | "danger" } | null {
  const state = sessionState(member.session);
  if (member.session.status === "failed") return { word: "Stopped by itself", tone: "danger" };
  if (!state.needs) return null;
  return { word: state.word, tone: "warning" };
}

/** A line into a member paused at the hop limit, and who it's from. */
export type Paused = { edge: CanvasEdgeInfo; from: string };

/** What waits on you about a member, as its node's one mark: its own
 * alert ("Asks you", "Limit reached"), else its agent's ask, else a line
 * paused into it; and how many wait in all. None: nothing waits. */
export function nodeNeed(member: MemberNode, asks: readonly NodeAskInfo[], paused: readonly Paused[]): { word: string; tone: "warning" | "danger"; waits: number } | null {
  const alert = sessionAlert(member);
  const waits = asks.length + paused.length + (alert ? 1 : 0);
  if (waits === 0) return null;
  return { word: alert?.word ?? (asks.length > 0 ? "Asks you" : "Paused"), tone: alert?.tone ?? "warning", waits };
}

/** A line paused into a member at the hop limit, answered on its node:
 * who it's from, why, Let it continue and Don't deliver. */
export function PausedLine({ paused, hops }: { paused: Paused; hops?: number }): JSX.Element {
  return (
    <div className="relative z-10 flex flex-col gap-1 rounded-lg border border-warning/40 bg-warning-soft px-2 py-1.5" data-testid="node-paused">
      <span className="line-clamp-2 text-label text-ink">
        {paused.from}: {pendingWords(paused.edge, hops)?.label}
      </span>
      <PendingChip edge={paused.edge} hops={hops} bare />
    </div>
  );
}

export function AskLine({ ask }: { ask: NodeAskInfo }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const answer = (choice: HeldAnswer) => {
    setBusy(true);
    setError(null);
    void answerAsk(ask, choice).then(
      () => setBusy(false),
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  const button = "rounded-md border px-1.5 py-0.5 text-label outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 disabled:opacity-60";
  return (
    <div className="relative z-10 flex flex-col gap-1 rounded-lg border border-warning/40 bg-warning-soft px-2 py-1.5" data-testid="node-ask">
      <span className="line-clamp-2 text-label text-ink" title={ask.title}>
        {ask.title}
      </span>
      <div className="flex flex-wrap gap-1" role="group" aria-label={`Answer: ${ask.title}`}>
        {askChoices(ask, "Always allow here").map((choice, index) => (
          <button key={choice.id} type="button" disabled={busy} onClick={() => answer(choice.id)} className={button + " " + nodeTone(choice.id, index === 0)} data-testid={NODE_TEST_ID[choice.id]}>
            {choice.label}
          </button>
        ))}
      </div>
      {error && (
        <span className="text-label text-danger" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}

export type NodeProps = {
  member: MemberNode;
  at: Point;
  asks: NodeAskInfo[];
  /** The lines into it paused at the hop limit (answered on the node). */
  paused: readonly Paused[];
  /** The pool's replies in a row before a line pauses. */
  hops?: number;
  selected: boolean;
  tabStop: boolean;
  dragging: boolean;
  /** The canvas's zoom: under 100 % the node draws its words at screen
   * size (CompactNode). */
  scale: number;
  onSelect: (id: string, focusInspector: boolean) => void;
  onMenu: (id: string, at: { x: number; y: number }) => void;
  onPointerDown: (id: string, event: PointerEvent<HTMLDivElement>) => void;
  onKey: (id: string, event: KeyboardEvent<HTMLButtonElement>) => void;
};

/** What a node's name tells a screen reader beside the name: its state,
 * what waits on you, and why it works through its shell only (the rc19
 * walk: its chip never said why). */
export function useNodeDescription(member: MemberNode, word: string, waits: number): string {
  const shell = useShellOnly(member.session.provider, member.session.account_id, member.session.project_id);
  return `${word}${waits > 0 ? `, ${waits} waiting on you` : ""}${shell ? `. Shell only: ${shell}` : ""}`;
}

/** A node's surface and edge (AJ-E3012, D-117): one selection treatment
 * — the solid 2 px accent ring with an offset and a raised surface; at
 * rest the card stands off the canvas by its surface, with no border and
 * no glow. What waits on you is its amber word and dot (L2: no fill, one
 * mark); the lead is its Lead mark, never a ring that reads as chosen.
 * `_waits` and `_lead` stay in the signature: both forms pass them. */
export function nodeBorder(selected: boolean, _waits: boolean, _lead: boolean): string {
  return selected ? `border-transparent ${SELECTED}` : "border-transparent bg-surface shadow-card";
}


/** P1S6 step 3 (the rc29 walk): a member whose Account is out of room
 * shows the one Out mark on its node (`RoomChip`); a member with room, or
 * only low, shows nothing (the canvas stays calm). */
export function NodeRoom({ profileId }: { profileId: string | null | undefined }): JSX.Element | null {
  const mark = useRoomMark(profileId);
  return mark?.kind === "out" ? <RoomChip mark={mark} compact testId="node-room" /> : null;
}

/** A canvas, reused, to measure a word as the node draws it. */
let measure: CanvasRenderingContext2D | null | undefined;

/** Each word of `name` at `fontPx`, semibold, in px, and a space's:
 * measured where the page can (the app's view), else estimated (0.6 em a
 * letter). */
function wordWidths(name: string, fontPx: number): { words: number[]; space: number } {
  if (measure === undefined) {
    try {
      measure = typeof document !== "undefined" ? document.createElement("canvas").getContext("2d") : null;
    } catch {
      measure = null;
    }
  }
  const words = name.split(/\s+/).filter(Boolean);
  if (measure) {
    measure.font = `600 ${fontPx}px -apple-system, BlinkMacSystemFont, "SF Pro Text", Inter, sans-serif`;
    return { words: words.map((word) => measure!.measureText(word).width), space: measure.measureText(" ").width };
  }
  return { words: words.map((word) => word.length * fontPx * 0.6), space: fontPx * 0.3 };
}

/** AJ-E3012: a node's name breaks only between words. When one word is
 * wider than the room, the name is one line with an ellipsis instead
 * ("Plann/er" read as two words at a small fit); the full name is the
 * inspector's title and the node's tooltip. */
export function nameWraps(name: string, roomPx: number, fontPx: number): boolean {
  return Math.max(0, ...wordWidths(name, fontPx).words) <= roomPx;
}

/** The lines `name` takes in `roomPx` at `fontPx`, broken between words
 * as the page breaks them (the rc3 critique: a compact title picks the
 * size that reads it whole on its two lines). */
export function nameLines(name: string, roomPx: number, fontPx: number): number {
  const { words, space } = wordWidths(name, fontPx);
  let lines = words.length > 0 ? 1 : 0;
  let line = 0;
  for (const width of words) {
    if (line > 0 && line + space + width > roomPx) {
      lines += 1;
      line = width;
    } else line += (line > 0 ? space : 0) + width;
  }
  return lines;
}

/** A compact node's name (below 100 %): how many lines its box has, its
 * name's size, its room in px and whether it wraps (between words only).
 * One place, for the node and its tests. */
export function compactName(title: string, scale: number, lead: boolean, selected: boolean): { lines: 1 | 2 | 3; wrap: boolean; tight: boolean; width: number; height: number; nameSize: "text-body" | "text-meta" | "text-label"; fontPx: number; room: number; wrapWords: boolean } {
  const form = nodeForm(scale);
  const width = NODE_W * scale;
  const height = NODE_H * scale;
  const lines: 1 | 2 | 3 = form.full ? 3 : form.lines;
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
  // The name's room: the box less its padding, the state dot, the lead's
  // crown and the ⋯ slot when it shows; words break only between them.
  // Every gap of the row counts (dot, crown), the border too, and a tenth
  // to spare for the font's own widths: a word that only just fits is cut
  // by the edge (the gallery's "Planne"). The ⋯ folded takes no room and
  // no gap (the rc3 critique: its gap cut "Review the checkout…").
  const gaps = 6 * (lines === 1 && lead ? 2 : 1);
  // Its mark: the App's logo from two lines up (20 or 16 px), else the dot.
  const mark = lines === 3 ? 20 : lines === 2 ? 16 : 8;
  const space = width - (tight ? 12 : 16) - 2 - mark - (lines === 1 && lead ? 12 : 0) - gaps - (selected && width >= 140 ? 26 : 0);
  // The two-line form measures its words in the page's own font, so 2 px
  // to spare is enough (a tenth more cut "Checkout" at a 45 % fit, the
  // coordinator's critique); elsewhere a tenth, as the gallery's "Planne".
  const room = lines === 2 ? space - 2 : space / 1.1;
  // Two lines at 13 px where the name reads whole on them; else 12 px
  // (the rc3 critique: at a 53 % fit "Spike: payment…" and "Review the
  // checkout…" were cut at 13 px where 12 px held them).
  const nameSize = lines === 3 ? "text-body" : lines === 2 && !(wrap && (width < 150 || nameLines(title, room, 13) > 2)) ? "text-meta" : "text-label";
  const fontPx = nameSize === "text-body" ? 14 : nameSize === "text-meta" ? 13 : 12;
  const wrapWords = wrap && nameWraps(title, room, fontPx);
  return { lines, wrap, tight, width, height, nameSize, fontPx, room, wrapWords };
}
