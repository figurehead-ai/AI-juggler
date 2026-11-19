/**
 * AJ-E3819's UI part — memory from the conversation: Forget asks first
 * (the forget sheet, with FORGET_AND_BACKUPS word for word), Edit puts the
 * words back in the box, Undo forgets what was just kept. The Bot page's
 * "What Bot remembers" tab reads the list (lazily, when it opens).
 */
import { useEffect, useState } from "react";
import { botForget, botMemories, type BotMemoriesInfo } from "./ipc/bot";
import { botState, openBot, setDraft } from "./bot-store";
import { errorText } from "./bridge";
import type { BotMemoryInfo } from "../types/generated/BotMemoryInfo";
import type { BotTurnInfo } from "../types/generated/BotTurnInfo";

/** The event the forget sheet listens for. */
export const FORGET_EVENT = "aij:bot-forget";
/** The event that says the memory list changed. */
export const MEMORY_CHANGED = "aij:bot-memory-changed";

/** Asks to forget `memory` (the sheet confirms). */
export function askForget(memory: BotMemoryInfo): void {
  window.dispatchEvent(new CustomEvent<BotMemoryInfo>(FORGET_EVENT, { detail: memory }));
}

/** The words that said it, back in the box to change. */
export function editMemory(memory: BotMemoryInfo): void {
  const words = memory.slot === "alias" && memory.about ? `Remember: call ${memory.about} ${memory.text}` : memory.slot === "focus" && memory.about ? `Remember: ${memory.about} matters most` : `Remember: ${memory.text}`;
  if (botState().mode === "closed") openBot();
  setDraft(words);
}

const undone = new Set<string>();

/** Whether the memory a turn kept was undone. */
export function wasUndone(key: string): boolean {
  return undone.has(key);
}

/** Undo: forgets what was just kept. */
export async function undoMemory(turn: BotTurnInfo): Promise<void> {
  if (!turn.memory) return;
  await botForget(turn.memory.key);
  undone.add(turn.memory.key);
  window.dispatchEvent(new Event(MEMORY_CHANGED));
}

/** Forgets a memory (the sheet's button). */
export async function forgetMemory(key: string): Promise<void> {
  await botForget(key);
  undone.add(key);
  window.dispatchEvent(new Event(MEMORY_CHANGED));
}

/** The list, read when a view wants it, again when it changes. */
export function useMemories(): { info: BotMemoriesInfo | null; error: string | null } {
  const [info, setInfo] = useState<BotMemoriesInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    const read = () => {
      void botMemories()
        .then((next) => live && setInfo(next))
        .catch((cause) => live && setError(errorText(cause)));
    };
    read();
    window.addEventListener(MEMORY_CHANGED, read);
    return () => {
      live = false;
      window.removeEventListener(MEMORY_CHANGED, read);
    };
  }, []);
  return { info, error };
}
