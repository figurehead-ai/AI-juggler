/**
 * E18 (AJ-E1802 Req.2) — what about messages waits on you, in Home's
 * Needs you:
 * - an agent's message above its pool's level, and a chain that reached
 *   the hop limit, as the same approval cards as a tool's asks (E23: the
 *   batch, keys 1–9, Enter for the first); the choices are the pool's
 *   (Allow once, Always allow in this pool from the second time, Deny),
 *   and a rule never lifts the hop limit (`MessageNeeds`);
 * - an agent's question to you (Reply · Open the thread, the question
 *   itself inline: pool messages are ours, so their words are always
 *   there), messages waiting for a session run in AI Juggler (Deliver,
 *   one click for all of them), and a paused pool or thread (Resume): one
 *   card each (`MessageNeedCard`, D-118's mock), in Needs you's one order
 *   with the sessions' needs (HomeNeeds).
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { AskCard } from "./AskCard";
import { useActionHost } from "./ItemMenu";
import { AgentDisc, NeedActions, NeedByLine, needCardClass } from "./NeedCard";
import { rememberTab } from "../pages/registry";
import { minutes } from "../lib/time-words";
import {
  focusThread,
  messagesAnswer,
  messagesDeliverWaiting,
  messagesSetPool,
  messagesSetThread,
  type MessageNeedInfo,
} from "../lib/ipc/messages";
import type { AskInfo } from "../lib/ipc/runtime";

/** A held ask as the approval card shows it. */
function asAsk(need: MessageNeedInfo): AskInfo {
  const deliver = need.kind === "deliver";
  const options = deliver
    ? [
        { id: "once", label: "Deliver", effect: "allow_once", rule: null },
        { id: "deny", label: "Don't deliver", effect: "deny", rule: null },
      ]
    : [
        { id: "once", label: "Allow once", effect: "allow_once", rule: null },
        ...(need.offer_always ? [{ id: "always", label: "Always allow in this pool", effect: "allow_rule", rule: null }] : []),
        { id: "deny", label: "Deny", effect: "deny", rule: null },
      ];
  return {
    id: need.ask_id ?? need.key,
    session_id: "",
    kind: "approval",
    approval_kind: null,
    summary: need.title,
    reason: [need.detail, need.preview].filter(Boolean).join("\n"),
    options,
    parts: [],
    asked_at_ms: need.at_ms,
  };
}

/** Held asks (approval cards) as against the rest (one card each). */
export function isHeldAsk(need: MessageNeedInfo): boolean {
  return need.kind === "ask" || need.kind === "deliver";
}

const PRIMARY: Record<string, string> = {
  question: "Reply",
  waiting: "Deliver",
  pool_paused: "Resume messages",
  thread_paused: "Resume",
};

/** Where in its pool: the pool, then its thread ("Release pool · Export
 * flow"); a paused pool's title names it already. */
function whereOf(need: MessageNeedInfo): string {
  if (need.kind === "pool_paused") return need.detail;
  return [need.pool_name, need.thread ?? need.detail].filter(Boolean).join(" · ");
}

/** Its card's words: who, where, and what it says. */
function wordsOf(need: MessageNeedInfo): { author: string; place: string | null } {
  if (need.kind === "question" || need.kind === "waiting") return { author: need.sender ?? need.pool_name, place: whereOf(need) || null };
  return { author: need.pool_name, place: need.kind === "pool_paused" ? null : (need.thread ?? null) };
}

/** Its primary and opening, as Needs you runs them. */
function useMessageActions(): { open: (need: MessageNeedInfo, reply?: boolean) => void; primary: (need: MessageNeedInfo) => void } {
  const host = useActionHost();
  const open = (need: MessageNeedInfo, reply = false) => {
    focusThread(need.thread_id, reply);
    rememberTab("pool", "messages");
    host.openPool?.(need.pool_id);
  };
  const primary = (need: MessageNeedInfo) => {
    switch (need.kind) {
      case "waiting":
        if (need.session_id) void messagesDeliverWaiting(need.session_id);
        return;
      case "pool_paused":
        void messagesSetPool(need.pool_id, false);
        return;
      case "thread_paused":
        if (need.thread_id) void messagesSetThread(need.thread_id, "open");
        return;
      case "question":
        open(need, true);
        return;
      default:
        open(need);
    }
  };
  return { open, primary };
}

/** An agent's message need as its own card (D-118's mock): "builder-1 ·
 * Release pool · Export flow  5 min", then 'Asks you: "Use the v2 export
 * API…?"', then Reply · Open the thread. `first`: the first card in
 * Needs you, its primary the one filled button. */
export function MessageNeedCard({ need, now, first = false, keys }: { need: MessageNeedInfo; now?: number; first?: boolean; keys?: string }): JSX.Element {
  const { open, primary } = useMessageActions();
  const { author, place } = wordsOf(need);
  const paused = need.kind.endsWith("paused");
  const question = need.kind === "question";
  const said = need.preview?.split("\n").find((line) => line.trim())?.trim() ?? null;
  const name = question ? `${author} asks you${said ? `: ${said}` : ""}` : need.title;
  return (
    <li className={needCardClass(2)} data-testid={`message-need-${need.kind}`} data-need-key={`message:${need.key}`} data-home-item data-first={first ? "true" : undefined} aria-label={name}>
      {paused ? (
        <span aria-hidden="true" className="flex size-[22px] shrink-0 items-center justify-center rounded-md bg-warning-soft text-label text-warning">
          ✉
        </span>
      ) : (
        <AgentDisc name={need.sender ?? need.pool_name} />
      )}
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <NeedByLine author={author} place={place} time={now === undefined ? null : minutes(need.at_ms, now)} timeTestId="need-wait" />
        <span className="text-body text-ink" data-testid="message-need-said">
          {question ? (
            <>
              <span className="font-semibold text-warning">Asks you</span>
              {said ? <span data-testid="need-asks">: “{said}”</span> : <span className="text-muted"> — open the thread to read it</span>}
            </>
          ) : (
            <>
              <span className="font-semibold">{need.title}</span>
              {paused && need.detail && need.kind === "thread_paused" && <span className="text-muted"> — {need.detail}</span>}
              {need.kind === "waiting" && said && <span className="text-ink-2">: “{said}”</span>}
            </>
          )}
        </span>
        <NeedActions>
          <Button size="sm" variant={first ? "primary" : "ghost"} data-home-row aria-keyshortcuts={first ? "Enter" : keys} onClick={() => primary(need)}>
            {PRIMARY[need.kind] ?? "Open"}
            {first && <span aria-hidden="true" className="opacity-80">↵</span>}
            <span className="sr-only"> — {need.title}</span>
          </Button>
          {need.thread_id && (
            <Button size="sm" variant="ghost" onClick={() => open(need)}>
              {question ? "Open the thread" : "Read"}
              <span className="sr-only"> — {need.title}</span>
            </Button>
          )}
        </NeedActions>
      </span>
    </li>
  );
}

/** Its held asks, as approval cards (the rest are cards of their own). */
export function MessageNeeds({ needs }: { needs: MessageNeedInfo[] }): JSX.Element | null {
  const asks = needs.filter(isHeldAsk);
  if (asks.length === 0) return null;
  return (
    <div className="flex flex-col gap-2" data-testid="message-needs">
      {asks.map((need) => (
        <AskCard
          key={need.key}
          ask={asAsk(need)}
          kindLabel={need.kind === "deliver" ? "A chain of replies paused for you" : "An agent asks to send"}
          title={need.pool_name}
          answer={(option) => messagesAnswer(need.ask_id ?? "", option as "once" | "always" | "deny")}
        />
      ))}
    </div>
  );
}
