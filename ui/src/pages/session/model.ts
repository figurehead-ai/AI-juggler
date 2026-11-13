/**
 * AJ-E2108 — one session, whichever started it: what its page says about
 * where it runs. A session AI Juggler started and one another app runs
 * read the same way here, so the header and the Overview are one
 * component for both (ADR-0019: no copy per kind).
 */
import type { ChipTone } from "../../components/primitives";
import { switchSourceOf, type ConversationInfo, type PlaceInfo } from "../../lib/ipc/conversations";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { stateWord } from "../../components/OutsideConversations";
import { openingIn } from "../../lib/just-opened";
import { statusTone } from "../ItemHeader";
import { sessionState } from "../../lib/rail-items";
import { toolSpec } from "../../lib/tools";
import { waitingSince } from "../../lib/waiting";
import { endedWord } from "../../lib/ended-how";
import { statusLive } from "../../lib/session-process";
import { appName, desktopAppWord, desktopProfile, sessionAppWords as rowAppWords, type AppWords } from "../../lib/app-name";

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
  /** AJ-E2108: what a switch to another profile starts from: AI Juggler's
   * session, or the session another app runs (its key). */
  switchSource: string;
  /** Open now: a live process runs it. */
  live: boolean;
  /** The state in E21's words, its chip's tone and since when; whether
   * it waits on you, and whether it is running a turn now. */
  state: { words: string; tone: ChipTone; asks: boolean; working: boolean; since: number | null };
  /** Where it's open, one place per process. */
  places: PlaceInfo[];
  /** When the user last had it in front, as its app recorded it. */
  viewedMs: number | null;
  cwd: string | null;
  model: string | null;
  /** AI Juggler started it in a tool's own desktop app (not a terminal). */
  desktop?: boolean;
  /** 0.4.1: a switch just opened it in this app, not reported open yet. */
  opening?: string | null;
  /** 0.4.1: its running app lists it, with no turn running now. */
  inactive?: boolean;
  /** The desktop app its tool's files say it was started in (`claude-app`). */
  startedIn?: string | null;
  /** rc22 (A3): the desktop App AI Juggler launched it in, stored at launch. */
  appId?: string | null;
  /** ADR-0032 (D-100): the App the backend says it belongs to (a
   * conversation's `app`): its chip names that one, nothing else does. */
  owner?: string | null;
  /** rc30: why it ended, when its ending says ("AI Juggler quit while it
   * ran", "exit code 1"): beside its state chip. */
  endedWhy?: string | null;
};

const TONE: Record<string, ChipTone> = { asks_you: "warning", working: "live", idle: "neutral", done_review: "accent" };

/** 0.4.1: where one its running app lists, with no turn running now, is. */
export const INACTIVE_WHERE = "Listed in its app, no turn running now";

/** A session another app runs (or AI Juggler's, seen in the tools' files). */
export function fromConversation(c: ConversationInfo): SessionModel {
  const model = asReported(c);
  // 0.4.1 (the rc8 walk): one its running app lists but no turn runs in is
  // Inactive, the rail's word, never Ended: a Claude app conversation runs
  // a process only while it works on a turn.
  if (!c.live && c.inactive) return { ...model, inactive: true, state: { ...model.state, words: "Inactive", tone: "neutral" } };
  // 0.4.1 (the rc7 walk): a copy a switch just opened reads Starting until
  // its app lists it or runs it, never Ended.
  const opening = c.live ? null : openingIn(c.account_id, c.id);
  return opening === null ? model : { ...model, opening, state: { ...model.state, words: "Starting", tone: "live" } };
}

/** A conversation's model as its app's files report it. */
function asReported(c: ConversationInfo): SessionModel {
  return {
    tool: c.tool,
    profileId: c.account_id,
    profileLabel: c.account_label,
    conversationId: c.id,
    sessionId: c.session_id,
    startedIn: c.started_in,
    owner: c.app ?? null,
    switchSource: switchSourceOf(c),
    live: c.live,
    state: {
      words: stateWord(c),
      tone: (c.state ? TONE[c.state] : undefined) ?? (c.live ? "live" : "neutral"),
      asks: c.live && c.state === "asks_you",
      working: c.live && c.state === "working",
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
  const rail = sessionState(s);
  const live = statusLive(s.status);
  return {
    tool: s.provider,
    profileId: s.account_id ?? "",
    profileLabel: s.account_label,
    conversationId: s.conversation_id,
    sessionId: s.id,
    switchSource: s.id,
    live,
    // The rail's one vocabulary (AJ-E2104): a running session between
    // turns is Idle, as its row says (the P4 walk: the header said Working
    // beside an Idle row).
    state: live
      ? // AJ-E3307: one that waits on you says since it began to wait,
        // as Home and the rail do (it said "2 min" from its last report).
        { words: rail.word, tone: TONE[rail.state] ?? tone.tone, asks: rail.needs, working: rail.state === "working", since: rail.needs ? waitingSince({ session: s }) : (s.last_activity_ms ?? s.started_at_ms) }
      : // D-96 C: an ended one says how it ended, when its signal said.
        { words: s.ending ? endedWord(s.ending.how) : tone.label, tone: tone.tone, asks: false, working: false, since: s.last_activity_ms ?? s.started_at_ms },
    places: seen?.places ?? [],
    viewedMs: seen?.viewed_ms ?? null,
    cwd: s.cwd,
    model: s.running_model ?? s.model ?? null,
    desktop: s.kind === "desktop",
    appId: s.app_id ?? null,
    endedWhy: live ? null : (s.ending?.detail ?? null),
  };
}

/** The app a place is, in words: "the Claude app" for a tool's own desktop
 * app, else the app's name ("Terminal"). */
export function appWords(place: PlaceInfo | undefined, tool: string): string | null {
  if (!place || place.app === "") return null;
  return toolSpec(tool).desktop_app === place.app ? `the ${place.app} app` : place.app;
}

/** D-62: the session's App in words, built as the rail builds it: a
 * desktop app in its profile ("Claude app · Work Max"), else its tool on
 * its folder ("Claude Code · Work Max"). The terminal it runs in is never
 * part of it (`sessionHost`). */
export function sessionAppName(model: SessionModel): string {
  const words = sessionAppWords(model);
  return appName(words.app, words.profile);
}

/** A session's App in its two parts: its app and its profile
 * ([`sessionAppName`]). */
export function sessionAppWords(model: SessionModel): AppWords {
  const place = model.places[0];
  if (place && place.app !== "" && (place.kind === "desktop" || isDesktopApp(place, model.tool))) return { app: desktopAppWord(place.app, model.tool, place.shown), profile: desktopProfile(place.instance, place.profile) };
  return rowAppWords({ kind: model.desktop ? "desktop" : "terminal", provider: model.tool, account_label: model.profileLabel });
}

/** Where a CLI session's window is: its terminal or editor ("Terminal"),
 * or AI Juggler's own terminal; `null` for a desktop App's session (its
 * window is the App) or when no place is known. */
export function sessionHost(model: SessionModel): string | null {
  const place = model.places[0];
  if (place && place.app !== "") return place.kind === "desktop" || isDesktopApp(place, model.tool) ? null : place.app;
  if (model.desktop) return null;
  return model.sessionId && model.live ? "AI Juggler's terminal" : null;
}

/** Whether a place is a tool's own desktop app (it has a sidebar of
 * sessions to pick from). */
export function isDesktopApp(place: PlaceInfo | undefined, tool: string): boolean {
  return place !== undefined && toolSpec(tool).desktop_app === place.app;
}

/** Whether this session may still need picking in its app's sidebar once
 * the app is in front: a tool's own app that Go to it can't open the
 * session in by a link (the Codex app's thread link opens it). */
export function picksInSidebar(place: PlaceInfo | undefined, tool: string): boolean {
  return isDesktopApp(place, tool) && !toolSpec(tool).supports.thread_link;
}

/** The tool's short name ("Claude"), for the turns' labels. */
export function speakerName(tool: string): string {
  const spec = toolSpec(tool);
  return spec.short_name || spec.display_name;
}
