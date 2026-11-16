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

function message(item: FeedItemInfo): Part[] {
  const verb: Record<string, string> = { handoff: "handed work over in ", question: "asked a question in ", status: "posted where it is in ", review_request: "asked for a review in " };
  return [plain(verb[item.facts.change ?? ""] ?? "wrote in "), strong(item.subject)];
}

/** One item's sentence (R10). */
export function sentence(item: FeedItemInfo): Part[] {
  const { facts } = item;
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
    case "stopped":
      return [plain("stopped "), strong(item.subject)];
    case "reviewed":
      return [plain("marked "), strong(item.subject), plain(" reviewed")];
    case "archived":
      return [plain("archived "), strong(item.subject)];
    case "answered":
      return answered(item);
    case "switch":
      return [plain("moved "), strong(item.subject), plain(` from ${appWords(facts.from_provider, facts.from_label, false)} to ${appWords(facts.to_provider, facts.to_label, false)}`)];
    case "switch_failed":
      return [plain("couldn't move "), strong(item.subject), plain(` to ${appWords(facts.to_provider, facts.to_label, false)}. The original is untouched.`)];
    case "quota":
    case "credits":
    case "reset":
    case "renewal":
      return account(item);
    case "task_done":
      return [plain("finished the task "), strong(item.subject)];
    case "task_review":
      return [strong(item.subject), plain(" is waiting for review")];
    case "task_blocked":
      return [strong(item.subject), plain(" is blocked")];
    case "message":
      return message(item);
    case "worked":
      return [plain("worked on "), strong(item.subject)];
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
