/**
 * AJ-E1537, AJ-E2106 — the actions of sessions: one AI Juggler runs, an
 * ended one (History), and one another app started (a conversation).
 * Groups and ranks follow the approved menu: getting there (Go to it,
 * Open its page, Show its project), working on it, stopping last.
 */
import type { InstanceInfo } from "./bridge";
import type { ActionHost } from "./action-host";
import type { ActionSpec } from "./action-types";

/** The statuses of a session that is still going. */
const LIVE = new Set(["starting", "running", "waiting_user", "stuck", "quota_blocked"]);
export const isLive = (row: InstanceInfo): boolean => LIVE.has(row.status);

export const has = (fn: unknown): boolean => typeof fn === "function";
const notSnoozed = (row: InstanceInfo, h: ActionHost) => has(h.snoozeSession) && !h.isSnoozed?.(row);

export const SESSION: ActionSpec<"session">[] = [
  // Getting there. D-37: the app's own window, or (one AI Juggler runs)
  // its terminal here. An ended session has nowhere to go: left out.
  { id: "focus", label: "Go to it", icon: "go-to", group: "go", rank: 10, shortcut: "row-go-to", applies: (row, h) => has(h.focusSession) && isLive(row), run: (row, h) => h.focusSession!(row) },
  { id: "open", label: "Open its page", icon: "page", group: "go", rank: 20, shortcut: "row-open", applies: (_, h) => has(h.openSession), run: (row, h) => h.openSession!(row) },
  {
    id: "reveal-project",
    label: "Show its project",
    icon: "project",
    group: "go",
    rank: 30,
    applies: (row, h) => has(h.revealSessionProject) && row.project_id !== null,
    run: (row, h) => h.revealSessionProject!(row),
  },
  // Working on it. D-05: "Continue on another account…" stays the name.
  {
    id: "continue",
    label: "Continue on another account…",
    icon: "carry-over",
    group: "work",
    rank: 10,
    shortcut: "handover",
    applies: (_, h) => has(h.continueSession),
    blocked: (row) => (row.conversation_id ? null : "It has no conversation to carry over yet"),
    run: (row, h) => h.continueSession!(row),
  },
  { id: "rename", label: "Rename…", icon: "rename", group: "work", rank: 30, applies: (_, h) => has(h.renameSession), run: (row, h) => h.renameSession!(row) },
  // Shows instead of the snoozes, where they would be.
  {
    id: "unsnooze",
    label: "Unsnooze / unmute",
    icon: "wake",
    group: "work",
    rank: 39,
    applies: (row, h) => has(h.unsnoozeSession) && h.isSnoozed?.(row) === true,
    run: (row, h) => h.unsnoozeSession!(row),
  },
  { id: "snooze-60", label: "Snooze for an hour", icon: "snooze", group: "work", rank: 40, applies: notSnoozed, run: (row, h) => h.snoozeSession!(row, 60 * 60_000) },
  {
    id: "snooze-15",
    label: "For 15 minutes",
    icon: "snooze",
    group: "work",
    rank: 41,
    submenu: "snooze",
    applies: notSnoozed,
    run: (row, h) => h.snoozeSession!(row, 15 * 60_000),
  },
  {
    id: "snooze-reset",
    label: "Until the limit resets",
    icon: "snooze",
    group: "work",
    rank: 42,
    submenu: "snooze",
    applies: notSnoozed,
    blocked: (row) => (row.quota?.resets_at_ms != null ? null : "No known reset time for this quota window"),
    run: (row, h) => h.snoozeSession!(row, null),
  },
  {
    id: "mute",
    label: "Mute this session",
    icon: "mute",
    group: "work",
    rank: 43,
    submenu: "snooze",
    applies: (row, h) => has(h.muteSession) && !h.isSnoozed?.(row),
    run: (row, h) => h.muteSession!(row),
  },
  {
    id: "copy-resume",
    label: "Copy the resume command",
    icon: "copy",
    group: "work",
    rank: 50,
    applies: (_, h) => has(h.copyResumeCommand),
    blocked: (row) => (row.conversation_id ? null : "It has no conversation to resume yet"),
    run: (row, h) => h.copyResumeCommand!(row),
  },
  {
    id: "pin",
    label: (row) => (row.pinned ? "Unpin" : "Pin to top"),
    icon: "pin",
    group: "work",
    rank: 60,
    applies: (_, h) => has(h.pinSession),
    run: (row, h) => h.pinSession!(row),
  },
  // Stopping. D-08: only a live session AI Juggler started has Stop.
  {
    id: "stop",
    label: "Stop this session",
    icon: "stop",
    group: "stop",
    rank: 10,
    shortcut: "stop",
    applies: (row, h) => has(h.stopSession) && isLive(row),
    run: (row, h) => h.stopSession!(row),
  },
];

export const HISTORY: ActionSpec<"history">[] = [
  { id: "open", label: "Open its page", icon: "page", group: "go", rank: 10, shortcut: "row-open", applies: (_, h) => has(h.openHistory), run: (row, h) => h.openHistory!(row) },
  {
    id: "continue",
    label: "Continue on another account…",
    icon: "carry-over",
    group: "work",
    rank: 10,
    applies: (_, h) => has(h.continueHistory),
    blocked: (row) => (row.conversation_id ? null : "It has no conversation to carry over"),
    run: (row, h) => h.continueHistory!(row),
  },
  { id: "replay", label: "Replay", icon: "replay", group: "work", rank: 20, applies: (_, h) => has(h.replay), run: (row, h) => h.replay!(row) },
  {
    id: "archive",
    label: (row, h) => (h.isArchived?.(row) ? "Unarchive" : "Archive"),
    icon: "archive",
    group: "work",
    rank: 30,
    applies: (_, h) => has(h.archive) && has(h.unarchive),
    run: (row, h) => (h.isArchived?.(row) ? h.unarchive!(row) : h.archive!(row)),
  },
  { id: "delete", label: "Delete…", icon: "delete", group: "stop", rank: 10, applies: (_, h) => has(h.deleteHistory), run: (row, h) => h.deleteHistory!(row) },
];

// D-08: one AI Juggler didn't start is read-only; it can be carried in
// ("Continue in AI Juggler"), never stopped or tidied from here.
export const CONVERSATION: ActionSpec<"conversation">[] = [
  // AJ-E2102: getting there first, while it's open somewhere.
  { id: "go-to", label: "Go to it", icon: "go-to", group: "go", rank: 10, shortcut: "row-go-to", applies: (c, h) => has(h.goToConversation) && c.live, run: (c, h) => h.goToConversation!(c) },
  { id: "open", label: "Open its page", icon: "page", group: "go", rank: 20, shortcut: "row-open", applies: (_, h) => has(h.openConversation), run: (c, h) => h.openConversation!(c) },
  {
    id: "reveal-project",
    label: "Show its project",
    icon: "project",
    group: "go",
    rank: 30,
    applies: (c, h) => has(h.openProjectById) && c.project_id !== null,
    run: (c, h) => h.openProjectById!(c.project_id!),
  },
  {
    id: "continue",
    label: "Continue in AI Juggler",
    icon: "carry-in",
    group: "work",
    rank: 10,
    applies: (_, h) => has(h.continueConversation),
    blocked: (c) => (c.live ? "It's open in another session now" : null),
    run: (c, h) => h.continueConversation!(c),
  },
  {
    id: "copy-resume",
    label: "Copy the resume command",
    icon: "copy",
    group: "work",
    rank: 50,
    applies: (_, h) => has(h.copyConversationResume),
    run: (c, h) => h.copyConversationResume!(c),
  },
];
