/**
 * AJ-E2104, AJ-E2107 — the rail's one list. AI Juggler's own sessions and
 * the sessions other apps run, each as one item with:
 *
 * - one state in E21's words: Asks you, Working, Idle, Done (review it),
 *   Ended. Every tool's and app's status maps onto them here, and a state
 *   worked out rather than recorded says "probably" (D-37);
 * - its place: the app and profile it runs in (AI Juggler's own sessions
 *   are the place "AI Juggler");
 * - why it is in Needs you, when it is.
 *
 * Pure, so the order, fold and grouping rules are unit-tested.
 */
import type { InstanceInfo, ProjectInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";
import { conversationTitle } from "./conversations";

export type RailState = "asks_you" | "working" | "idle" | "done_review" | "ended";

/** Why an item is in Needs you. */
export type Need = "asks" | "limit" | "stuck" | "failed" | "review";

/** The place AI Juggler's own sessions are in. */
export const OWN_PLACE = "ai-juggler";
export const OWN_APP = "AI Juggler";

export interface RailPlace {
  /** App and profile (or the app's instance): one place. */
  key: string;
  app: string;
  /** D-38: the host registry's id for the app (its icon), `OWN_PLACE`
   * for AI Juggler's own; `null` when the registry doesn't know it. */
  appId: string | null;
  /** `desktop`, `terminal`, `editor`, `own` (AI Juggler), or unknown. */
  kind: string | null;
  /** The profile it runs in, in words. */
  profile: string | null;
  /** The app's process, which the place's Open brings forward. */
  appPid: number | null;
}

export interface RailItem {
  /** `s:<session>` or `c:<account>/<conversation>`. */
  key: string;
  session: InstanceInfo | null;
  conversation: ConversationInfo | null;
  title: string;
  projectId: string | null;
  project: string | null;
  place: RailPlace;
  /** The other places it's open in (one row that says so). */
  alsoIn: string[];
  accountId: string | null;
  accountLabel: string;
  tool: string;
  state: RailState;
  /** Worked out, not recorded: the word says "probably". */
  probably: boolean;
  need: Need | null;
  lastMs: number;
  viewedMs: number | null;
  live: boolean;
  background: boolean;
  inactive: boolean;
  archived: boolean;
  /** Helpers it runs (sub-agents), when known. */
  subagents: number;
  /** AJ-E1502: the user pinned it (an AI Juggler session's own pin, or
   * the rail's pin of one another app runs). Pinned rows lead their group. */
  pinned: boolean;
  /** When it started: the order it was first seen in (Your order, D-45). */
  startMs: number;
}

const WORDS: Record<RailState, string> = {
  asks_you: "Asks you",
  working: "Working",
  idle: "Idle",
  done_review: "Done, review it",
  ended: "Ended",
};

/** The state in words: "Asks you", "Probably asks you". */
export function stateWord(state: RailState, probably = false): string {
  return probably ? `Probably ${WORDS[state].toLowerCase()}` : WORDS[state];
}

/** Why it needs you, in a few words — the reason line ("Limit reached",
 * "Probably stuck") — else its state in words. One wording for every row
 * that shows a session's state (AJ-E2104 Req.3). */
export function needWord(item: Pick<RailItem, "need" | "state" | "probably">): string {
  switch (item.need) {
    case "limit":
      return "Limit reached";
    case "failed":
      return "Stopped with an error";
    case "stuck":
      return "Probably stuck";
    case "review":
      return stateWord("done_review");
    default:
      return stateWord(item.state, item.probably);
  }
}

/** The state's colour token (text and dot). */
export const STATE_TONE: Record<RailState, { text: string; dot: string }> = {
  asks_you: { text: "text-warning", dot: "bg-warning" },
  working: { text: "text-live", dot: "bg-live" },
  idle: { text: "text-muted", dot: "bg-faint" },
  done_review: { text: "text-success", dot: "bg-success" },
  ended: { text: "text-muted", dot: "bg-faint" },
};

const ENDED = new Set(["finished", "stopped", "failed", "handed_over"]);
/** A status that is itself a need (as Home has always read it). */
const NEED_STATUSES = new Set(["waiting_user", "stuck", "quota_blocked", "failed"]);
const ACTIVE = new Set(["running", "starting"]);

function folderOf(cwd: string | null): string {
  return (cwd ?? "").replace(/\/+$/, "").split("/").pop() ?? "";
}

/** AI Juggler's own session: Needs you from its attention (a snoozed one
 * waits in its place), Working while it runs, Ended after. */
export function sessionItem(row: InstanceInfo, projectName: (id: string | null) => string | null, snoozed: boolean): RailItem {
  const raw = row.needs_attention ?? (NEED_STATUSES.has(row.status) ? (row.status as NonNullable<InstanceInfo["needs_attention"]>) : null);
  const attention = snoozed ? null : raw;
  const need: Need | null =
    attention === "waiting_user" ? "asks" : attention === "quota_blocked" ? "limit" : attention === "stuck" ? "stuck" : attention === "failed" ? "failed" : null;
  const ended = ENDED.has(row.status);
  const guessed: RailState = need === "asks" || need === "stuck" || need === "limit" ? "asks_you" : ended ? "ended" : ACTIVE.has(row.status) ? "working" : "idle";
  // AJ-E2302/E2306: the tool's own word (its protocol or its hooks), never a guess.
  const exact = row.runtime?.state as RailState | null | undefined;
  const state: RailState = !snoozed && exact && exact in WORDS ? exact : guessed;
  return {
    key: `s:${row.id}`,
    session: row,
    conversation: null,
    title: row.label ?? row.git_branch ?? (folderOf(row.cwd) || row.account_label),
    projectId: row.project_id,
    project: projectName(row.project_id),
    place: { key: OWN_PLACE, app: OWN_APP, appId: OWN_PLACE, kind: "own", profile: null, appPid: null },
    alsoIn: [],
    accountId: row.account_id,
    accountLabel: row.account_label,
    tool: row.provider,
    state,
    // "Looks stuck" is the supervisor's guess.
    probably: need === "stuck",
    need,
    lastMs: row.last_activity_ms ?? row.started_at_ms,
    viewedMs: null,
    live: ACTIVE.has(row.status) || attention !== null,
    background: false,
    inactive: false,
    archived: row.lifecycle === "archived",
    subagents: row.subtask_count ?? row.child_count ?? 0,
    pinned: row.pinned === true,
    startMs: row.started_at_ms,
  };
}

/** An AI Juggler session's state for a row outside the rail by place (the
 * Project lens, a page's list): the state, its word or the reason it
 * needs you, and the word's colour — the place rows' wording. */
export function sessionState(row: InstanceInfo): { state: RailState; word: string; tone: string; dot: string; needs: boolean } {
  const item = sessionItem(row, () => null, false);
  const needs = item.need !== null;
  return { state: item.state, word: needWord(item), tone: needs ? "text-warning" : STATE_TONE[item.state].text, dot: STATE_TONE[item.state].dot, needs };
}

/** A status alone (a paired Mac's row: no attention record) as the
 * rail's state and words. */
export function statusState(status: string): { state: RailState; word: string } {
  const need: Need | null = status === "waiting_user" ? "asks" : status === "quota_blocked" ? "limit" : status === "stuck" ? "stuck" : status === "failed" ? "failed" : null;
  const state: RailState = need === "asks" || need === "stuck" || need === "limit" ? "asks_you" : ENDED.has(status) ? "ended" : ACTIVE.has(status) ? "working" : "idle";
  return { state, word: needWord({ need, state, probably: need === "stuck" }) };
}

/** A place's key: the app, then its instance or the profile. */
function placeKey(app: string, instance: string | null, profile: string | null): string {
  return `${app}|${instance ?? profile ?? ""}`;
}

/** A session another app runs. "Done, review it" clears once the user has
 * looked: AI Juggler's seen mark, or the app's own last view, after its
 * last activity (AJ-E2107). `pinned`: the rail's pin (AI Juggler's own
 * record; the app's files are never written). */
export function conversationItem(c: ConversationInfo, projectName: (id: string | null) => string | null, seenMs: number | null, pinned = false): RailItem {
  const places = c.places ?? [];
  const first = places[0];
  const last = c.last_activity_ms ?? 0;
  const looked = Math.max(seenMs ?? 0, c.viewed_ms ?? 0);
  const raw = c.live ? c.state : null;
  const state: RailState = !c.live
    ? "ended"
    : raw === "asks_you"
      ? "asks_you"
      : raw === "working"
        ? "working"
        : raw === "done_review" && looked < last
          ? "done_review"
          : "idle";
  const app = first?.app || "An app";
  return {
    key: `c:${c.account_id}/${c.id}`,
    session: null,
    conversation: c,
    title: conversationTitle(c),
    projectId: c.project_id,
    project: projectName(c.project_id) ?? (folderOf(c.cwd) || null),
    place: first
      ? { key: placeKey(app, first.instance, first.profile), app, appId: first.app_id ?? null, kind: first.kind ?? null, profile: first.profile ?? first.instance, appPid: first.app_pid ?? null }
      : { key: placeKey(app, null, c.account_label), app, appId: null, kind: null, profile: c.account_label, appPid: null },
    alsoIn: places.slice(1).map((place) => place.label || place.app),
    accountId: c.account_id,
    accountLabel: c.account_label,
    tool: c.tool,
    state,
    probably: state === "asks_you" && c.probably === true,
    need: state === "asks_you" ? "asks" : state === "done_review" ? "review" : null,
    lastMs: last,
    viewedMs: c.viewed_ms,
    live: c.live,
    background: c.live && c.background,
    inactive: !c.live && c.inactive,
    archived: false,
    subagents: 0,
    pinned,
    startMs: c.started_ms ?? last,
  };
}

/** Where Go to it takes it, in words ("Claude · Work"), when that is
 * another app; `null` for a session whose own place is AI Juggler (its
 * terminal is its page here: the row already goes there). */
export function goToWhere(item: Pick<RailItem, "place" | "accountLabel" | "session">): string | null {
  const desktop = item.session?.kind === "desktop";
  if (item.place.key === OWN_PLACE && !desktop) return null;
  const app = desktop ? "its app" : item.place.app;
  return [app, item.place.profile ?? item.accountLabel].filter(Boolean).join(" · ");
}

export interface ItemInputs {
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  projects: ProjectInfo[];
  seen: Record<string, number>;
  snoozed?: (row: InstanceInfo) => boolean;
  /** The rail's pins of sessions other apps run, by key (`lib/rail-order`). */
  pins?: ReadonlySet<string>;
}

/** Every item: AI Juggler's sessions, then the sessions it didn't start. */
export function itemsOf({ sessions, conversations, projects, seen, snoozed = () => false, pins }: ItemInputs): RailItem[] {
  const names = new Map(projects.map((p) => [p.id, p.name]));
  const projectName = (id: string | null) => (id === null ? null : (names.get(id) ?? null));
  return [
    ...sessions.map((row) => sessionItem(row, projectName, snoozed(row))),
    ...conversations
      .filter((c) => c.session_id === null)
      .map((c) => {
        const key = `c:${c.account_id}/${c.id}`;
        return conversationItem(c, projectName, seen[key] ?? null, pins?.has(key) === true);
      }),
  ];
}

const NEED_ORDER: Need[] = ["asks", "limit", "failed", "stuck", "review"];

/** Needs you: what asks first, then limits, failures, stuck, things to
 * review; the longest waiting first within each. */
export function needsYou(items: RailItem[]): RailItem[] {
  return items
    .filter((item) => item.need !== null)
    .sort((a, b) => NEED_ORDER.indexOf(a.need!) - NEED_ORDER.indexOf(b.need!) || a.lastMs - b.lastMs || a.key.localeCompare(b.key));
}

/** Open now: open and interactive. A session that needs you stays in its
 * place here too, its state in amber; Needs you lists it as well, as a
 * shortcut (D-45, which replaced "not in Needs you"). A pinned session
 * stays where the user put it, under its place, once it has ended too
 * (AJ-E1502; Ended today leaves it out, so it is never in neither). */
export function openNow(item: RailItem): boolean {
  if (item.pinned) return true;
  return item.live && !item.background && item.state !== "ended";
}

/** The start of `now`'s day, local time. */
export function startOfDay(now: number): number {
  const day = new Date(now);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/** Ended today (archived ones only when asked for). */
export function endedToday(item: RailItem, now: number, archived = false): boolean {
  if (item.need !== null || item.inactive) return false;
  if (item.archived && !archived) return false;
  if (item.pinned) return false;
  return item.state === "ended" && item.lastMs >= startOfDay(now);
}
