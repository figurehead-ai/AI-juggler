/**
 * D-38 — which app's icon a row shows, as data (ADR-0019: never a branch
 * on a tool's id).
 *
 * - A session open in an app shows that app: its first place (the host
 *   registry's id, and the app's process for an app the registry doesn't
 *   list).
 * - Anything else shows its tool's own app, from the tool's spec
 *   (`host_app`: the Claude app for Claude Code, ChatGPT for Codex), with
 *   the tool's name for the letters when that app isn't installed. So a
 *   Codex session never shows another tool's mark.
 */
import type { InstanceInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";
import { toolSpec } from "./tools";

/** What `AppIcon` asks with, and the name its letters come from. */
export interface AppRef {
  appId: string | null;
  appPid: number | null;
  name: string;
}

/** Tool `tool`'s own app. */
export function appOfTool(tool: string): AppRef {
  const spec = toolSpec(tool);
  return { appId: spec.host_app ?? null, appPid: null, name: spec.display_name };
}

/** Where a conversation is open, else its tool's app. */
export function appOfConversation(conversation: ConversationInfo): AppRef {
  const place = conversation.live ? (conversation.places ?? [])[0] : undefined;
  if (place && (place.app_id || place.app_pid !== null)) return { appId: place.app_id ?? null, appPid: place.app_pid ?? null, name: place.app || toolSpec(conversation.tool).display_name };
  return appOfTool(conversation.tool);
}

/** One of AI Juggler's own sessions: its tool's app. */
export function appOfSession(session: Pick<InstanceInfo, "provider">): AppRef {
  return appOfTool(session.provider);
}
