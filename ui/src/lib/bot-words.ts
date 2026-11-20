/**
 * D-136 — every sentence Bot shows, written by code from the records'
 * own states and names, never from model text (ADR-0034 §6.6). English
 * for the window; the lead follows the question's language (English and
 * German in 0.5).
 *
 * **The lead's rule** (the maintainer's note on round 1): one clause per
 * state, each with its own count, counts in bold, "and" before the last.
 * Never one word for several states: "1 session asks you, 1 looks stuck,
 * and 1 task is blocked", never "3 are waiting on you".
 */
import { agoWords, clock, dayWords, minutes } from "./time-words";
import { cliAppName } from "./app-name";
import type { BotLang } from "../types/generated/BotLang";
import type { BotLooked } from "../types/generated/BotLooked";
import type { BotRowInfo } from "../types/generated/BotRowInfo";
import type { BotScope } from "../types/generated/BotScope";
import type { BotShortcut } from "../types/generated/BotShortcut";
import type { BotState } from "../types/generated/BotState";
import type { BotUnknown } from "../types/generated/BotUnknown";

/** A piece of a sentence; `strong` is shown bold. */
export interface Piece {
  text: string;
  strong?: boolean;
}

type Kind = BotRowInfo["kind"];

/** The order clauses and rows come in: what needs you first. */
const ORDER: BotState[] = ["asks_you", "stuck", "limit", "failed", "blocked", "in_review", "working", "finished", "done", "ended", "ready", "todo", "cancelled"];

const NOUN: Record<BotLang, Record<Kind, [string, string]>> = {
  en: { session: ["session", "sessions"], task: ["task", "tasks"], account: ["account", "accounts"] },
  de: { session: ["Sitzung", "Sitzungen"], task: ["Aufgabe", "Aufgaben"], account: ["Konto", "Konten"] },
};

/** Each state's verb phrase, one and many. */
const VERB_EN: Record<BotState, [string, string]> = {
  asks_you: ["asks you", "ask you"],
  stuck: ["looks stuck", "look stuck"],
  limit: ["reached its limit", "reached their limit"],
  failed: ["stopped with an error", "stopped with an error"],
  blocked: ["is blocked", "are blocked"],
  in_review: ["waits for review", "wait for review"],
  working: ["is working", "are working"],
  finished: ["finished", "finished"],
  ended: ["ended", "ended"],
  done: ["is done", "are done"],
  ready: ["is ready", "are ready"],
  todo: ["isn’t ready yet", "aren’t ready yet"],
  cancelled: ["was dropped", "were dropped"],
};
const VERB_DE: Record<BotState, [string, string]> = {
  asks_you: ["fragt dich", "fragen dich"],
  stuck: ["hängt fest", "hängen fest"],
  limit: ["ist am Limit", "sind am Limit"],
  failed: ["ist mit einem Fehler beendet", "sind mit einem Fehler beendet"],
  blocked: ["ist blockiert", "sind blockiert"],
  in_review: ["wartet auf Prüfung", "warten auf Prüfung"],
  working: ["arbeitet", "arbeiten"],
  finished: ["ist fertig", "sind fertig"],
  ended: ["wurde beendet", "wurden beendet"],
  done: ["ist erledigt", "sind erledigt"],
  ready: ["ist bereit", "sind bereit"],
  todo: ["ist noch nicht bereit", "sind noch nicht bereit"],
  cancelled: ["wurde verworfen", "wurden verworfen"],
};
/** A task that is working is "in progress". */
const TASK_WORKING: Record<BotLang, [string, string]> = { en: ["is in progress", "are in progress"], de: ["ist in Arbeit", "sind in Arbeit"] };

const FOUND: Record<BotLang, string> = { en: "Here’s what I found", de: "Das habe ich gefunden" };
const ABOUT: Record<BotLang, string> = { en: "about", de: "zu" };
const AND: Record<BotLang, string> = { en: "and", de: "und" };
const NOTHING: Record<BotLang, string> = { en: "I found nothing that matches right now.", de: "Dazu habe ich gerade nichts gefunden." };

/** The state a row's lead counts: how it stood when asked. */
function leadState(row: BotRowInfo): BotState | null {
  return row.then_state ?? row.state;
}

/** The lead sentence over the rows, by code: one clause per state. */
export function lead(rows: readonly BotRowInfo[], lang: BotLang, about?: BotScope | null): Piece[] {
  if (rows.length === 0) return [{ text: NOTHING[lang] }];
  const groups = new Map<string, { kind: Kind; state: BotState | null; n: number }>();
  for (const row of rows) {
    const state = leadState(row);
    const key = `${row.kind}:${state ?? "-"}`;
    const group = groups.get(key) ?? { kind: row.kind, state, n: 0 };
    group.n += 1;
    groups.set(key, group);
  }
  const rank = (state: BotState | null) => (state === null ? ORDER.length : ORDER.indexOf(state));
  const kindRank = (kind: Kind) => ["session", "task", "account"].indexOf(kind);
  const ordered = [...groups.values()].sort((a, b) => rank(a.state) - rank(b.state) || kindRank(a.kind) - kindRank(b.kind));
  const named = new Set<Kind>();
  const clauses: Piece[][] = ordered.map((group) => {
    const many = group.n === 1 ? 0 : 1;
    const noun = NOUN[lang][group.kind][many];
    const count = named.has(group.kind) ? `${group.n}` : `${group.n} ${noun}`;
    named.add(group.kind);
    if (group.state === null) return [{ text: count, strong: true }];
    const verb = group.kind === "task" && group.state === "working" ? TASK_WORKING[lang][many] : (lang === "de" ? VERB_DE : VERB_EN)[group.state][many];
    return [{ text: count, strong: true }, { text: ` ${verb}` }];
  });
  const head: Piece[] = about ? [{ text: `${FOUND[lang]} ${ABOUT[lang]} ` }, { text: `“${about.name}”`, strong: true }, { text: ": " }] : [{ text: `${FOUND[lang]}: ` }];
  const out: Piece[] = [...head];
  clauses.forEach((clause, i) => {
    if (i > 0) {
      const last = i === clauses.length - 1;
      const comma = clauses.length > 2 && lang === "en" ? "," : "";
      out.push({ text: last ? `${comma} ${AND[lang]} ` : ", " });
    }
    out.push(...clause);
  });
  out.push({ text: "." });
  return out;
}

/** The lead as plain words (tests, the screen reader's announcement). */
export function leadText(pieces: readonly Piece[]): string {
  return pieces.map((p) => p.text).join("");
}

/** A row's state in the status column. */
export const STATE_WORDS: Record<BotState, string> = {
  asks_you: "Asks you",
  stuck: "Looks stuck",
  limit: "Limit reached",
  failed: "Stopped with an error",
  blocked: "Blocked",
  in_review: "Waits for review",
  working: "Working",
  finished: "Finished",
  ended: "Ended",
  done: "Done",
  ready: "Ready",
  todo: "Not ready yet",
  cancelled: "Dropped",
};

/** The level a state's words are drawn at (D-117: amber only for what needs you). */
export function stateLevel(state: BotState | null): 0 | 1 | 2 {
  return state === "asks_you" || state === "stuck" || state === "limit" ? 2 : state === "failed" ? 1 : 0;
}

/** The second line under a row's state: how long, or when. */
export function stateSub(row: BotRowInfo, now: number): string {
  if (row.kind === "account") return row.resets_at_ms ? `resets ${resetDay(row.resets_at_ms, now)}` : "";
  const at = row.at_ms ?? 0;
  switch (row.state) {
    case "asks_you":
      return at ? `waiting ${minutes(at, now)}` : "";
    case "stuck":
      return at ? `no output ${minutes(at, now)}` : "";
    case "working":
      return at ? minutes(at, now) : "";
    case "blocked":
      return row.blocked_by ? `waiting on ${row.blocked_by} ${row.blocked_by === 1 ? "task" : "tasks"}` : at ? whenShort(at, now) : "";
    default:
      return at ? whenShort(at, now) : "";
  }
}

/** "14:32", "yesterday 16:20", or the day. */
function whenShort(ms: number, now: number): string {
  const day = dayWords(ms, now);
  if (day === "Today") return clock(ms);
  if (day === "Yesterday") return `yesterday ${clock(ms)}`;
  return day;
}

function resetDay(ms: number, now: number): string {
  const day = dayWords(ms, now);
  if (day === "Today") return clock(ms);
  if (day === "Yesterday") return clock(ms);
  return new Date(ms).toLocaleDateString([], { weekday: "short" });
}

/** "Session · Agent app one · shop", "Task · Release pool · builder-2". */
export function rowPlace(row: BotRowInfo): string {
  if (row.kind === "session") {
    const app = row.desktop ? (row.app_label ?? "Desktop app") : cliAppName(row.provider ?? "", row.app_label);
    return ["Session", app, row.project].filter(Boolean).join(" · ");
  }
  if (row.kind === "task") return ["Task", row.pool, row.holder].filter(Boolean).join(" · ");
  return ["Account", row.window === "weekly" ? "this week" : row.window === "five_hour" ? "5-hour window" : null].filter(Boolean).join(" · ");
}

/** What a row's link opens, for its label ("Open web’s page"). */
export function rowLabel(row: BotRowInfo, now: number): string {
  const state = row.state ? STATE_WORDS[row.state] : row.used_percent !== null ? `${row.used_percent} % used` : "";
  const gone = row.gone ? "No longer here" : "";
  const shown = `${row.name_hidden ? "A record with a hidden name" : row.title || "A record"} ${rowPlace(row)}`;
  return [shown, gone || state, stateSub(row, now)].filter(Boolean).join(", ");
}

/** The words after "Looked at ·". */
export function lookedWords(looked: BotLooked, atMs: number, now: number): string {
  const when = now - atMs < 90_000 ? "just now" : agoWords(atMs, now);
  const scope = looked.scope;
  const memory = looked.memories > 0 ? `, and ${looked.memories === 1 ? "1 memory" : `${looked.memories} memories`}` : "";
  let what: string;
  switch (looked.reach) {
    case "scope":
    case "subject":
      what = scope ? scopeWords(scope, looked.reach === "scope") : "every session, task and pool";
      break;
    case "accounts":
      what = "every account";
      break;
    case "searched":
      return "sessions, tasks, pools and usage";
    default:
      what = "every session, task and pool";
  }
  return `${what}${memory} · ${when}`;
}

/** "api only", "every session in Agent app one", "shop", "the session api". */
function scopeWords(scope: BotScope, chip: boolean): string {
  switch (scope.kind) {
    case "session":
      return chip ? `${scope.name} only` : `the session ${scope.name}`;
    case "app":
      return `every session in ${scope.name}`;
    case "project":
      return scope.name;
    case "pool":
      return `every task in ${scope.name}`;
    case "account":
      return `${scope.name}’s usage`;
    default:
      return scope.name;
  }
}

/** "I don't know" (the bold part first, then why). */
export function unknownWords(unknown: BotUnknown): { strong: string; rest: string } {
  switch (unknown) {
    case "nothing_found":
      return { strong: "I don’t know:", rest: " I couldn’t find anything about that in AI Juggler." };
    case "test_durations":
      return { strong: "I don’t know.", rest: " AI Juggler doesn’t record how long tests take." };
    case "test_results":
      return { strong: "I don’t know.", rest: " AI Juggler doesn’t record a session’s test results." };
    case "costs":
      return { strong: "I don’t know.", rest: " I don’t look up costs; Usage shows them." };
    case "authors":
      return { strong: "I don’t know.", rest: " AI Juggler doesn’t record who wrote what." };
    default:
      return { strong: "I don’t know.", rest: "" };
  }
}

/** What the nearest record is, under "I don't know". */
export function nearestWords(unknown: BotUnknown, row: BotRowInfo): { lead: string; name: string; tail: string } {
  const what: Record<BotUnknown, string> = {
    nothing_found: "",
    test_durations: "its test times",
    test_results: "its test results",
    costs: "its costs",
    authors: "who wrote it",
  };
  return { lead: "The nearest record is about the ", name: `${row.kind} “${row.title}”`, tail: `, not ${what[unknown]}:` };
}

/** A shortcut to the screen that already answers it (no AI needed). */
export const SHORTCUTS: Record<BotShortcut, { title: string; where: string }> = {
  needs_me: { title: "What needs me", where: "Needs you, in the rail" },
  finished_today: { title: "What finished today", where: "Home, filtered to Finished" },
  runway: { title: "Runway by account", where: "Home’s Runway column" },
  running: { title: "What’s running", where: "Open now" },
  usage: { title: "Usage", where: "Every account’s usage and costs" },
};

export { HELP, OUT_OF_SCOPE, WORDS } from "./bot-ui-words";
export { CLEAR_AND_BACKUPS, FORGET_AND_BACKUPS } from "./bot-backup-words";

/** The day a divider names: "Today", "Yesterday", or the date. */
export function dayDivider(ms: number, now: number): string {
  return dayWords(ms, now);
}

/** The day key two turns share when they're on the same local day. */
export function dayKey(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}
