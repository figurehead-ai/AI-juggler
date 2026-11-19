/**
 * D-136 — Bot's doors: the top bar's icon (⌘J, its tooltip "Bot ⌘J"),
 * and "Ask Bot about this" on a session, App, project page and a feed
 * item, which opens the one conversation docked, the box focused and the
 * scope chip filled in by the target's id (actions target by id).
 */
import type { JSX } from "react";
import { IconBot } from "./bot-icons";
import { WORDS } from "../../lib/bot-words";
import { openBot, toggleBot, useBot, type BotScope } from "../../lib/bot-store";
import { keysOf } from "../../lib/shortcuts";

export function BotButton(): JSX.Element {
  const bot = useBot();
  const on = bot.mode !== "closed";
  const keys = keysOf("bot")[0] ?? "⌘J";
  return (
    <button
      type="button"
      className={"grid size-[34px] place-items-center rounded-[9px] outline-none focus-visible:ring-2 focus-visible:ring-accent/60 focus-visible:ring-offset-2 focus-visible:ring-offset-canvas " + (on ? "bg-surface-3 text-ink inset-ring-1 inset-ring-hairline" : "text-muted hover:bg-surface-2 hover:text-ink")}
      title={`${WORDS.iconTip} (${keys})`}
      aria-label={WORDS.iconTip}
      aria-keyshortcuts="Meta+J"
      aria-expanded={bot.mode === "panel"}
      onClick={toggleBot}
      data-testid="header-bot"
    >
      <IconBot width={19} height={19} aria-hidden="true" />
    </button>
  );
}

/** "Ask Bot about this", beside an object's own actions. */
export function AskBot({ scope, size = "sm", className = "" }: { scope: BotScope; size?: "sm" | "xs"; className?: string }): JSX.Element {
  return (
    <button
      type="button"
      className={(size === "sm" ? "h-7 gap-1.5 px-2.5 " : "h-6 gap-1 px-2 ") + "inline-flex shrink-0 items-center rounded-lg bg-surface-2 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60 " + className}
      onClick={(event) => {
        event.stopPropagation();
        openBot(scope);
      }}
      data-testid="ask-bot"
      data-scope={`${scope.kind}:${scope.id}`}
    >
      <IconBot width={size === "sm" ? 14 : 13} height={size === "sm" ? 14 : 13} aria-hidden="true" />
      Ask Bot about this
    </button>
  );
}
