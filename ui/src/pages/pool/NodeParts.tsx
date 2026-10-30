/**
 * What a pool canvas's node shows in both its forms (the full card and
 * the compact one): an agent's ask with its answers, what else waits on
 * you about it, its edge, and the props both take.
 */
import { useState, type JSX, type KeyboardEvent, type PointerEvent } from "react";

import { errorText } from "../../lib/bridge";
import { messagesAnswer } from "../../lib/ipc/messages";
import { tasksAnswer } from "../../lib/ipc/tasks";
import type { NodeAskInfo } from "../../lib/ipc/pool-canvas";
import { sessionState } from "../../lib/rail-items";
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
    <div className="flex flex-col gap-1 rounded-lg border border-warning/40 bg-warning-soft px-2 py-1.5" data-testid="node-ask">
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

/** A node's edge: chosen, waiting on you (a warm glow), the lead, or plain. */
export function nodeBorder(selected: boolean, waits: boolean, lead: boolean): string {
  return selected
    ? "border-accent ring-2 ring-accent/50"
    : waits
      ? "border-warning/70 shadow-[0_0_0_1px_var(--color-warning-soft),0_0_18px_var(--color-warning-soft)]"
      : lead
        ? "border-accent/70 ring-1 ring-accent/40"
        : "border-hairline";
}

