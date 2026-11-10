/**
 * AJ-E3307 (M2, D2) — how long a session has waited on you: one rule and
 * one source for Home's Needs you, the rail, a session's header, the bell
 * and the summary of what happened while you were away (deep/11 §4.1:
 * rows tick "waiting 24m", sorted "Waiting longest"; §6.1: the inbox
 * oldest first).
 *
 * The moment it began to wait is the backend's: an AI Juggler session's
 * attention record says when its condition began (an ask's own time for
 * a tool's question, `asked_at_ms`). Never its last activity: a session
 * that asks is touched again each time its tool reports, so "last
 * activity" read "2 min" beside a question asked hours before. A session
 * another app runs reports no ask time; its last activity is when it
 * stopped to ask.
 */
import { minutes } from "./time-words";

interface WaitingSession {
  started_at_ms: number;
  last_activity_ms: number | null;
  attention?: { since_ms: number } | null;
  runtime?: { asks: readonly { asked_at_ms: number }[] } | null;
}

interface WaitingConversation {
  last_activity_ms: number | null;
}

/**
 * When it began to wait (epoch ms). Per adapter:
 *
 * - **Claude Code in AI Juggler's terminal** (hooks): the first ask's
 *   `asked_at_ms`, kept while later reports (a `Notification` after the
 *   `PermissionRequest`) repeat it; the attention record holds it
 *   (`tool_asks`, `features/runtime/driver.rs`).
 * - **A driven session** (Codex's app-server): the approval's or the
 *   question's arrival, the same way.
 * - **A terminal session without hooks or a driver**: when the
 *   supervisor's rule (`waiting_prompt`, `stuck_no_output`) or a limit
 *   (`limit_hit`, `quota_exit`) first held: its attention's `since_ms`.
 * - **A session another app runs**: its transcript's last activity (the
 *   reader reports no ask time).
 */
export function waitingSince(row: { session?: WaitingSession | null; conversation?: WaitingConversation | null; lastMs?: number }): number {
  const session = row.session;
  if (session) return session.attention?.since_ms ?? session.runtime?.asks[0]?.asked_at_ms ?? session.last_activity_ms ?? session.started_at_ms;
  return row.conversation?.last_activity_ms ?? row.lastMs ?? 0;
}

/** How long, in Needs you's words: "waiting 7 h", or "just now". */
export function waitingWords(since: number, now: number): string {
  const waited = minutes(since, now);
  return waited === "just now" ? waited : `waiting ${waited}`;
}

/** A Needs-you row as the rail sorts it (`rail-items`' `byNeed`). */
export interface NeedRow {
  need: string | null;
  lastMs: number;
  key: string;
  session?: WaitingSession | null;
  conversation?: WaitingConversation | null;
}

/** The needs that wait on you (a finished one to review, or a failure,
 * has nothing waiting: it says what happened). */
const WAITS = new Set(["asks", "limit", "stuck", "cut_off"]);

/** A Needs-you row's wait, or `null` for one that doesn't wait. */
export function needWait(item: { need: string | null; session?: WaitingSession | null; conversation?: WaitingConversation | null; lastMs?: number }, now: number): string | null {
  return item.need !== null && WAITS.has(item.need) ? waitingWords(waitingSince(item), now) : null;
}

/** The longest excerpt a row shows of what it asks (the rest on its
 * page). */
const EXCERPT = 90;

/** What a hook's ask says when its tool reported no words
 * (`features/runtime/reports.rs`, `ask_of`): not the question. */
const NOT_SAID = "It waits for your answer";

/**
 * The first line of what a session asks, when its tool reported it
 * (deep/11 §10: `Waiting {t} — “{prompt excerpt}”`); `null` when it
 * didn't ("open it to see"). Where it comes from, per adapter:
 *
 * - **Claude Code in AI Juggler's terminal** (its hooks, through
 *   `juggler-agent`): a `PermissionRequest` says the tool and what it
 *   runs ("Bash: npm test"), a `PreToolUse` of `AskUserQuestion` its
 *   question; both are its first open ask (`runtime.asks`). A
 *   `Notification` alone carries its type word, never its message (by
 *   design, `juggler-agentbridge` hook.rs): no words.
 * - **A driven session** (Codex's app-server, Beta): the approval's
 *   summary or the question's first part, from the protocol.
 * - **A terminal session without hooks or a driver**: the supervisor's
 *   `waiting_prompt` rule sees a prompt, never its words: none.
 * - **A session another app runs** (the Claude app, Codex's app, a CLI
 *   outside AI Juggler): its reader says it asks, never what: none.
 */
export function askExcerpt(session: { attention?: { rule: string; inputs: readonly string[] } | null; runtime?: { asks: readonly { summary: string }[] } | null } | null | undefined): string | null {
  const said = session?.runtime?.asks[0]?.summary ?? (session?.attention?.rule === "tool_asks" ? session.attention.inputs[0] : undefined);
  const line = said?.split("\n").map((part) => part.replace(/\s+/g, " ").trim()).find(Boolean);
  if (!line || line === NOT_SAID) return null;
  return line.length > EXCERPT ? `${line.slice(0, EXCERPT - 1).trimEnd()}…` : line;
}

/** A Needs-you row's wait and what it asks: "waiting 7 h — “Bash: npm
 * test”", or "waiting 7 h · open it to see" when its tool didn't say;
 * `null` for a row that doesn't wait. */
export function waitLine(item: { need: string | null; session?: Parameters<typeof askExcerpt>[0] & WaitingSession | null; conversation?: WaitingConversation | null; lastMs?: number }, now: number | undefined): string | null {
  const wait = now === undefined ? null : needWait(item, now);
  if (item.need !== "asks") return wait;
  const asks = askExcerpt(item.session);
  const what = asks ? `“${asks}”` : "open it to see";
  if (!wait) return what;
  return asks ? `${wait} — ${what}` : `${wait} · ${what}`;
}
