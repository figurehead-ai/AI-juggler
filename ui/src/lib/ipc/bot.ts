/**
 * D-136 — Bot's commands, as the window calls them: ask (writes only the
 * turn, on this Mac), a page of the one conversation, find in it, New
 * topic, clear, the clear sheet's counts, and "This is wrong".
 *
 * The preview and tests set `window.__AIJ_BOT__`: a page of turns, the
 * answers to give, the counts. With none, the conversation is empty and
 * an answer is "I don't know" (nothing is looked up in the preview).
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { BotAskInput } from "../../types/generated/BotAskInput";
import type { BotCountInfo } from "../../types/generated/BotCountInfo";
import type { BotFoundInfo } from "../../types/generated/BotFoundInfo";
import type { BotMemoriesInfo } from "../../types/generated/BotMemoriesInfo";
import { FORGET_AND_BACKUPS as FORGET_AND_BACKUPS_PREVIEW } from "../bot-words";
import type { BotPageInfo } from "../../types/generated/BotPageInfo";
import type { BotScope } from "../../types/generated/BotScope";
import type { BotTopicInfo } from "../../types/generated/BotTopicInfo";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";

export type { BotAskInput, BotCountInfo, BotFoundInfo, BotMemoriesInfo, BotPageInfo, BotTopicInfo, BotTurnInfo };

/** The preview's seam. */
export interface BotSeam {
  /** The conversation, newest first. */
  turns?: BotTurnInfo[];
  topics?: BotTopicInfo[];
  /** Answers by question (the preview's look-ups). */
  answers?: Record<string, Omit<BotTurnInfo, "id" | "at_ms" | "question">>;
  count?: BotCountInfo;
  memories?: BotMemoriesInfo;
  /** The chip the panel opens with (the preview's door). */
  scope?: BotScope;
  /** Calls made, for tests: `[command, args]`. */
  calls?: [string, unknown][];
}

declare global {
  interface Window {
    __AIJ_BOT__?: BotSeam;
  }
}

function seam(): BotSeam {
  window.__AIJ_BOT__ ??= {};
  return window.__AIJ_BOT__;
}

function note(command: string, args: unknown): void {
  (seam().calls ??= []).push([command, args]);
}

const PREVIEW_PAGE = 20;

/** Asks Bot; the turn as kept. */
export async function botAsk(input: BotAskInput): Promise<BotTurnInfo> {
  if (!hasLiveBackend()) {
    note("bot_ask", input);
    const preset = seam().answers?.[input.question];
    const turn: BotTurnInfo = {
      id: `t-${Date.now()}-${(seam().turns ?? []).length}`,
      at_ms: Date.now(),
      question: input.question,
      ...(preset ?? {
        lang: "en",
        outcome: "dont_know",
        scope: input.scope,
        rows: [],
        total: 0,
        tallies: [],
        unknown: "nothing_found",
        shortcut: "finished_today",
        looked: { reach: "searched", scope: input.scope, memories: 0 },
        written: null,
        memory: null,
        used: [],
        live: true,
        redacted: false,
        flagged: false,
      }),
    };
    seam().turns = [turn, ...(seam().turns ?? [])];
    return turn;
  }
  return call("bot_ask", { input });
}

/** A page of the conversation, newest first: before `before` (the oldest
 * turn shown), or the newest. */
export async function botPage(before: { atMs: number; id: string } | null, limit = PREVIEW_PAGE): Promise<BotPageInfo> {
  if (!hasLiveBackend()) {
    const all = seam().turns ?? [];
    const start = before ? all.findIndex((t) => t.id === before.id) + 1 : 0;
    const turns = all.slice(start, start + limit);
    const low = start + limit < all.length ? (turns.at(-1)?.at_ms ?? 0) : Number.NEGATIVE_INFINITY;
    const high = before ? before.atMs : Number.POSITIVE_INFINITY;
    const topics = (seam().topics ?? []).filter((t) => t.started_ms >= low && t.started_ms <= high);
    return { turns, topics, more: start + limit < all.length };
  }
  return call("bot_page", { beforeMs: before?.atMs ?? null, beforeId: before?.id ?? null, limit });
}

/** The turns holding `words`, oldest first. */
export async function botFind(words: string): Promise<BotFoundInfo[]> {
  if (!hasLiveBackend()) {
    const w = words.trim().toLowerCase();
    if (!w) return [];
    return (seam().turns ?? [])
      .filter((t) => t.question.toLowerCase().includes(w) || t.rows.some((r) => r.title.toLowerCase().includes(w)))
      .map((t) => ({ id: t.id, at_ms: t.at_ms }))
      .reverse();
  }
  return call("bot_find", { words });
}

/** Starts a New topic. */
export async function botNewTopic(): Promise<BotTopicInfo> {
  if (!hasLiveBackend()) {
    note("bot_new_topic", null);
    const topic = { id: `topic-${Date.now()}`, started_ms: Date.now(), first: false };
    seam().topics = [topic, ...(seam().topics ?? [])];
    return topic;
  }
  return call("bot_new_topic");
}

/** What the clear sheet says goes and stays. */
export async function botCount(): Promise<BotCountInfo> {
  if (!hasLiveBackend()) {
    const turns = seam().turns ?? [];
    return seam().count ?? { questions: turns.length, since_ms: turns.at(-1)?.at_ms ?? null, memories: 0, watches: 0 };
  }
  return call("bot_count");
}

/** Clears the whole conversation from this Mac. */
export async function botClear(): Promise<number> {
  if (!hasLiveBackend()) {
    note("bot_clear", null);
    const n = (seam().turns ?? []).length;
    seam().turns = [];
    seam().topics = [];
    return n;
  }
  return call("bot_clear");
}

/** "This is wrong" about a turn's written answer (kept on this Mac). */
export async function botWrong(turn: string): Promise<void> {
  if (!hasLiveBackend()) {
    note("bot_wrong", turn);
    return;
  }
  return call("bot_wrong", { turn });
}

/** What Bot remembers: the user's names, focus and notes. */
export async function botMemories(): Promise<BotMemoriesInfo> {
  if (!hasLiveBackend()) {
    return seam().memories ?? { items: [], forget_and_backups: FORGET_AND_BACKUPS_PREVIEW, watches: 0, questions: (seam().turns ?? []).length };
  }
  return call("bot_memories");
}

/** Forgets one memory, every version (the sheet asked first). */
export async function botForget(key: string): Promise<void> {
  if (!hasLiveBackend()) {
    note("bot_forget", key);
    const memories = seam().memories;
    if (memories) seam().memories = { ...memories, items: memories.items.filter((m) => m.key !== key) };
    return;
  }
  return call("bot_forget", { key });
}

/** The preview's chip (none with the real backend: only a door sets it). */
export function previewScope(): BotScope | null {
  return hasLiveBackend() ? null : (seam().scope ?? null);
}
