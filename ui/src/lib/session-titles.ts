/**
 * A title for a session nothing named (the review of 0.1.5: Home's rows
 * all read "Untitled session · payments-api"): its conversation's title,
 * the tool's own name for it or else the first words of its first prompt
 * as `claude --resume` and Codex's picker list it (0.4.5: the backend
 * reads it in the pass that indexes its transcript, so no second read),
 * else its telling branch, else its facts: its folder and when it started
 * (D-96, D-103.8). Never a pasted key or a bare path.
 */
import { pastWhen } from "./time-words";
import { tellingBranch } from "./project-words";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** The most characters a name of words keeps (0.4.5: about 60, cut at a
 * word). */
const NAME_CHARS = 60;

/** A word that looks like a pasted key or token (`sk-ant-api03-…`, a
 * hash, a base64 blob): a run of at least 16 ASCII letters and digits
 * that has both. Long plain words ("extraordinarily") have no digit. */
function tokenLike(word: string): boolean {
  return (word.match(/[A-Za-z0-9]{16,}/g) ?? []).some((run) => /[A-Za-z]/.test(run) && /\d/.test(run));
}

/** A word that is a path: a `/` or a backslash, and not a web address. */
function pathLike(word: string): boolean {
  return /[/\\]/.test(word) && /[\p{L}\p{N}]/u.test(word) && (!word.includes("://") || word.startsWith("file://"));
}

/** A line of only paths or pasted keys (an `@"/Users/…/brief.md"`
 * mention, a dragged file, a pasted token): nothing to name a session by,
 * nor for a name to show. */
function saysNothing(line: string): boolean {
  const words = line
    .replace(/\\ /g, "_")
    .split(/\s+/)
    .map((word) => word.replace(/^[@"'`*<>()[\],;]+|[@"'`*<>()[\],;]+$/g, ""))
    .filter(Boolean);
  return words.length > 0 && words.every((word) => pathLike(word) || tokenLike(word));
}

/** The first line that says something. */
function spokenLine(text: string): string | null {
  return (
    text
      .split("\n")
      .map((part) => part.trim())
      .find((line) => line && !saysNothing(line)) ?? null
  );
}

/** `line` up to a redaction mark, then up to a word that looks like a key. */
function kept(line: string): string {
  const unmarked = line.split(/•{2,}|\[redacted\]/i)[0]!;
  let at = 0;
  for (const part of unmarked.match(/\S*\s*/g) ?? []) {
    if (!part) break;
    if (tokenLike(part)) return unmarked.slice(0, at);
    at += part.length;
  }
  return unmarked;
}

/** The first words of what was asked, one line: its first line that says
 * something (not only paths or pasted keys), whitespace collapsed, cut
 * before a redaction mark or a key, its first clause past a short opener
 * ("Good.", "OK, thanks."), at most 60 characters cut at a word, "…" when
 * cut short. A colon doesn't end a clause: a tool's own title ("Billing
 * move: rehearsal") reads whole. `naming::first_words` is the same. */
export function firstWords(text: string): string | null {
  const line = spokenLine(text);
  if (!line) return null;
  const clean = kept(line).replace(/[`*_#>]/g, "").replace(/\s+/g, " ").trim();
  const clauses = clean.split(/(?<=[.;!?])\s+/).filter(Boolean);
  while (clauses.length > 1 && clauses[0]!.split(" ").length < 3) clauses.shift();
  const clause = (clauses[0] ?? "").replace(/[\s,;:–—-]+$/, "");
  if (clause.length < 3) return null;
  let out = clause;
  if (out.length > NAME_CHARS) {
    const head = out.slice(0, NAME_CHARS);
    // A whole word that ends right at the limit stays.
    out = /\s/.test(out.charAt(NAME_CHARS)) ? head.trimEnd() : head.replace(/\s+\S*$/, "");
  }
  const cut = out.length < clause.replace(/[.!?]+$/, "").length;
  out = out.replace(/[.,;:!]+$/, "");
  return cut ? `${out}…` : out;
}

/** What a title says as a name: its first words, a short one ("ok") as it
 * is, `null` when it says nothing but a path or a key
 * (`naming::title_words`). */
function titleWords(title: string): string | null {
  const words = firstWords(title);
  if (words) return words;
  const line = spokenLine(title);
  return (line && kept(line).replace(/\s+/g, " ").trim()) || null;
}

/** When it started, in a few words (`pastWhen`): "started 1:14 AM"
 * today, "started yesterday 1:14 AM", "started Thu 1:14 AM" within the
 * week, and with its date from a week back ("started Thu, Oct 23 1:14
 * AM"; its year when not this one's). */
export function startedWords(ms: number, now: number): string {
  return `started ${pastWhen(ms, now)}`;
}

/** What an untitled session is told apart by (D-96): its folder (or
 * project) and when it started. Never its App or Account (D-103.8): its
 * row says which in its chip or on its second line. */
export interface SessionFacts {
  folder: string | null;
  startedMs: number | null;
}

/** D-96 (persona D2), D-103.8: an untitled session's name (one with no
 * first prompt, 0.4.5), by its facts —
 * "billing · started 1:14 AM", "billing · started yesterday 9:02 PM" (its
 * day as `pastWhen` says it), never "Untitled session · billing", which
 * two sessions in one folder share, and never its Account (persona R2:
 * "client-b · Client A · client-a · started 4:46 PM" said it twice, and
 * the rail's width cut it off). "Session started 1:14 AM" when only the
 * time is known; "Untitled session" when nothing is. */
export function untitledName(facts: SessionFacts, now: number): string {
  const lead = facts.folder || null;
  const when = facts.startedMs != null && facts.startedMs > 0 ? startedWords(facts.startedMs, now) : null;
  if (lead === null) return when === null ? "Untitled session" : `Session ${when}`;
  return when === null ? lead : `${lead} · ${when}`;
}

/** Names that would read alike, told apart the way a pool's cards are:
 * "(1)", "(2)" by when each started (then by key), so the same minute in
 * the same folder still reads as two. Rows whose names differ are left as
 * they are. */
export function tellApart<T>(rows: readonly T[], name: (row: T) => string, order: (row: T) => [number, string]): Map<T, string> {
  const numbers = numberAlike(rows, name, order);
  return new Map(rows.map((row) => [row, numbers.get(row) ? `${name(row)} (${numbers.get(row)})` : name(row)] as const));
}

/** The number each row is told apart by among the rows whose `key` is
 * the same (1, 2… by `order`), `null` for a row no other shares it with:
 * [`tellApart`]'s count, for a name said apart from its key (a canvas
 * node's title, its start under it). */
export function numberAlike<T>(rows: readonly T[], key: (row: T) => string, order: (row: T) => [number, string]): Map<T, number | null> {
  const groups = new Map<string, T[]>();
  for (const row of rows) groups.set(key(row), [...(groups.get(key(row)) ?? []), row]);
  const out = new Map<T, number | null>();
  for (const same of groups.values()) {
    if (same.length === 1) {
      out.set(same[0]!, null);
      continue;
    }
    const sorted = [...same].sort((a, b) => order(a)[0] - order(b)[0] || order(a)[1].localeCompare(order(b)[1]));
    sorted.forEach((row, at) => out.set(row, at + 1));
  }
  return out;
}

/** The last part of a working folder ("billing"). */
export function folderName(cwd: string | null | undefined): string {
  const path = cwd ?? "";
  return path.replace(/\/+$/, "").split("/").pop() || path;
}

/** Whether a session is named by its facts (D-96): no label, no title
 * with words to say, no branch that says what the work was. Its name has
 * its start time in it already. */
export function namedByFacts(session: { label?: string | null; title?: string | null; git_branch?: string | null }): boolean {
  const title = given(session.title);
  return !given(session.label) && (title === null || titleWords(title) === null) && tellingBranch(session.git_branch) === null;
}

/** `text` unless it's blank (the backend's rule: "  " names nothing). */
function given(text: string | null | undefined): string | null {
  return text?.trim() ? text : null;
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
 * session's header, the palette): its label, else its title's words (the
 * tool's own name for it, else the first words of its first prompt,
 * 0.4.5), else a branch that says what the work was ("fix/limits"), else
 * its facts (D-96, D-103.8: "billing · started 1:14 AM") — never the whole
 * first prompt, never "main" (the rc7 and rc12 walks), never "Untitled
 * session · <folder>" (persona D2), never a pasted key or a bare path. The
 * member card's name (`card_name`) and the backend's `naming::short_name`
 * are the same rule, held to it by one file of cases both run
 * (`fixtures/session-names.json`). */
export function shortName(session: Pick<InstanceInfo, "label" | "title" | "git_branch" | "cwd" | "started_at_ms">, now: number = Date.now()): string {
  const label = given(session.label);
  if (label) return label;
  const title = given(session.title);
  const words = title === null ? null : titleWords(title);
  if (words) return words;
  return tellingBranch(session.git_branch) ?? untitledName({ folder: folderName(session.cwd) || null, startedMs: session.started_at_ms }, now);
}

/** Sessions named in a sentence by the one short-name rule
 * ([`shortName`]), each once: "fix/limits and billing · started 1:14
 * AM"; two untitled ones that read alike are told apart as the rail
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

/** Every session's one name, kept for the list it was asked for. */
const named = new WeakMap<readonly NamedSession[], ReadonlyMap<string, string>>();

/** SK48-3 (the 0.4.8 soak, P7): each session's one name wherever it's
 * said ([`namesApart`] over every session the app holds: the rail's
 * rule). A pool's canvas, its List view, its sealed line, the seal's ask,
 * Who's in and Add a session read it here, so "client · started 8:25 AM
 * (1)" is that in each, never told apart again within one pool's few
 * (web's List said it without its "(1)"). Asked again for the same list,
 * the same map. */
export function sessionNames(sessions: readonly NamedSession[]): ReadonlyMap<string, string> {
  const known = named.get(sessions);
  if (known) return known;
  const names = namesApart(sessions);
  named.set(sessions, names);
  return names;
}
