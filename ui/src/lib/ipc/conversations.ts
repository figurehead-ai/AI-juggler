/**
 * AJ-E1529 — every account's conversations, the ones AI Juggler started
 * and the ones it didn't (ENGINEERING-GUIDE U3: the feature's calls). The
 * backend reads them from each tool's own files and says when they
 * change (`conversations://changed`).
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend, refused, type LaunchResult } from "../bridge";
import type { ConversationInfo } from "../../types/generated/ConversationInfo";
import type { GoToOutcomeInfo } from "../../types/generated/GoToOutcomeInfo";
import type { PlaceInfo } from "../../types/generated/PlaceInfo";
import type { LastMessagesInfo } from "../../types/generated/LastMessagesInfo";
import type { LastTurnInfo } from "../../types/generated/LastTurnInfo";
import type { TurnBlockInfo } from "../../types/generated/TurnBlockInfo";
import { PREVIEW_TOOLS } from "../../preview/tools";
import { placesConversations } from "../../preview/places";

export type { ConversationInfo, GoToOutcomeInfo, LastMessagesInfo, LastTurnInfo, PlaceInfo, TurnBlockInfo };

/** AJ-E2108: what a switch starts from, where a session id goes: AI
 * Juggler's session when it runs this conversation, else the conversation
 * itself by its account and id. The backend reads the same key
 * (`conversations::switch_source::parse_source_key`). */
export function switchSourceOf(conversation: Pick<ConversationInfo, "session_id" | "account_id" | "id">): string {
  return conversation.session_id ?? `conversation:${conversation.account_id}:${conversation.id}`;
}

const NOW = Date.now();

function placesPreview(): boolean {
  try {
    return sessionStorage.getItem("aij.preview.places") === "1";
  } catch {
    return false;
  }
}

/** The preview's conversations: two started outside AI Juggler (one
 * live); AJ-E2104's desktop with several places open when
 * `sessionStorage["aij.preview.places"]` is "1". A spec that seeds a fixture gets exactly its fixture's (none
 * unless it lists them), so the other captures stay as they were. */
function previewConversations(): ConversationInfo[] {
  const fixture = (window as unknown as { __AIJ_FIXTURE__?: { conversations?: ConversationInfo[] } }).__AIJ_FIXTURE__;
  if (fixture) return fixture.conversations ?? [];
  if (placesPreview()) return placesConversations(NOW);
  const base = {
    tool: PREVIEW_TOOLS[0]!.id,
    git_branch: "main",
    started_ms: NOW - 3_600_000,
    tokens: null,
    session_id: null,
    places: [] as PlaceInfo[],
    pid: null,
    state: null,
    viewed_ms: null, probably: false,
    background: false,
    inactive: false,
  };
  return [
    {
      ...base,
      id: "outside-1",
      account_id: "claude-work",
      account_label: "Work Max",
      project_id: "proj-ai-juggler",
      cwd: "/Users/you/Dev/ai-juggler",
      title: "Refactor the settings screen",
      last_activity_ms: NOW - 60_000,
      live: true,
      places: [{ app: "Claude", app_id: "claude-app", profile: "Work Max", instance: "Work", pid: 4242, kind: "desktop", app_pid: 4100, label: "Claude · Work" }],
      pid: 4242,
      state: "working",
      viewed_ms: NOW - 120_000,
    },
    {
      ...base,
      id: "outside-2",
      account_id: "claude-personal",
      account_label: "Personal",
      project_id: "proj-payments",
      cwd: "/Users/you/Dev/payments-api",
      title: "Add pagination to the orders endpoint",
      last_activity_ms: NOW - 5 * 3_600_000,
      live: false,
    },
  ];
}

export async function conversationsList(): Promise<ConversationInfo[]> {
  if (!hasLiveBackend()) return previewConversations();
  return invoke<ConversationInfo[]>("conversations_list");
}

/** One conversation by its account and id: one past the working set the
 * list holds (an old one opened from History). `null` when it's gone. */
export async function conversationsGet(accountId: string, conversationId: string): Promise<ConversationInfo | null> {
  if (!hasLiveBackend()) return previewConversations().find((c) => c.account_id === accountId && c.id === conversationId) ?? null;
  return invoke<ConversationInfo | null>("conversations_get", { accountId, conversationId });
}

/** The copyable line that resumes a conversation in a terminal. */
export async function conversationsResumeCommand(accountId: string, conversationId: string): Promise<string> {
  if (!hasLiveBackend()) return `cd ~/work && tool --resume ${conversationId}`;
  return invoke<string>("conversations_resume_command", { accountId, conversationId });
}

/** "Continue in AI Juggler": a resume launch on its account. */
export async function conversationsContinue(accountId: string, conversationId: string): Promise<LaunchResult> {
  if (!hasLiveBackend()) {
    const live = previewConversations().find((row) => row.id === conversationId)?.live;
    if (live) throw refused("it's open in another session now: close it there first, then continue here");
    return { session_id: `sess-${conversationId}`, pid: 0 };
  }
  return invoke<LaunchResult>("conversations_continue", { accountId, conversationId });
}

/** AJ-E2102: Go to it, for any session open now: its app forward by the
 * app's own process, then what the app allows; says what it reached. */
export async function conversationsGoTo(accountId: string, conversationId: string, pid: number | null = null): Promise<GoToOutcomeInfo> {
  if (!hasLiveBackend()) {
    const place = previewConversations().find((row) => row.id === conversationId)?.places[0];
    return place
      ? { outcome: `Brought ${place.label} forward; pick the session in its sidebar`, reached: "app" }
      : { outcome: "It isn't open now", reached: "none" };
  }
  return invoke<GoToOutcomeInfo>("conversations_go_to", { accountId, conversationId, pid });
}

/** AJ-E2104: a place's Open: brings the running app whose process is
 * `appPid` forward (never launches it), and says what it did. */
export async function conversationsBringForward(appPid: number): Promise<GoToOutcomeInfo> {
  if (!hasLiveBackend()) {
    const place = previewConversations().flatMap((row) => row.places).find((p) => p.app_pid === appPid);
    return place ? { outcome: `Brought ${place.label} forward`, reached: "app" } : { outcome: "That app isn't open now", reached: "none" };
  }
  return invoke<GoToOutcomeInfo>("conversations_bring_forward", { appPid });
}

/** What changed: `index` the files (new conversations, titles, folders,
 * counts), `live` only which are open and their states (every few
 * seconds with many open sessions). */
export type ConversationsChange = "index" | "live";

/** Called when the backend's list changed (read it again), with what changed. */
export function onConversationsChanged(handler: (change: ConversationsChange) => void): () => void {
  if (!hasLiveBackend()) return () => {};
  let stop: (() => void) | null = null;
  let disposed = false;
  void listen<ConversationsChange | null>("conversations://changed", (event) => handler(event.payload === "live" ? "live" : "index")).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}

/** What a page asks for (AJ-E2108): the last turns, the ones `before` a
 * page's start, or everything `from` one; `known` is the stamp it shows. */
export type LastMessagesAsk = { known?: string | null; before?: number | null; from?: number | null; max?: number };

/** A fixture's turn: the backend's shape, or speaker and text only. */
type SeededTurn = Partial<LastTurnInfo> & { speaker: string; text?: string };
const none = (state: string): LastMessagesInfo => ({ state, turns: [], redacted: 0, earlier: null, stamp: null });

/** The preview's last messages: the fixture's per conversation id (paged
 * by turn index, as the backend pages by offset), else a short exchange
 * ending in a question (a key in it shows redacted). */
function previewLastMessages(conversationId: string, ask: LastMessagesAsk): LastMessagesInfo {
  const w = window as unknown as {
    __AIJ_FIXTURE__?: { last_messages?: Record<string, { state: string; redacted: number; turns: SeededTurn[] }> };
    __AIJ_APP_SETTINGS__?: Record<string, string>;
  };
  // Off in Settings: nothing is read.
  if (w.__AIJ_APP_SETTINGS__?.show_last_messages === "0") return none("off");
  const fixture = w.__AIJ_FIXTURE__;
  const seeded = fixture?.last_messages?.[conversationId] ?? (fixture?.last_messages ? null : PREVIEW_TURNS);
  if (!seeded) return none("none");
  if (seeded.state !== "shown") return none(seeded.state);
  const all: LastTurnInfo[] = seeded.turns.map((turn) => ({
    speaker: turn.speaker,
    blocks: turn.blocks ?? [{ kind: "text", text: turn.text ?? "", steps: [], hidden: 0 }],
    at_ms: turn.at_ms ?? null,
    model: turn.model ?? null,
  }));
  const stamp = `preview:${all.length}`;
  if (ask.before == null && ask.known === stamp) return { ...none("unchanged"), stamp };
  const max = Math.min(Math.max(ask.max ?? 30, 1), 60);
  const end = ask.before != null ? Math.min(ask.before, all.length) : all.length;
  const start = ask.before == null && ask.from != null ? Math.min(ask.from, end) : Math.max(0, end - max);
  return { state: "shown", redacted: seeded.redacted, turns: all.slice(start, end), earlier: start > 0 ? start : null, stamp };
}

const PREVIEW_TURNS: { state: string; redacted: number; turns: SeededTurn[] } = {
  state: "shown",
  redacted: 1,
  turns: [
    { speaker: "assistant", text: "Seven panes are in, and every setting has a home. Two rows had no footer, so I wrote one for each." },
    { speaker: "you", text: "Good. Keep the old shortcuts working; the test key is •••" },
    { speaker: "assistant", text: "Should the Privacy pane stay last, or go after Notifications as in the design?" },
  ],
};

/** AJ-E2108: a session's last messages (up to 30 turns), read from the
 * end of its transcript now, redacted; kept nowhere. `before` pages back,
 * `from` follows along, and `known` answers "unchanged" without a read. */
export async function conversationsLastMessages(accountId: string, conversationId: string, ask: LastMessagesAsk = {}): Promise<LastMessagesInfo> {
  if (!hasLiveBackend()) return previewLastMessages(conversationId, ask);
  return invoke<LastMessagesInfo>("conversations_last_messages", {
    accountId,
    conversationId,
    known: ask.known ?? null,
    before: ask.before ?? null,
    from: ask.from ?? null,
    max: ask.max ?? null,
  });
}
