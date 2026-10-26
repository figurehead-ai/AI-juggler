/**
 * E18 (AJ-E1802 Req.2) — what about messages waits on you, in Home's
 * Needs you:
 * - an agent's message above its pool's level, and a chain that reached
 *   the hop limit, as the same approval cards as a tool's asks (E23: the
 *   batch, keys 1–9, Enter for the first); the choices are the pool's
 *   (Allow once, Always allow in this pool from the second time, Deny),
 *   and a rule never lifts the hop limit;
 * - an agent's question to you (Reply), messages waiting for a session run
 *   in AI Juggler (Deliver, one click for all of them), and a paused pool
 *   or thread (Resume), one row each.
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { AskCard } from "./AskCard";
import { useActionHost } from "./ItemMenu";
import { rememberTab } from "../pages/registry";
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

const PRIMARY: Record<string, string> = {
  question: "Reply",
  waiting: "Deliver",
  pool_paused: "Resume messages",
  thread_paused: "Resume",
};

export function MessageNeeds({ needs }: { needs: MessageNeedInfo[] }): JSX.Element | null {
  const host = useActionHost();
  if (needs.length === 0) return null;
  const asks = needs.filter((need) => need.kind === "ask" || need.kind === "deliver");
  const rest = needs.filter((need) => need.kind !== "ask" && need.kind !== "deliver");
  const open = (need: MessageNeedInfo) => {
    focusThread(need.thread_id);
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
      default:
        open(need);
    }
  };
  return (
    <div className="flex flex-col gap-2" data-testid="message-needs">
      {asks.map((need) => (
        <AskCard
          key={need.key}
          ask={asAsk(need)}
          kindLabel={need.kind === "deliver" ? "A chain reached the hop limit" : "An agent asks to send"}
          title={need.pool_name}
          answer={(option) => messagesAnswer(need.ask_id ?? "", option as "once" | "always" | "deny")}
        />
      ))}
      {rest.length > 0 && (
        <ul className="flex flex-col">
          {rest.map((need) => (
            <li key={need.key} className="flex min-w-0 items-center gap-2.5 border-b border-hairline/40 py-2 last:border-b-0" data-testid={`message-need-${need.kind}`} data-home-item>
              <span aria-hidden="true" className={"flex size-[22px] shrink-0 items-center justify-center rounded-lg text-[12px] " + (need.kind.endsWith("paused") ? "bg-warning-soft text-warning" : "bg-accent-soft text-accent-hi")}>
                ✉
              </span>
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-[13px] font-medium text-ink">{need.title}</span>
                <span className="truncate text-[12px] text-muted">{need.detail}</span>
              </span>
              {need.kind !== "question" && need.thread_id && (
                <Button size="sm" variant="ghost" onClick={() => open(need)}>
                  Read
                  <span className="sr-only"> — {need.title}</span>
                </Button>
              )}
              <Button size="sm" variant="secondary" data-home-row onClick={() => primary(need)}>
                {PRIMARY[need.kind] ?? "Open"}
                <span className="sr-only"> — {need.title}</span>
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
