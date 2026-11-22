/**
 * D-118 (AJ-E3702 R10) — every sentence of Home's activity feed, from the
 * facts the read model keeps (as `notice-words.ts` does for notices): who
 * acted, what changed, the result, the object's name as its link. No
 * emoji, no exclamation marks, no first person; the author is whoever
 * acted (an App, an agent, an Account, or AI Juggler only for its own
 * acts); a number is measured or said to be an estimate. Times: relative
 * under an hour, then the clock, "Yesterday …", then the date; the exact
 * time in the tooltip and the accessible name.
 */
import { appName, cliAppName, desktopAppWord } from "./app-name";
import { toolSpec } from "./tools";
import { clock } from "./time-words";
import type { FeedItemInfo } from "./ipc/feed";
import type { DigestInfo } from "../types/generated/DigestInfo";
import type { DigestLineInfo } from "../types/generated/DigestLineInfo";

/** A sentence in parts: `strong` is the value (the object's name, a
 * number), drawn semibold; the object's name is its link. */
export interface Part {
  text: string;
  strong?: boolean;
}

const plain = (text: string): Part => ({ text });
const strong = (text: string): Part => ({ text, strong: true });

/** A duration in words: "42 min", "2 h 5 min", "under a minute". */
export function durationWords(ms: number): string {
  if (ms < 60_000) return "under a minute";
  const minutes = Math.round(ms / 60_000);
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${hours} h` : `${hours} h ${rest} min`;
}

const DAY_MS = 86_400_000;

function dayStart(ms: number): number {
  const day = new Date(ms);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/** When, as the feed says it (R10): "just now", "12 min", "9:58 AM",
 * "Yesterday 4:10 PM", "Nov 3". */
export function feedWhen(at: number, now: number): string {
  const ago = now - at;
  if (ago < 60_000) return "just now";
  if (ago < 3_600_000) return `${Math.floor(ago / 60_000)} min`;
  const today = dayStart(now);
  if (at >= today) return clock(at);
  if (at >= today - DAY_MS) return `Yesterday ${clock(at)}`;
  const sameYear = new Date(at).getFullYear() === new Date(now).getFullYear();
  return new Date(at).toLocaleDateString([], sameYear ? { month: "short", day: "numeric" } : { year: "numeric", month: "short", day: "numeric" });
}

/** The exact time (the tooltip, the accessible name). */
export function exactWhen(at: number): string {
  return new Date(at).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}

/** A day separator's label: Today, Yesterday, else the date. */
export function dayLabel(at: number, now: number): string {
  const today = dayStart(now);
  if (at >= today) return "Today";
  if (at >= today - DAY_MS) return "Yesterday";
  return new Date(at).toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
}

/** An App by its full name, from a tool and an Account folder's name. */
function appWords(provider: string | null, folderLabel: string | null, desktop: boolean): string {
  if (!provider) return folderLabel ?? "An app";
  if (!desktop) return cliAppName(provider, folderLabel);
  const app = toolSpec(provider).desktop_app;
  return app ? appName(desktopAppWord(app, provider), folderLabel) : cliAppName(provider, folderLabel);
}

/** Who acted, as the author line names them. */
export function authorWords(item: Pick<FeedItemInfo, "author">): string {
  const { author } = item;
  switch (author.kind) {
    case "app":
      return appWords(author.provider, author.folder_label, author.desktop);
    case "agent":
    case "account":
      return author.name ?? (author.kind === "agent" ? "An agent" : "An Account");
    case "you":
      return "You";
    default:
      return "AI Juggler";
  }
}

/** Where it happened, after the author: a project, a pool and thread, or
 * what the author is ("Account"). */
export function placeWords(item: Pick<FeedItemInfo, "author" | "facts" | "kind">): string | null {
  const { facts } = item;
  if (facts.pool) return [facts.pool, facts.thread].filter(Boolean).join(" · ");
  if (item.author.kind === "account") return "Account";
  if (item.kind === "switch" || item.kind === "switch_failed") return facts.project ? `switch · ${facts.project}` : "switch";
  return facts.project ?? null;
}

const ANSWERED: Record<string, string> = {
  asks: " got its answer",
  stuck: " moved on",
  limit: " is past its limit",
  signin: " is signed in again",
};

/** A resolved need's history line: quiet, never a second loud mark. */
function answered(item: FeedItemInfo): Part[] {
  const tail = ANSWERED[item.facts.need ?? ""] ?? " no longer needs you";
  if (item.subject) return [strong(item.subject), plain(tail)];
  return [plain(`Resolved: ${item.facts.line ?? "a need"}`)];
}

function ended(item: FeedItemInfo, how: string): Part[] {
  const { facts } = item;
  const detail = facts.detail ? ` (${facts.detail})` : "";
  const after = facts.duration_ms ? ` after ${durationWords(facts.duration_ms)}` : "";
  return [strong(item.subject), plain(` ${how}${detail}${after}`)];
}

function windowWords(window: string | null): string {
  return window ? `its ${window} window` : "its usage window";
}

function account(item: FeedItemInfo): Part[] {
  const { facts } = item;
  const resets = facts.resets_at_ms ? ` It resets at ${clock(facts.resets_at_ms)}.` : "";
  const sessions = facts.sessions ? ` ${facts.sessions} ${facts.sessions === 1 ? "session uses" : "sessions use"} it.` : "";
  switch (item.kind) {
    case "quota":
      return facts.meter === null
        ? [plain(`is running low on ${windowWords(facts.window)}.${resets}${sessions}`)]
        : [plain("is at "), strong(`${facts.meter}%`), plain(` of ${windowWords(facts.window)}.${resets}${sessions}`)];
    case "credits":
      return [plain(`used ${windowWords(facts.window)}. Credits carry on${facts.credits ? ` (${facts.credits})` : ""}.`)];
    case "reset":
      return [plain(`has room again: ${windowWords(facts.window)} reset.${facts.sessions ? ` ${facts.sessions} ${facts.sessions === 1 ? "session" : "sessions"} can switch back.` : ""}`)];
    default:
      return [plain(facts.line ?? "changed.")];
  }
}

/** AJ-E3817: a watch the user set was met ("api finished · your watch");
 * the words are the app's, from the typed watch. */
function watched(item: FeedItemInfo): Part[] {
  const { told, line } = item.facts;
  if (told && item.subject) return [strong(item.subject), plain(` ${told} · your watch`)];
  return [plain(line ?? "A watch you set was met."), plain(" · your watch")];
}

function message(item: FeedItemInfo): Part[] {
  const verb: Record<string, string> = { handoff: "handed work over in ", question: "asked a question in ", status: "posted where it is in ", review_request: "asked for a review in " };
  return [plain(verb[item.facts.change ?? ""] ?? "wrote in "), strong(item.subject)];
}

/** The kinds whose sentence is a verb, then the thing's name. */
const VERB_FIRST: Partial<Record<string, string>> = {
  stopped: "stopped ",
  archived: "archived ",
  task_done: "finished the task ",
  worked: "worked on ",
};

/** A switch edge that resumed the original (a switch back, an undo). */
const RESUMED = "native-resume";

/** One item's sentence (R10). */
export function sentence(item: FeedItemInfo): Part[] {
  const { facts } = item;
  const verb = VERB_FIRST[item.kind];
  if (verb) return [plain(verb), strong(item.subject)];
  switch (item.kind) {
    case "finished":
      return [plain("finished "), strong(item.subject), plain(facts.duration_ms ? ` in ${durationWords(facts.duration_ms)}` : "")];
    case "failed":
      return ended(item, "stopped with an error");
    case "crashed":
      return ended(item, "crashed");
    case "ended":
      return ended(item, "ended when AI Juggler quit");
    case "restarted":
      return [plain("restarted "), strong(item.subject), plain(` after it crashed${facts.attempt && facts.tries ? ` (try ${facts.attempt} of ${facts.tries})` : ""}`)];
    case "reviewed":
      return [plain("marked "), strong(item.subject), plain(" reviewed")];
    case "answered":
      return answered(item);
    // The 0.5.0 walk, F9: a switch copies the conversation (the original
    // stays), so it never says "moved"; a switch back resumes the original.
    case "switch":
      return facts.mechanism === RESUMED
        ? [plain("took "), strong(item.subject), plain(` back to ${appWords(facts.to_provider, facts.to_label, false)}`)]
        : [plain("copied "), strong(item.subject), plain(` from ${appWords(facts.from_provider, facts.from_label, false)} to ${appWords(facts.to_provider, facts.to_label, false)}`)];
    case "switch_failed":
      return [plain(facts.mechanism === RESUMED ? "couldn't take " : "couldn't copy "), strong(item.subject), plain(`${facts.mechanism === RESUMED ? " back" : ""} to ${appWords(facts.to_provider, facts.to_label, false)}. The original is untouched.`)];
    case "quota":
    case "credits":
    case "reset":
    case "renewal":
      return account(item);
    case "task_review":
      return [strong(item.subject), plain(" is waiting for review")];
    case "task_blocked":
      return [strong(item.subject), plain(" is blocked")];
    case "message":
      return message(item);
    case "watch":
      return watched(item);
    default:
      // AI Juggler's own notices (This Mac, keep alive, an update, an app
      // found): the line it wrote from the same facts.
      return [plain(facts.line ?? "Something changed.")];
  }
}

/** The facts line (muted): project or folder, the ending's detail, a
 * restart's try, what a switch carried, a task's change. */
export function factsLine(item: FeedItemInfo, restart?: FeedItemInfo | null): string | null {
  const { facts } = item;
  const parts: string[] = [];
  if (restart) parts.push(`AI Juggler restarted it${restart.facts.attempt && restart.facts.tries ? ` (try ${restart.facts.attempt} of ${restart.facts.tries})` : ""}`);
  if (item.kind === "switch") {
    if (facts.fidelity) parts.push(`carried: ${facts.fidelity}`);
    if (facts.mechanism) parts.push(`by ${facts.mechanism}`);
  }
  if (item.kind === "switch_failed" && facts.detail) parts.push(facts.detail);
  if (item.kind.startsWith("task_") && facts.change) parts.push(facts.change);
  return parts.length > 0 ? parts.join(" · ") : null;
}

/** A rollup's sentence: "finished 4 sessions", with its place. */
export function rollupSentence(kind: string, count: number, head: FeedItemInfo): Part[] {
  const sessions = `${count} sessions`;
  const where = head.facts.project ? ` in ${head.facts.project}` : "";
  switch (kind) {
    case "finished":
      return [plain("finished "), strong(sessions), plain(where)];
    case "worked":
      return [plain("worked on "), strong(sessions), plain(where)];
    case "ended":
      return [strong(sessions), plain(`${where} ended when AI Juggler quit`)];
    case "task_done":
      return [plain("finished "), strong(`${count} tasks`)];
    case "task_review":
      return [strong(`${count} tasks`), plain(" are waiting for review")];
    case "task_blocked":
      return [strong(`${count} tasks`), plain(" are blocked")];
    case "message":
      return [strong(`${count} messages`), plain(" in "), strong(head.subject)];
    default:
      return [...sentence(head), plain(` (${count} times)`)];
  }
}

/** A sentence as one string (an accessible name, a test). */
export function said(parts: readonly Part[]): string {
  return parts.map((part) => part.text).join("");
}

/** The away digest's kinds, most important first, in words. */
const DIGEST: [kinds: string[], one: string, many: string][] = [
  [["failed", "crashed"], "stopped with an error", "stopped with an error"],
  [["switch_failed"], "switch didn't carry", "switches didn't carry"],
  [["finished"], "finished", "finished"],
  [["worked"], "session worked on in other apps", "sessions worked on in other apps"],
  [["switch"], "switch", "switches"],
  [["reset"], "Account got its room back", "Accounts got their room back"],
  [["quota", "credits"], "Account ran low", "Accounts ran low"],
  [["task_done"], "task done", "tasks done"],
  [["task_review", "task_blocked"], "task waits", "tasks wait"],
  [["message"], "pool message", "pool messages"],
  [["answered"], "need resolved", "needs resolved"],
];

/** The away digest's first line: "6 finished, 1 stopped with an error,
 * 2 switches" (counts of things, by kind, most important first). */
export function digestWords(kinds: readonly string[]): string {
  const parts: [count: number, words: string][] = [];
  for (const [group, one, many] of DIGEST) {
    const count = kinds.filter((kind) => group.includes(kind)).length;
    if (count > 0) parts.push([count, `${count} ${count === 1 ? one : many}`]);
  }
  // A glance, not a list: the four most important kinds, then the rest
  // in one count (Show all lists them).
  const shown = parts.slice(0, DIGEST_KINDS);
  const rest = kinds.length - shown.reduce((sum, [count]) => sum + count, 0);
  const words = shown.map(([, part]) => part);
  if (rest > 0) words.push(`${rest} more ${rest === 1 ? "update" : "updates"}`);
  return words.length > 0 ? words.join(", ") : "Nothing changed";
}

/** Kinds the digest names before "n more updates". */
const DIGEST_KINDS = 4;

/** The empty feed (R12). */
export const EMPTY_FEED = "Quiet so far. When your apps start, finish or ask you something, it shows here.";

/** AJ-E3712 (ADR-0036 §1.3): the agent digest's header, how many agents
 * changed in its window ("5 agents"). */
export function digestHeader(digest: Pick<DigestInfo, "lines">): string {
  const count = digest.lines.length;
  return `${count} ${count === 1 ? "agent" : "agents"}`;
}

/** A digest line's state, from the newest fact (the backend's rollup). */
const DIGEST_STATE: Record<string, string> = {
  failed: "failed",
  asks: "asks you",
  limit: "hit its usage limit",
  stuck: "looks stuck",
  finished: "finished",
  stopped: "stopped",
  idle: "idle",
  working: "working",
};
const DIGEST_CAVEAT: Record<string, string> = { not_run: "tests not run", not_done: "not done", partial: "partly done", blocked: "blocked", unsure: "unsure" };
const DIGEST_TASK: Record<string, string> = { done: "a task done", blocked: "a task blocked", in_review: "a task in review" };

/** A digest line's words after the agent's name (ADR-0036 §1.4): its
 * state, its own question or failure, its facts, its last words. Only the
 * facts the line holds are said: none is guessed. */
export function digestLineWords(line: DigestLineInfo): { state: string; said: string | null; facts: string | null; last: string | null } {
  const state = line.state === "stuck" && line.stuck_min ? `stuck for ${line.stuck_min} min` : (DIGEST_STATE[line.state] ?? line.state);
  const own = line.quote && line.quote_kind !== "last" ? `: “${line.quote}”` : null;
  const facts = [...digestEvents(line), ...digestWork(line)];
  const last = line.quote && line.quote_kind === "last" ? ` · last: “${line.quote}”` : null;
  return { state: ` · ${state}`, said: own, facts: facts.length > 0 ? ` · ${facts.join(" · ")}` : null, last };
}

const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

/** What happened to the agent: processes left, room, a restart, a task. */
function digestEvents(line: DigestLineInfo): string[] {
  const facts: string[] = [];
  if (line.left_processes) facts.push(`left ${plural(line.left_processes, "process", "processes")} running`);
  if (line.runway_left !== null) facts.push(`${line.runway_left}% room left`);
  if (line.restarted) facts.push("restarted after a crash");
  if (line.task) facts.push(DIGEST_TASK[line.task] ?? "a task moved");
  return facts;
}

/** What the agent's work says: turns, files, tests, reverted, a caveat. */
function digestWork(line: DigestLineInfo): string[] {
  const facts: string[] = [];
  if (line.turns) facts.push(plural(line.turns, "turn", "turns"));
  if (line.files) facts.push(`${plural(line.files, "file", "files")} changed`);
  if (line.tests === "passed") facts.push(line.passed ? `tests passed (${line.passed})` : "tests passed");
  if (line.tests === "failed") facts.push(line.failed ? `tests failed (${line.failed})` : "tests failed");
  if (line.tests === "not_run") facts.push("tests not run");
  if (line.reverted) facts.push("reverted");
  // A caveat only where the facts allow it (the backend's rule); never
  // "tests not run" twice.
  if (line.caveat && !(line.caveat === "not_run" && line.tests !== null)) facts.push(DIGEST_CAVEAT[line.caveat] ?? "");
  return facts;
}

/** The digest's fold: "and 3 more agents · 2 quiet"; none when nothing is
 * folded or quiet. */
export function digestMoreWords(more: number, quiet: number): string | null {
  const parts: string[] = [];
  if (more > 0) parts.push(`and ${more} more ${more === 1 ? "agent" : "agents"}`);
  if (quiet > 0) parts.push(`${quiet} quiet`);
  return parts.length > 0 ? parts.join(" · ") : null;
}
