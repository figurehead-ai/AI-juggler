/**
 * D-136 — Bot's one state: one conversation, shown in the docked panel or
 * on the Bot page, never both. Switching between them keeps the draft,
 * the scope chip and the place in the conversation (point 4).
 *
 * Lazy (lazy-load-everywhere): nothing is read until Bot first opens;
 * then the newest page, and "Earlier" loads the page above, 20 at a time,
 * never all (instant-and-paged). The width is a per-viewer convenience,
 * kept in local storage.
 */
import { useSyncExternalStore } from "react";
import { readKept, keep } from "./local-store";
import { listeners } from "./listeners";
import { botAsk, botClear, botFind, botNewTopic, botPage, previewScope, type BotTopicInfo, type BotTurnInfo } from "./ipc/bot";
import { errorText } from "./bridge";
import type { BotScope } from "../types/generated/BotScope";

export type { BotScope };

/** Where Bot is. */
type BotMode = "closed" | "panel" | "page";

/** The panel's width: 340 to 560, 400 by default (the mock). */
const WIDTH_MIN = 340;
const WIDTH_MAX = 560;
export const WIDTH_DEFAULT = 400;
const WIDTH_KEY = "aij.bot.width";

/** Under this window width the panel is a sheet over the page. */
export const SHEET_UNDER = 1100;

interface BotFind {
  words: string;
  /** Matching turns, oldest first. */
  matches: string[];
  /** The current match (an index into `matches`). */
  at: number;
}

export interface BotState {
  mode: BotMode;
  width: number;
  draft: string;
  scope: BotScope | null;
  /** Oldest first. */
  turns: BotTurnInfo[];
  topics: BotTopicInfo[];
  more: boolean;
  loaded: boolean;
  loading: boolean;
  asking: boolean;
  error: string | null;
  find: BotFind | null;
  /** The turn at the top of the view, kept across panel and page. */
  anchor: string | null;
  /** Bumped to focus the question box (a door, ⌘J). */
  focusKey: number;
}

function keptWidth(): number {
  const n = Number(readKept(WIDTH_KEY));
  return Number.isFinite(n) && n >= WIDTH_MIN && n <= WIDTH_MAX ? n : WIDTH_DEFAULT;
}

const INITIAL: BotState = {
  mode: "closed",
  width: WIDTH_DEFAULT,
  draft: "",
  scope: null,
  turns: [],
  topics: [],
  more: false,
  loaded: false,
  loading: false,
  asking: false,
  error: null,
  find: null,
  anchor: null,
  focusKey: 0,
};

let state: BotState = { ...INITIAL, width: keptWidth() };
const { subscribe, notify } = listeners();

function set(next: Partial<BotState>): void {
  state = { ...state, ...next };
  notify();
}

/** The state now (tests, non-React callers). */
export function botState(): BotState {
  return state;
}

/** Bot's state, for a view. */
export function useBot(): BotState {
  return useSyncExternalStore(subscribe, botState, botState);
}

/** Back to nothing open and nothing read (tests). */
export function resetBot(): void {
  state = { ...INITIAL, width: keptWidth() };
  notify();
}

function merge(older: BotTopicInfo[], newer: BotTopicInfo[]): BotTopicInfo[] {
  const byId = new Map([...older, ...newer].map((t) => [t.id, t]));
  return [...byId.values()].sort((a, b) => a.started_ms - b.started_ms);
}

async function loadNewest(): Promise<void> {
  if (state.loaded || state.loading) return;
  set({ loading: true, error: null });
  try {
    const page = await botPage(null);
    set({ turns: [...page.turns].reverse(), topics: merge([], page.topics), more: page.more, loaded: true, loading: false });
  } catch (cause) {
    set({ loading: false, error: errorText(cause) });
  }
}

/** The page above the oldest turn shown. */
export async function loadEarlier(): Promise<void> {
  const oldest = state.turns[0];
  if (!oldest || !state.more || state.loading) return;
  set({ loading: true });
  try {
    const page = await botPage({ atMs: oldest.at_ms, id: oldest.id });
    set({ turns: [...[...page.turns].reverse(), ...state.turns], topics: merge(state.topics, page.topics), more: page.more, loading: false });
  } catch (cause) {
    set({ loading: false, error: errorText(cause) });
  }
}

/** Opens the panel (or keeps the page), the box focused; a door sets the
 * scope (the chip), by id. */
export function openBot(scope?: BotScope | null): void {
  const next: Partial<BotState> = { mode: state.mode === "page" ? "page" : "panel", focusKey: state.focusKey + 1 };
  if (scope !== undefined) next.scope = scope;
  else if (!state.loaded && state.scope === null) next.scope = previewScope();
  set(next);
  void loadNewest();
}

/** ⌘J and the icon: open the panel, or close it. */
export function toggleBot(): void {
  if (state.mode === "panel") closeBot();
  else openBot();
}

/** Closes the panel; focus goes back to the icon. */
export function closeBot(): void {
  set({ mode: "closed", find: null });
  window.requestAnimationFrame(() => document.querySelector<HTMLElement>("[data-testid=header-bot]")?.focus());
}

/** The Bot page (Expand, ⋯ › Open the Bot page). */
export function expandBot(): void {
  set({ mode: "page" });
  void loadNewest();
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "bot" } }));
}

/** "Back to the panel": the same place in the conversation. */
export function backToPanel(): void {
  set({ mode: "panel", focusKey: state.focusKey + 1 });
  window.dispatchEvent(new Event("aij:bot-back"));
}

/** The page was left some other way: the panel stays closed. */
export function leftPage(): void {
  if (state.mode === "page") set({ mode: "closed" });
}

export function setDraft(draft: string): void {
  set({ draft });
}

/** Starts the question with `draft` ("Remember: ") and puts the cursor
 * after it, so the words typed next land in the box (the 0.5 walk's N-04). */
export function startDraft(draft: string): void {
  set({ draft, focusKey: state.focusKey + 1 });
}

/** × on the chip: ask about everything. */
export function widenScope(): void {
  set({ scope: null, focusKey: state.focusKey + 1 });
}

export function setWidth(width: number): void {
  const w = Math.round(Math.min(WIDTH_MAX, Math.max(WIDTH_MIN, width)));
  set({ width: w });
  keep(WIDTH_KEY, String(w));
}

export function setAnchor(anchor: string | null): void {
  state = { ...state, anchor };
}

/** Asks the draft (or `question`), with the chip riding on it. */
export async function ask(question?: string): Promise<BotTurnInfo | null> {
  const words = (question ?? state.draft).trim();
  if (!words || state.asking) return null;
  set({ asking: true, error: null, draft: question === undefined ? "" : state.draft });
  try {
    const turn = await botAsk({ question: words, scope: state.scope });
    set({ turns: [...state.turns, turn], asking: false, loaded: true });
    return turn;
  } catch (cause) {
    set({ asking: false, error: errorText(cause), draft: question === undefined ? words : state.draft });
    return null;
  }
}

/** New topic: follow-ups start fresh; nothing is deleted. */
export async function newTopic(): Promise<void> {
  try {
    const topic = await botNewTopic();
    set({ topics: merge(state.topics, [topic]), focusKey: state.focusKey + 1 });
  } catch (cause) {
    set({ error: errorText(cause) });
  }
}

/** Clears the conversation from this Mac. */
export async function clearConversation(): Promise<void> {
  await botClear();
  set({ turns: [], topics: [], more: false, find: null, anchor: null });
}

/** Find in the conversation: the matches, the newest current. */
export async function findWords(words: string): Promise<void> {
  if (!words.trim()) {
    set({ find: { words, matches: [], at: 0 } });
    return;
  }
  const found = await botFind(words);
  set({ find: { words, matches: found.map((f) => f.id), at: Math.max(0, found.length - 1) } });
  await reachMatch();
}

/** Loads earlier pages until the current match is shown. */
async function reachMatch(): Promise<void> {
  const find = state.find;
  const id = find?.matches[find.at];
  if (!id) return;
  let guard = 0;
  while (!state.turns.some((t) => t.id === id) && state.more && guard < 50) {
    guard += 1;
    await loadEarlier();
  }
}

/** ↑ ↓ between matches. */
export async function stepMatch(by: 1 | -1): Promise<void> {
  const find = state.find;
  if (!find || find.matches.length === 0) return;
  const at = (find.at + by + find.matches.length) % find.matches.length;
  set({ find: { ...find, at } });
  await reachMatch();
}

export function openFind(): void {
  set({ find: state.find ?? { words: "", matches: [], at: 0 } });
}

export function closeFind(): void {
  set({ find: null });
}
