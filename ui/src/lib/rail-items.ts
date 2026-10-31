/**
 * AJ-E2104, AJ-E2107 — the rail's one list. AI Juggler's own sessions and
 * the sessions other apps run, each as one item with:
 *
 * - one state in E21's words: Asks you, Working, Idle, Done (review it),
 *   Ended. Every tool's and app's status maps onto them here, and a state
 *   worked out rather than recorded says "probably" (D-37);
 * - its place: the App it runs in (D-62): a desktop app in one of its
 *   profiles ("Claude app · b-work"), or a CLI tool on its folder
 *   ("Claude Code · Work Max"), whose terminal (or AI Juggler, for its own
 *   sessions) is where its window is, never its name;
 * - why it is in Needs you, when it is.
 *
 * Pure, so the order, fold and grouping rules are unit-tested.
 */
import type { InstanceInfo, ProjectInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";
import { conversationTitle } from "./conversations";
import { appOfTool } from "./app-of";
import { cliAppKey, desktopAppKey, desktopAppWord, hostWords } from "./app-name";
import { toolSpec } from "./tools";

export type RailState = "asks_you" | "working" | "idle" | "done_review" | "ended";

/** Why an item is in Needs you. */
export type Need = "asks" | "limit" | "stuck" | "failed" | "review";

/** AI Juggler itself: the `kind` of a place whose window is AI Juggler's
 * own terminal, and the id its icon is asked with. */
export const OWN_PLACE = "ai-juggler";
export const OWN_APP = "AI Juggler";
/** A place whose app isn't named (a session no app shows now). */
export const SOME_APP = "An app";

export interface RailPlace {
  /** The App (D-62): a CLI tool and its folder, or a desktop app and its
   * instance. */
  key: string;
  /** The App's app: "Claude Code", "Claude app", "ChatGPT". */
  app: string;
  /** D-38: the icon's id: the desktop app's, or a CLI tool's own app;
   * `null` when the registry doesn't know it. */
  appId: string | null;
  /** Where its window is: `desktop`, `terminal`, `editor`, `own` (AI
   * Juggler's terminal), or unknown. */
  kind: string | null;
  /** The App's profile, in words: its folder's label, or the app's
   * instance. */
  profile: string | null;
  /** The process the place's Open brings forward: the desktop app, or the
   * terminal or editor a CLI session is in. */
  appPid: number | null;
  /** Where a CLI App's window is ("Terminal", "AI Juggler"); `null` for a
   * desktop App. A detail of the session, never the App's name. */
  host: string | null;
}

/** A place in words: its App, then where its window is when that's a
 * terminal or editor ("Claude Code · Work Max · in Terminal"). AI
 * Juggler's own terminal isn't said: you're in it. */
export function placeWords(place: RailPlace): string[] {
  return [place.app, place.profile, place.kind === "own" ? null : hostWords(place.host)].filter((part): part is string => Boolean(part));
}

/** The CLI App tool `tool` runs on the folder `accountId` (labelled
 * `folder`), its window in `host`. */
export function cliPlace(tool: string, accountId: string | null, folder: string, host: { name: string | null; kind: string | null; pid: number | null }): RailPlace {
  return { key: cliAppKey(tool, accountId, folder), app: toolSpec(tool).display_name, appId: appOfTool(tool).appId, kind: host.kind, profile: folder || null, appPid: host.pid, host: host.name };
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
  /** Sub-agents it runs now, when read (AJ-E1520: its tool's subagent
   * reader, the one its Subagents tab reads, for AI Juggler's sessions
   * and the ones other apps run alike; never its child processes). */
  subagents: number;
  /** AJ-E1502: the user pinned it (an AI Juggler session's own pin, or
   * the rail's pin of one another app runs). Pinned rows lead their group. */
  pinned: boolean;
  /** When it started: the order it was first seen in (Your order, D-47). */
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
    place:
      row.kind === "desktop"
        ? { key: desktopAppKey(toolSpec(row.provider).host_app ?? null, row.provider, row.account_label), app: desktopAppWord(toolSpec(row.provider).desktop_app ?? "", row.provider) || toolSpec(row.provider).display_name, appId: toolSpec(row.provider).host_app ?? null, kind: "own", profile: row.account_label, appPid: null, host: null }
        : cliPlace(row.provider, row.account_id, row.account_label, { name: OWN_APP, kind: "own", pid: null }),
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
    subagents: row.subtask_count ?? 0,
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
  return {
    key: `c:${c.account_id}/${c.id}`,
    session: null,
    conversation: c,
    title: conversationTitle(c),
    projectId: c.project_id,
    project: projectName(c.project_id) ?? (folderOf(c.cwd) || null),
    place:
      first?.kind === "desktop"
        ? { key: desktopAppKey(first.app_id ?? null, first.app, first.instance), app: desktopAppWord(first.app, c.tool), appId: first.app_id ?? null, kind: "desktop", profile: first.instance ?? null, appPid: first.app_pid ?? null, host: null }
        : cliPlace(c.tool, c.account_id, c.account_label, { name: first?.app || null, kind: first?.kind ?? null, pid: first?.app_pid ?? null }),
    alsoIn: places.slice(1).map((place) => place.label || place.app),
    accountId: c.account_id,
    accountLabel: c.account_label,
    tool: c.tool,
    state,
    probably: state === "asks_you" && c.probably === true,
    need: state === "asks_you" ? "asks" : state === "done_review" ? "review" : null,
    lastMs: last,
    // A payload without the field (an older reader's) reads as never
    // viewed: "viewed NaN d ago" in the 0.2.0 re-render's replica rows.
    viewedMs: c.viewed_ms ?? null,
    live: c.live,
    background: c.live && c.background,
    inactive: !c.live && c.inactive,
    archived: false,
    subagents: c.subtask_count ?? 0,
    pinned,
    startMs: c.started_ms ?? last,
  };
}

/** Where Go to it takes it, in words: the desktop App ("Claude app ·
 * b-work"), or the terminal a CLI session is in ("Terminal"); `null` for
 * a session in AI Juggler's own terminal (its page is here: the row
 * already goes there). */
export function goToWhere(item: Pick<RailItem, "place" | "accountLabel" | "session">): string | null {
  if (item.session?.kind === "desktop") return ["its app", item.place.profile ?? item.accountLabel].filter(Boolean).join(" · ");
  if (item.place.kind === "own") return null;
  if (item.place.host) return item.place.host;
  return [item.place.app, item.place.profile].filter(Boolean).join(" · ") || null;
}

export interface ItemInputs {
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  projects: ProjectInfo[];
  seen: Record<string, number>;
  snoozed?: (row: InstanceInfo) => boolean;
  /** The rail's pins of sessions other apps run, by key (`lib/rail-order`). */
  pins?: ReadonlySet<string>;
  /** D-49: the backend's Needs you, by item key (its word decides). */
  needOf?: ReadonlyMap<string, Need>;
}

/** Every item: AI Juggler's sessions, then the sessions it didn't start. */
export function itemsOf({ sessions, conversations, projects, seen, snoozed = () => false, pins, needOf }: ItemInputs): RailItem[] {
  const names = new Map(projects.map((p) => [p.id, p.name]));
  const projectName = (id: string | null) => (id === null ? null : (names.get(id) ?? null));
  const items = [
    ...sessions.map((row) => sessionItem(row, projectName, snoozed(row))),
    ...conversations
      .filter((c) => c.session_id === null)
      .map((c) => {
        const key = `c:${c.account_id}/${c.id}`;
        return conversationItem(c, projectName, seen[key] ?? null, pins?.has(key) === true);
      }),
  ];
  // D-49: what is in Needs you is the backend's (one list, one count).
  if (!needOf) return items;
  return items.map((item) => {
    const need = needOf.get(item.key) ?? null;
    if (need === item.need) return item;
    // The row's word follows the need (the 0.2 walk, F4: "Done, review it"
    // on Home and in the centre, "Working" on its rail row).
    const state: RailState = need === null ? item.state : (NEED_STATE[need] ?? item.state);
    return { ...item, need, state, probably: item.session ? need === "stuck" : item.probably, live: item.live || (item.session !== null && need !== null) };
  });
}

const NEED_ORDER: Need[] = ["asks", "limit", "failed", "stuck", "review"];

/** The state a need says, as `sessionItem` reads a session's (a failure
 * keeps the state it has). */
const NEED_STATE: Partial<Record<Need, RailState>> = { asks: "asks_you", stuck: "asks_you", limit: "asks_you", review: "done_review" };

/** Needs you: what asks first, then limits, failures, stuck, things to
 * review; the longest waiting first within each. */
export function needsYou(items: RailItem[]): RailItem[] {
  return items
    .filter((item) => item.need !== null)
    .sort((a, b) => NEED_ORDER.indexOf(a.need!) - NEED_ORDER.indexOf(b.need!) || a.lastMs - b.lastMs || a.key.localeCompare(b.key));
}

/** Open now: open and interactive. A session that needs you stays in its
 * place here too, its state in amber; Needs you lists it as well, as a
 * shortcut (D-47, which replaced "not in Needs you"). A pinned session
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
