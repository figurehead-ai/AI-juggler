/**
 * ADR-0037 §2.4, §6 — memory in the conversation: the code-written echo
 * of "Remember …" (Edit, Undo), the refusal of something that looks like
 * a key, and the receipt under "Looked at" (each memory used, with
 * Forget). Only what the user said is ever kept (stated preferences).
 */
import type { JSX } from "react";
import { IconBookmark, IconEdit, IconLines, IconUndo } from "./bot-icons";
import { IconShield } from "../icons";
import { LogoTile } from "../ToolLogo";
import { WORDS } from "../../lib/bot-words";
import { agoWords } from "../../lib/time-words";
import { askForget, editMemory, undoMemory, wasUndone } from "../../lib/bot-memory";
import { useEffect, useState } from "react";
import { MEMORY_CHANGED } from "../../lib/bot-memory";
import type { BotMemoryInfo } from "../../types/generated/BotMemoryInfo";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";

/** "you call “web” the api agent", "“shop” matters most", or the note. */
export function memoryWords(memory: BotMemoryInfo, start = false): JSX.Element {
  if (memory.slot === "alias" && memory.about) {
    return (
      <>
        {start ? "You call" : "you call"} <b className="font-semibold text-ink">“{memory.about}”</b> <b className="font-semibold text-ink">{memory.text}</b>
      </>
    );
  }
  if (memory.slot === "focus" && memory.about) {
    return (
      <>
        <b className="font-semibold text-ink">“{memory.about}”</b> matters most
      </>
    );
  }
  return <>“{memory.text}”</>;
}

export function BotMemoryEcho({ turn }: { turn: BotTurnInfo }): JSX.Element {
  const memory = turn.memory;
  const [, redraw] = useState(0);
  useEffect(() => {
    const on = () => redraw((n) => n + 1);
    window.addEventListener(MEMORY_CHANGED, on);
    return () => window.removeEventListener(MEMORY_CHANGED, on);
  }, []);
  if (memory && wasUndone(memory.key)) {
    return <p className="text-meta text-muted" data-testid="bot-memory-undone">Undone: I won’t remember that.</p>;
  }
  if (turn.outcome === "remembered" && !memory) {
    return <p className="text-body text-ink-2">I couldn’t keep that: it’s too long or not the shape of a name or note.</p>;
  }
  if (turn.outcome === "refused" || !memory) {
    return (
      <div className="flex items-start gap-2.5 rounded-[14px] bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-memory-refused">
        <IconShield width={16} height={16} aria-hidden="true" className="mt-0.5 shrink-0 text-faint" />
        <span className="text-body text-ink-2">That looks like a key or password, so I won’t keep it.</span>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-2.5 rounded-[14px] bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-memory-echo">
      <div className="flex items-start gap-2.5">
        <LogoTile icon={IconBookmark} size={24} on />
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-body text-ink-2">I’ll remember: {memoryWords(memory)}.</span>
          <span className="text-meta text-muted">
            Stays on this Mac.{" "}
            <button type="button" className="rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => window.dispatchEvent(new Event("aij:bot-memories"))}>
              {WORDS.whatBotRemembers}
            </button>
          </span>
        </div>
      </div>
      <div className="flex gap-1.5 pl-[34px]">
        <button type="button" className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => editMemory(memory)} data-testid="bot-memory-edit">
          <IconEdit width={13} height={13} aria-hidden="true" />
          Edit
        </button>
        <button type="button" className="inline-flex h-7 items-center gap-1.5 rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void undoMemory(turn)} data-testid="bot-memory-undo">
          <IconUndo width={13} height={13} aria-hidden="true" />
          Undo
        </button>
      </div>
    </div>
  );
}

/** The receipt under "Looked at": memories used, and the records read. */
export function BotUsedMemories({ turn }: { turn: BotTurnInfo }): JSX.Element {
  const [now] = useState(() => Date.now());
  const sessions = turn.rows.filter((r) => r.kind === "session").length;
  const tasks = turn.rows.filter((r) => r.kind === "task").length;
  const read = [sessions ? `${sessions} ${sessions === 1 ? "session" : "sessions"}` : null, tasks ? `${tasks} ${tasks === 1 ? "task" : "tasks"}` : null].filter(Boolean).join(" and ");
  return (
    <div className="flex flex-col gap-px rounded-xl bg-surface-2 px-1.5 py-2" data-testid="bot-receipt">
      {turn.used.map((memory) => (
        <div key={memory.key} className="flex items-start gap-2.5 px-2 py-1 text-meta leading-snug text-ink-2">
          <IconBookmark width={14} height={14} aria-hidden="true" className="mt-px shrink-0 text-faint" />
          <span className="grow">
            {memory.slot === "alias" && memory.about ? (
              <>
                Your name for “{memory.about}”: <b className="font-medium text-ink">{memory.text}</b>
              </>
            ) : (
              memory.text
            )}{" "}
            <span className="whitespace-nowrap text-faint">· you said, {agoWords(memory.at_ms, now)}</span>
          </span>
          <button type="button" className="rounded-md font-medium whitespace-nowrap text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => askForget(memory)} data-testid="bot-receipt-forget">
            Forget
          </button>
        </div>
      ))}
      <div className="flex items-start gap-2.5 px-2 py-1 text-meta text-ink-2">
        <IconLines width={14} height={14} aria-hidden="true" className="mt-px shrink-0 text-faint" />
        <span>{read ? `${read}, read ${agoWords(turn.at_ms, now)}` : "Nothing matched"}</span>
      </div>
    </div>
  );
}
