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
import { PREVIEW_TOOLS } from "../../preview/tools";
import { placesConversations } from "../../preview/places";

export type { ConversationInfo, GoToOutcomeInfo, PlaceInfo };

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

/** Called when the backend's list changed (no payload: read it again). */
export function onConversationsChanged(handler: () => void): () => void {
  if (!hasLiveBackend()) return () => {};
  let stop: (() => void) | null = null;
  let disposed = false;
  void listen("conversations://changed", () => handler()).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}
