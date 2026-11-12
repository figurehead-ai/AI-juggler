/**
 * A title for a session nothing named (the review of 0.1.5: Home's rows
 * all read "Untitled session · payments-api"): the first words of the
 * last thing you asked it, read from the end of its transcript (already
 * redacted, and only while Last messages is on), else its telling branch,
 * else its facts: folder · App · when it started (D-96). Read once per conversation, after Home
 * has painted, and kept for the window's life only.
 */
import { useEffect, useState } from "react";
import { conversationsLastMessages } from "./ipc/conversations";
import { hasLiveBackend } from "./bridge";
import { pastWhen } from "./time-words";
import { tellingBranch } from "./project-words";
import { appShortName, sessionAppWords } from "./app-name";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** The first few words of what was asked: its first line, cut before a
 * redaction mark, its first clause past a short opener ("Good.", "OK,
 * thanks."), at most `max` words or 56 characters, "…" when cut short. */
export function firstWords(text: string, max = 8): string | null {
  const line = text
    .split("\n")
    .map((part) => part.trim())
    .find(Boolean);
  if (!line) return null;
  const clean = line.split(/•{2,}|\[redacted\]/i)[0]!.replace(/[`*_#>]/g, "").replace(/\s+/g, " ").trim();
  const clauses = clean.split(/(?<=[.;:!?])\s+/).filter(Boolean);
  while (clauses.length > 1 && clauses[0]!.split(" ").length < 3) clauses.shift();
  const clause = (clauses[0] ?? "").replace(/[\s,;:–—-]+$/, "");
  if (clause.length < 3) return null;
  const words = clause.split(" ");
  let out = words.slice(0, max).join(" ");
  if (out.length > 56) out = out.slice(0, 56).replace(/\s+\S*$/, "");
  const cut = out.length < clause.replace(/[.!?]+$/, "").length;
  out = out.replace(/[.,;:!]+$/, "");
  return cut ? `${out}…` : out;
}

/** When it started, in a few words (`pastWhen`): "started 1:14 AM"
 * today, "started yesterday 1:14 AM", "started Thu 1:14 AM" within the
 * week, and with its date from a week back ("started Thu, Oct 23 1:14
 * AM"; its year when not this one's). */
export function startedWords(ms: number, now: number): string {
  return `started ${pastWhen(ms, now)}`;
}

/** What an untitled session is told apart by (D-96): its folder (or
 * project), its App, and when it started. A list whose rows sit under one
 * of them (the processes tree, under its App) leaves it `null`. */
export interface SessionFacts {
  folder: string | null;
  app: string | null;
  startedMs: number | null;
}

/** D-96 (persona D2): an untitled session's name, by its facts — "billing
 * · Work · started 1:14 AM", "billing · Work · started yesterday 9:02
 * PM", never "Untitled session · billing", which two sessions in one
 * folder share. "Session started 1:14 AM" when only the time is known;
 * "Untitled session" when nothing is; an App label that is an email is
 * left out (D-33). */
export function untitledName(facts: SessionFacts, now: number): string {
  const { folder, app: named, startedMs } = facts;
  const lead = folder || null;
  const when = startedMs != null && startedMs > 0 ? startedWords(startedMs, now) : null;
  // D-33: an App is never named by an email.
  const app = named && !named.includes("@") ? named : null;
  const parts = [lead, app, when].filter((part): part is string => part !== null);
  if (parts.length === 0) return "Untitled session";
  return lead === null && app === null ? `Session ${when}` : parts.join(" · ");
}

/** A session's App in the short words a name has room for: its profile,
 * else its app ("Work", "ChatGPT"; `appShortName`). */
export function sessionAppShort(row: { kind: string; provider: string; account_label: string }): string {
  return appShortName(sessionAppWords(row));
}

/** Names that would read alike, told apart the way a pool's cards are:
 * "(1)", "(2)" by when each started (then by key), so the same minute in
 * the same folder on the same App still reads as two. Rows whose names
 * differ are left as they are. */
export function tellApart<T>(rows: readonly T[], name: (row: T) => string, order: (row: T) => [number, string]): Map<T, string> {
  const groups = new Map<string, T[]>();
  for (const row of rows) groups.set(name(row), [...(groups.get(name(row)) ?? []), row]);
  const out = new Map<T, string>();
  for (const [shown, same] of groups) {
    if (same.length === 1) {
      out.set(same[0]!, shown);
      continue;
    }
    const sorted = [...same].sort((a, b) => order(a)[0] - order(b)[0] || order(a)[1].localeCompare(order(b)[1]));
    sorted.forEach((row, at) => out.set(row, `${shown} (${at + 1})`));
  }
  return out;
}

/** An untitled session's name from what it was last asked, as Home and
 * the rail show it; `null` while unknown or when it has its own title
 * (the 0.4.1 persona walk: its own page said "Untitled session · …"). */
export function useAskedName(session: { label?: string | null; title?: string | null; account_id?: string | null; conversation_id?: string | null } | null): string | null {
  const ask = session && !session.label && !session.title && session.conversation_id ? { profileId: session.account_id ?? "", conversationId: session.conversation_id } : null;
  const asked = useAskedWords(ask ? [ask] : []);
  return ask ? (asked.get(askedKey(ask.profileId, ask.conversationId)) ?? null) : null;
}

/** The key what was asked is kept under. */
export function askedKey(profileId: string, conversationId: string): string {
  return `${profileId}\u0000${conversationId}`;
}

const asked = new Map<string, string | null>();
const reading = new Map<string, Promise<string | null>>();

/** Tests: forget what was read. */
export function forgetAskedForTests(): void {
  asked.clear();
  reading.clear();
}

function read(profileId: string, conversationId: string): Promise<string | null> {
  const key = askedKey(profileId, conversationId);
  const pending =
    reading.get(key) ??
    conversationsLastMessages(profileId, conversationId, { max: 12 }).then(
      (info) => {
        const last = info.state === "shown" ? [...info.turns].reverse().find((turn) => turn.speaker === "you") : undefined;
        const text = last?.blocks.filter((block) => block.kind === "text").map((block) => block.text).join("\n") ?? "";
        const words = text ? firstWords(text) : null;
        asked.set(key, words);
        return words;
      },
      () => {
        asked.set(key, null);
        return null;
      },
    );
  reading.set(key, pending);
  return pending;
}

/** What each untitled conversation was last asked, by
 * `profile\0conversation`: read in idle time after the first paint. */
export function useAskedWords(rows: { profileId: string; conversationId: string }[]): ReadonlyMap<string, string | null> {
  const wanted = rows.map((row) => askedKey(row.profileId, row.conversationId)).join("\n");
  const [, setTick] = useState(0);
  useEffect(() => {
    const todo = rows.filter((row) => !asked.has(askedKey(row.profileId, row.conversationId)));
    if (todo.length === 0) return;
    let alive = true;
    const w = window as unknown as { requestIdleCallback?: (fn: () => void, opts?: { timeout: number }) => number };
    const start = () => {
      void Promise.all(todo.map((row) => read(row.profileId, row.conversationId))).then(() => alive && setTick((tick) => tick + 1));
    };
    // The preview has nothing to read from disk: at once, so a capture
    // shows the words the first time as the second.
    if (!hasLiveBackend()) start();
    else if (w.requestIdleCallback) w.requestIdleCallback(start, { timeout: 1500 });
    else setTimeout(start, 50);
    return () => {
      alive = false;
    };
    // `wanted` stands for `rows` (a new array each render).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wanted]);
  return asked;
}

/** The last part of a working folder ("billing"). */
export function folderName(cwd: string | null | undefined): string {
  const path = cwd ?? "";
  return path.replace(/\/+$/, "").split("/").pop() || path;
}

/** Whether a session is named by its facts (D-96): no label, no title,
 * no branch that says what the work was. Its name has its start time in
 * it already. */
export function namedByFacts(session: { label?: string | null; title?: string | null; git_branch?: string | null }): boolean {
  return !session.label && !session.title && tellingBranch(session.git_branch) === null;
}

/** A list's rows with the names its facts-named rows share told apart
 * ([`tellApart`]); a row with a title or label is left as it is. */
export function tellFactsApart<T extends { key: string; title: string; startMs: number; session: { label?: string | null; title?: string | null; git_branch?: string | null } | null; conversation: { title?: string | null; git_branch?: string | null } | null }>(rows: T[]): T[] {
  const facts = rows.filter((row) => (row.session ?? row.conversation) !== null && namedByFacts(row.session ?? row.conversation!));
  if (facts.length < 2) return rows;
  const apart = tellApart(facts, (row) => row.title, (row) => [row.startMs, row.key]);
  return rows.map((row) => {
    const title = apart.get(row);
    return title === undefined || title === row.title ? row : { ...row, title };
  });
}

/** A session's short name, the one rule wherever a short one is needed
 * (the rail, Needs you, a pool's canvas, its list and its messages, the
 * session's header, the palette): its label, else its title's first
 * words, else a branch that says what the work was ("fix/limits"), else
 * its facts (D-96: "billing · Work · started 1:14 AM") — never the whole
 * first prompt, never "main" (the rc7 and rc12 walks), never "Untitled
 * session · <folder>" (persona D2). The member card's name (`card_name`)
 * and the backend's `naming::short_name` are the same rule. */
export function shortName(session: Pick<InstanceInfo, "label" | "title" | "git_branch" | "cwd" | "account_label" | "kind" | "provider" | "started_at_ms">, now: number = Date.now()): string {
  if (session.label) return session.label;
  if (session.title) return firstWords(session.title) ?? session.title;
  return tellingBranch(session.git_branch) ?? untitledName({ folder: folderName(session.cwd) || null, app: sessionAppShort(session), startedMs: session.started_at_ms }, now);
}

/** Sessions named in a sentence by the one short-name rule
 * ([`shortName`]), each once: "fix/limits and billing · Work · started
 * 1:14 AM"; two untitled ones that read alike are told apart as the rail
 * does ([`namesApart`]). The worktree question says who is working in a folder
 * this way (the rc27 pool pass: "Account G, Account G are already working
 * in web" named the App twice and no session). */
export function sessionsInWords(sessions: readonly NamedSession[], now: number = Date.now()): string {
  const named = namesApart(sessions, now);
  const names = [...new Set(sessions.map((session) => named.get(session.id) ?? shortName(session, now)))];
  if (names.length <= 1) return names[0] ?? "Another session";
  return `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** What a session's short name is made of, and its id. */
type NamedSession = Parameters<typeof shortName>[0] & { id: string };

/** Each session's short name ([`shortName`]) by its id, the ones named by
 * their facts that still read alike told apart ([`tellApart`]: "(1)",
 * "(2)" by when each started), as the rail, Needs you and a pool's nodes
 * name them (D-96 D). Every list of sessions to pick from uses it (the
 * rc27 pool pass: four untitled Lab sessions in one folder all read "team
 * · Lab · started 9:2x AM" in the New pool wizard and the canvas's Add a
 * session). */
export function namesApart(sessions: readonly NamedSession[], now: number = Date.now()): Map<string, string> {
  const out = new Map(sessions.map((session) => [session.id, shortName(session, now)] as const));
  const facts = sessions.filter((session) => namedByFacts(session));
  const apart = tellApart(facts, (session) => out.get(session.id)!, (session) => [session.started_at_ms, session.id]);
  for (const [session, name] of apart) out.set(session.id, name);
  return out;
}
