/**
 * AJ-E3713 R2, R4 — Home's feed as posts that tell the story: who the
 * post is by (an App, by its own icon and name, or AI Juggler for what it
 * did and saw), a 1–3 sentence story from the stored facts with the
 * things it names as chips, what it can show beside the words (a switch's
 * before → after, the agent's own reply, the room an Account has left),
 * and its follow-ups as a thread ("↳ 2 min later · finished").
 *
 * Every story is a template over facts (a): it says only what a journal
 * kept, never "Resolved: …" or a log line. D-149's feed-line job may
 * supply the story instead once its verdict passes (AJ-E3831, "pick,
 * don't write"): `setStorySource` is that seam; whatever it gives back
 * must be parts of the same shape, and without it (no AI) the template's
 * story still shows.
 */
import { appName, cliAppWord, desktopAppWord } from "./app-name";
import { toolSpec } from "./tools";
import { clock } from "./time-words";
import { durationWords } from "./feed-words";
import type { FeedItemInfo } from "./ipc/feed";

/** A story in parts: a `chip` names a thing (it opens it: the item's
 * own, or its pool); `strong` is a measured number. */
export interface StoryPart {
  text: string;
  chip?: "subject" | "pool";
  strong?: boolean;
}

const words = (text: string): StoryPart => ({ text });
const subject = (text: string): StoryPart => ({ text, chip: "subject" });
const pool = (text: string): StoryPart => ({ text, chip: "pool" });
const num = (text: string): StoryPart => ({ text, strong: true });

/** A post's author: an App (its own icon), or AI Juggler (its mark). */
export interface PostAuthor {
  kind: "app" | "juggler";
  /** The App's word ("Claude app", "Codex CLI"), or "AI Juggler". */
  name: string;
  /** The App's profile or folder ("work-max"), after the name. */
  place: string | null;
}

/** An App's word, from its tool, as every list names it. */
function appWord(provider: string, desktop: boolean): string {
  if (!desktop) return cliAppWord(provider);
  const app = toolSpec(provider).desktop_app;
  return app ? desktopAppWord(app, provider) : cliAppWord(provider);
}

/** Who a post is by: a session's App, else AI Juggler, which tells what
 * the pools, the Accounts and This Mac did (the mock's own posts). */
export function postAuthor(item: Pick<FeedItemInfo, "author" | "kind">): PostAuthor {
  const { author } = item;
  if (author.kind === "app" && author.provider) return { kind: "app", name: appWord(author.provider, author.desktop), place: author.folder_label || null };
  return { kind: "juggler", name: "AI Juggler", place: null };
}

/** An App named in a sentence ("Claude Code CLI · Work Max"). */
function appWords(provider: string | null, label: string | null): string {
  if (!provider) return label ?? "another app";
  return appName(cliAppWord(provider), label);
}

/** Who did it, as the story starts: the App's word, or the agent. */
function actor(item: FeedItemInfo): string {
  const { author } = item;
  if (author.kind === "agent") return author.name ?? "An agent";
  if (author.kind === "app" && author.provider) return appWord(author.provider, author.desktop);
  if (author.kind === "you") return "You";
  return "AI Juggler";
}

function windowWords(window: string | null): string {
  return window ? `its ${window} window` : "its usage window";
}

function after(ms: number | null): string {
  return ms ? ` after ${durationWords(ms)}` : "";
}

/** "in web-app" when the item names its project. */
function inProject(item: FeedItemInfo): string {
  return item.facts.project ? ` in ${item.facts.project}` : "";
}

const ANSWERED: Record<string, string> = {
  asks: " — it's moving again.",
  stuck: " got going again.",
  limit: " has room again and carries on.",
  signin: " is signed in again and carries on.",
};

/** A need settled, told as what came of it ("You answered X — it's
 * moving again"), never "Resolved: …". */
function settled(item: FeedItemInfo): StoryPart[] {
  const need = item.facts.need ?? "";
  if (!item.subject) return [words(`Something that needed you is settled${item.facts.line ? `: ${item.facts.line}` : ""}.`)];
  if (need === "asks") return [words("You answered "), subject(item.subject), words(ANSWERED.asks!)];
  return [subject(item.subject), words(ANSWERED[need] ?? " no longer needs you.")];
}

/** A switch: the conversation is copied, so it never says "moved" (the
 * 0.5.0 walk, F9); a switch back takes the original back. */
function switched(item: FeedItemInfo): StoryPart[] {
  const { facts } = item;
  const to = appWords(facts.to_provider, facts.to_label);
  if (item.kind === "switch_failed") return [words("AI Juggler couldn't copy "), subject(item.subject), words(` to ${to}${facts.detail ? ` (${facts.detail})` : ""}. The original is untouched.`)];
  if (facts.mechanism === "native-resume") return [subject(item.subject), words(` is back in ${to}.`)];
  const carried = facts.fidelity === "full" ? " It took the whole conversation along." : facts.fidelity ? ` It carried: ${facts.fidelity}.` : "";
  return [subject(item.subject), words(` carries on in ${to}: AI Juggler copied it over from ${appWords(facts.from_provider, facts.from_label)}.${carried}`)];
}

function usage(item: FeedItemInfo): StoryPart[] {
  const { facts } = item;
  const who = item.author.name || item.subject || "An Account";
  const resets = facts.resets_at_ms ? ` It resets at ${clock(facts.resets_at_ms)}` : "";
  const sessions = facts.sessions ? `${resets ? ", and" : " Right now"} ${facts.sessions} ${facts.sessions === 1 ? "session uses" : "sessions use"} it` : "";
  const tail = resets || sessions ? `${resets}${sessions}.` : "";
  switch (item.kind) {
    case "quota":
      return facts.meter === null
        ? [words(`${who} is running low on ${windowWords(facts.window)}.${tail}`)]
        : [words(`${who} has `), num(`${facts.meter}%`), words(` of ${windowWords(facts.window)} left.${tail}`)];
    case "credits":
      return [words(`${who} used ${windowWords(facts.window)}; credits carry on${facts.credits ? ` (${facts.credits})` : ""}.`)];
    case "reset":
      return [words(`${who} has room again: ${windowWords(facts.window)} reset.${facts.sessions ? ` ${facts.sessions} ${facts.sessions === 1 ? "session" : "sessions"} can switch back.` : ""}`)];
    default:
      return [words(facts.line ?? `${who} changed.`)];
  }
}

const MESSAGE: Record<string, string> = {
  handoff: " handed work over in ",
  question: " asked a question in ",
  status: " said where things stand in ",
  review_request: " asked for a review in ",
};

/** A pool's work, told from the pool ("In the X pool, Planner …"). */
function pooled(item: FeedItemInfo): StoryPart[] {
  const { facts } = item;
  const lead = facts.pool ? [words("In the "), pool(facts.pool), words(" pool, ")] : [];
  const who = item.author.kind === "agent" ? (item.author.name ?? "an agent") : null;
  const lower = (parts: StoryPart[]) => (lead.length > 0 && parts[0] && !parts[0].chip ? [{ ...parts[0], text: parts[0].text.charAt(0).toLowerCase() + parts[0].text.slice(1) }, ...parts.slice(1)] : parts);
  switch (item.kind) {
    case "message":
      return [...lead, words(`${who ?? "an agent"}${MESSAGE[facts.change ?? ""] ?? " wrote in "}`), subject(item.subject), words(".")];
    case "task_done":
      return [...lead, ...lower(who ? [words(`${who} finished the task `), subject(item.subject), words(".")] : [words("the task "), subject(item.subject), words(" is done.")])];
    case "task_review":
      return [...lead, ...lower([words("the task "), subject(item.subject), words(` is waiting for your review${who ? ` from ${who}` : ""}.`)])];
    default:
      return [...lead, ...lower([words("the task "), subject(item.subject), words(` is blocked${facts.change ? ` (${facts.change})` : ""}.`)])];
  }
}

/** A session's endings and marks, told from their facts. */
const SESSION_STORIES: Record<string, (item: FeedItemInfo, who: string) => StoryPart[]> = {
  finished: (item, who) => [words(`${who} finished `), subject(item.subject), words(`${inProject(item)}${item.facts.duration_ms ? ` in ${durationWords(item.facts.duration_ms)}` : ""}.`)],
  failed: (item, who) => [words(`${who} hit an error on `), subject(item.subject), words(` and stopped${item.facts.detail ? ` (${item.facts.detail})` : ""}${after(item.facts.duration_ms)}.`)],
  crashed: (item, who) => [subject(item.subject), words(` crashed in ${who}${item.facts.detail ? ` (${item.facts.detail})` : ""}${after(item.facts.duration_ms)}.`)],
  ended: (item) => [subject(item.subject), words(` ended when AI Juggler quit${after(item.facts.duration_ms)}.`)],
  restarted: (item) => [words("AI Juggler restarted "), subject(item.subject), words(` after it crashed${item.facts.attempt && item.facts.tries ? ` (try ${item.facts.attempt} of ${item.facts.tries})` : ""}.`)],
  stopped: (item) => [words("You stopped "), subject(item.subject), words(`${after(item.facts.duration_ms)}.`)],
  reviewed: (item) => [words("You looked over "), subject(item.subject), words(" and marked it reviewed.")],
  archived: (item) => [words("You archived "), subject(item.subject), words(".")],
  worked: (item, who) => [words(`${who} worked on `), subject(item.subject), words(`${inProject(item)}.`)],
};

/** Every other kind's story, by kind. */
const STORIES: Record<string, (item: FeedItemInfo) => StoryPart[]> = {
  answered: settled,
  switch: switched,
  switch_failed: switched,
  quota: usage,
  credits: usage,
  reset: usage,
  renewal: usage,
  message: pooled,
  task_done: pooled,
  task_review: pooled,
  task_blocked: pooled,
  watch: (item) => (item.facts.told && item.subject ? [subject(item.subject), words(` ${item.facts.told} — your watch.`)] : [words(`${item.facts.line ?? "A watch you set was met"} — your watch.`)]),
};

/** The template's story of one item (R4: no AI, still a story). */
export function templateStory(item: FeedItemInfo): StoryPart[] {
  const session = SESSION_STORIES[item.kind];
  if (session) return session(item, actor(item));
  const other = STORIES[item.kind];
  // AI Juggler's own notices (This Mac, keep alive, an update, an app
  // found): the line it wrote from the same facts.
  return other ? other(item) : [words(item.facts.line ?? "Something changed.")];
}

/** D-149's feed-line job (AJ-E3831): a story for an item, once its
 * verdict passes; `null` keeps the template's. */
export type StorySource = (item: FeedItemInfo) => StoryPart[] | null;

let source: StorySource | null = null;

/** Sets (or clears) the story source; AJ-E3831 calls it when its feed
 * job's verdict passes, and with `null` when it doesn't. */
export function setStorySource(next: StorySource | null): void {
  source = next;
}

/** A post's story: the job's when it gives one that names the same
 * thing, else the template's. */
export function story(item: FeedItemInfo): StoryPart[] {
  const picked = source?.(item) ?? null;
  // A picked story must still name the item's thing (stories from facts
  // only, the usability answer's watch-for): one that doesn't is dropped.
  if (picked && picked.length > 0 && (!item.subject || picked.some((part) => part.chip === "subject" && part.text === item.subject))) return picked;
  return templateStory(item);
}

/** A story as one string (an accessible name, a test). */
export function storyText(parts: readonly StoryPart[]): string {
  return parts.map((part) => part.text).join("");
}

/** What a post shows beside its words, from its facts only. */
export type Attachment =
  | { kind: "switch"; from: { label: string; provider: string | null }; to: { label: string; provider: string | null } }
  | { kind: "quote"; text: string; who: string }
  | { kind: "usage"; left: number; window: string | null; resetsAt: number | null };

/** A post's attachment, when its facts carry one. */
export function attachmentOf(item: FeedItemInfo): Attachment | null {
  const { facts } = item;
  if ((item.kind === "switch" || item.kind === "switch_failed") && facts.from_provider && facts.to_provider) {
    return { kind: "switch", from: { label: appWords(facts.from_provider, facts.from_label), provider: facts.from_provider }, to: { label: appWords(facts.to_provider, facts.to_label), provider: facts.to_provider } };
  }
  if (facts.quote) return { kind: "quote", text: facts.quote, who: item.kind === "message" ? `${item.author.name ?? "The agent"}'s words` : `${actor(item)}'s last reply` };
  if ((item.kind === "quota" || item.kind === "credits") && facts.meter !== null) return { kind: "usage", left: facts.meter, window: facts.window, resetsAt: facts.resets_at_ms };
  return null;
}

/** A follow-up's lead: "↳ 2 min later", from the post's own time. */
export function laterWords(at: number, from: number): string {
  const gap = Math.max(0, at - from);
  if (gap < 60_000) return "↳ moments later";
  return `↳ ${durationWords(gap)} later`;
}

/** A follow-up's words: what came next for the same thing, short. */
export function followUp(item: FeedItemInfo): StoryPart[] {
  const { facts } = item;
  switch (item.kind) {
    case "finished":
      return [subject(item.subject), words(` finished${facts.duration_ms ? ` in ${durationWords(facts.duration_ms)}` : ""}.`)];
    case "failed":
      return [subject(item.subject), words(` stopped with an error${facts.detail ? ` (${facts.detail})` : ""}.`)];
    case "crashed":
      return [subject(item.subject), words(" crashed.")];
    case "restarted":
      return [words(`AI Juggler restarted it${facts.attempt && facts.tries ? ` (try ${facts.attempt} of ${facts.tries})` : ""}.`)];
    case "reviewed":
      return [words("You marked it reviewed.")];
    case "archived":
      return [words("You archived it.")];
    case "stopped":
      return [words("You stopped it.")];
    default:
      return templateStory(item);
  }
}
