/**
 * What a pool canvas's node shows in both its forms (the full card and
 * the compact one): an agent's ask with its answers, what else waits on
 * you about it, its edge, and the props both take.
 */
import { RoomChip, useRoomMark } from "../../components/RoomChip";
import { useState, type JSX, type KeyboardEvent, type PointerEvent } from "react";

import { useShellOnly } from "../../components/ShellOnly";
import { errorText } from "../../lib/bridge";
import { messagesAnswer } from "../../lib/ipc/messages";
import { tasksAnswer } from "../../lib/ipc/tasks";
import type { NodeAskInfo } from "../../lib/ipc/pool-canvas";
import { sessionState } from "../../lib/rail-items";
import { SELECTED } from "./selection";
import type { Point } from "./layout";
import type { MemberNode } from "./useCanvas";

export const NO_TASK = "None";

/** Answers an agent's ask (Allow once, Always allow in this pool, Deny). */
export function answerAsk(ask: NodeAskInfo, answer: "once" | "always" | "deny"): Promise<void> {
  return ask.source === "task" ? tasksAnswer(ask.id, answer) : messagesAnswer(ask.id, answer);
}

/** What about a member waits on you besides its agent's asks: its tool
 * asks you, it's at its limit, or it stopped by itself. */
export function sessionAlert(member: MemberNode): { word: string; tone: "warning" | "danger" } | null {
  const state = sessionState(member.session);
  if (member.session.status === "failed") return { word: "Stopped by itself", tone: "danger" };
  if (!state.needs) return null;
  return { word: state.word, tone: "warning" };
}

export function AskLine({ ask }: { ask: NodeAskInfo }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const answer = (choice: "once" | "always" | "deny") => {
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
        <button type="button" disabled={busy} onClick={() => answer("once")} className={button + " border-warning/60 bg-warning/15 font-medium text-ink hover:bg-warning/25"} data-testid="node-ask-once">
          Allow once
        </button>
        {ask.offer_always && (
          <button type="button" disabled={busy} onClick={() => answer("always")} className={button + " border-hairline text-ink hover:bg-surface-2"} data-testid="node-ask-always">
            Always allow here
          </button>
        )}
        <button type="button" disabled={busy} onClick={() => answer("deny")} className={button + " border-hairline text-muted hover:bg-surface-2 hover:text-ink"} data-testid="node-ask-deny">
          Deny
        </button>
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
  /** Messages to or from it held at the hop limit (answered on the edge). */
  held: number;
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
 * no glow. What waits on you is its amber word, dot and badge (L2: no
 * fill); the lead is its Lead mark, never a ring that reads as chosen.
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

/** The widest word of `name` at `fontPx`, semibold, in px: measured where
 * the page can (the app's view), else estimated (0.6 em a letter). */
function widestWord(name: string, fontPx: number): number {
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
    return Math.max(0, ...words.map((word) => measure!.measureText(word).width));
  }
  return Math.max(0, ...words.map((word) => word.length * fontPx * 0.6));
}

/** AJ-E3012: a node's name breaks only between words. When one word is
 * wider than the room, the name is one line with an ellipsis instead
 * ("Plann/er" read as two words at a small fit); the full name is the
 * inspector's title and the node's tooltip. */
export function nameWraps(name: string, roomPx: number, fontPx: number): boolean {
  return widestWord(name, fontPx) <= roomPx;
}
