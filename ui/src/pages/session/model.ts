/**
 * AJ-E2108 — one session, whichever started it: what its page says about
 * where it runs. A session AI Juggler started and one another app runs
 * read the same way here, so the header and the Overview are one
 * component for both (ADR-0019: no copy per kind).
 */
import type { ChipTone } from "../../components/primitives";
import type { ConversationInfo, PlaceInfo } from "../../lib/ipc/conversations";
import type { InstanceInfo } from "../../lib/bridge";
import { stateWord } from "../../components/OutsideConversations";
import { statusTone } from "../ItemHeader";
import { toolSpec } from "../../lib/tools";

export type SessionModel = {
  /** Its tool. */
  tool: string;
  /** Its profile (the stored account row) and that profile's name. */
  profileId: string;
  profileLabel: string;
  /** The tool's id for its conversation, when it has one yet. */
  conversationId: string | null;
  /** AI Juggler's session, when AI Juggler started it. */
  sessionId: string | null;
  /** Open now: a live process runs it. */
  live: boolean;
  /** The state in E21's words, its chip's tone and since when. */
  state: { words: string; tone: ChipTone; asks: boolean; since: number | null };
  /** Where it's open, one place per process. */
  places: PlaceInfo[];
  /** When the user last had it in front, as its app recorded it. */
  viewedMs: number | null;
  cwd: string | null;
  model: string | null;
};

const TONE: Record<string, ChipTone> = { asks_you: "warning", working: "live", idle: "neutral", done_review: "accent" };

/** A session another app runs (or AI Juggler's, seen in the tools' files). */
export function fromConversation(c: ConversationInfo): SessionModel {
  return {
    tool: c.tool,
    profileId: c.account_id,
    profileLabel: c.account_label,
    conversationId: c.id,
    sessionId: c.session_id,
    live: c.live,
    state: {
      words: stateWord(c),
      tone: (c.state ? TONE[c.state] : undefined) ?? (c.live ? "live" : "neutral"),
      asks: c.live && c.state === "asks_you",
      since: c.last_activity_ms,
    },
    places: c.places ?? [],
    viewedMs: c.viewed_ms ?? null,
    cwd: c.cwd,
    model: null,
  };
}

/** A session AI Juggler started; `seen` is its conversation as the tools'
 * files show it (where it's open, when it was viewed), when found. */
export function fromInstance(s: InstanceInfo, seen?: ConversationInfo): SessionModel {
  const tone = statusTone(s.status);
  const live = ["starting", "running", "waiting_user", "stuck", "quota_blocked"].includes(s.status);
  return {
    tool: s.provider,
    profileId: s.account_id ?? "",
    profileLabel: s.account_label,
    conversationId: s.conversation_id,
    sessionId: s.id,
    live,
    state: { words: tone.label, tone: tone.tone, asks: s.status === "waiting_user", since: s.last_activity_ms ?? s.started_at_ms },
    places: seen?.places ?? [],
    viewedMs: seen?.viewed_ms ?? null,
    cwd: s.cwd,
    model: s.running_model ?? s.model ?? null,
  };
}

/** The app a place is, in words: "the Claude app" for a tool's own desktop
 * app, else the app's name ("Terminal"). */
export function appWords(place: PlaceInfo | undefined, tool: string): string | null {
  if (!place || place.app === "") return null;
  return toolSpec(tool).desktop_app === place.app ? `the ${place.app} app` : place.app;
}

/** Whether a place is a tool's own desktop app (it has a sidebar of
 * sessions to pick from). */
export function isDesktopApp(place: PlaceInfo | undefined, tool: string): boolean {
  return place !== undefined && toolSpec(tool).desktop_app === place.app;
}

/** The tool's short name ("Claude"), for the turns' labels. */
export function speakerName(tool: string): string {
  const spec = toolSpec(tool);
  return spec.short_name || spec.display_name;
}
