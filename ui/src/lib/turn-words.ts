/**
 * F-rc3f-5 (0.4.6 rc3f) — a driven session's turn that didn't finish, in
 * words. When a tool says nothing else, it names such a turn by its own
 * code, and after "Deny and stop" the conversation showed that code while
 * the line under the message box kept saying it after the next turn had
 * gone well.
 *
 * AJ-E2025: the tool's codes are its adapter's. A note that ended its
 * turn carries a neutral kind (`turn_end`; the session's last error,
 * `error_end`), and this file says the kind in words: a turn the user
 * stopped says what they chose, any other kind says what happened, a
 * note with no kind is the tool's own sentence and stays as it is (a bare
 * code there reads "Stopped with an error"). The line under the box is
 * the latest turn's only: it goes once a new turn runs or answers (the
 * tool's last error stays set until another one).
 *
 * Who stopped a turn only this window knows: it keeps the user's own
 * Deny and stop and Interrupt per session, in memory, while it is open.
 */
import type { ConversationItemInfo } from "../types/generated/ConversationItemInfo";
import type { RuntimeInfo } from "../types/generated/RuntimeInfo";

/** How the user stopped a turn. */
export type StopHow = "deny_and_stop" | "interrupt";
type Stop = { how: StopHow; at_ms: number };

/** The kind of a turn that was stopped before it finished. */
const STOPPED = "stopped";

/** Why a turn ended early, by its neutral kind, in words. */
const END_WORDS: Record<string, string> = {
  [STOPPED]: "Stopped before it finished",
  turn_limit: "Stopped: it reached the most turns it may take",
  spend_limit: "Stopped: it reached its spending limit",
  answer_form: "Stopped: it couldn't give its answer in the form asked for",
  failed: "Stopped with an error",
};

const STOP_WORDS: Record<StopHow, string> = {
  deny_and_stop: "Stopped: you chose Deny and stop",
  interrupt: "Stopped: you interrupted it",
};

/** A bare snake_case code, never a sentence. */
const BARE_CODE = /^[a-z][a-z0-9]*(?:_[a-z0-9]+)+$/;

/** The user's stops this window saw, by session, oldest first. */
const stops = new Map<string, Stop[]>();
/** Stops kept per session. */
const KEPT = 20;

/** The user stopped `sessionId`'s turn (their click, as it is sent);
 * the returned call forgets it again (the stop didn't reach the tool). */
export function noteStop(sessionId: string, how: StopHow): () => void {
  const stop: Stop = { how, at_ms: Date.now() };
  stops.set(sessionId, [...(stops.get(sessionId) ?? []), stop].slice(-KEPT));
  return () => stops.set(sessionId, (stops.get(sessionId) ?? []).filter((one) => one !== stop));
}

/** Tests: forget every stop. */
export function forgetStops(): void {
  stops.clear();
}

/** A tool's error or note in words, by why its turn ended when it says
 * (`end`): the user's stop by what they chose. */
function turnWords(text: string, end: string | null | undefined, how: StopHow | null = null): string {
  if (end === STOPPED && how) return STOP_WORDS[how];
  if (end) return END_WORDS[end] ?? END_WORDS.failed!;
  return BARE_CODE.test(text.trim()) ? END_WORDS.failed! : text;
}

/** What the latest turn's error leaves under the message box: its words,
 * and whether the user stopped it (said calmly then). */
export type StandingError = { text: string; yours: boolean };

/** A driven session's conversation with each note in words, and its
 * latest turn's error while nothing came after it. A stop belongs to the
 * first stopped turn's note at or after it. */
export function turnNotes(sessionId: string, runtime: Pick<RuntimeInfo, "error" | "error_end" | "state"> | null, items: readonly ConversationItemInfo[]): { items: ConversationItemInfo[]; standing: StandingError | null } {
  const mine = stops.get(sessionId) ?? [];
  let after = -Infinity;
  const told = items.map((item) => {
    if (item.kind !== "note") return { item, how: null };
    let how: StopHow | null = null;
    if (item.turn_end === STOPPED) {
      const since = after;
      how = [...mine].reverse().find((stop) => stop.at_ms > since && stop.at_ms <= item.at_ms)?.how ?? null;
      after = item.at_ms;
    }
    const text = turnWords(item.text, item.turn_end, how);
    return { item: text === item.text ? item : { ...item, text }, how };
  });
  const plain = told.map((one) => one.item);
  const raw = runtime?.error?.trim() ?? "";
  const end = runtime?.error_end ?? null;
  // A new turn runs (or asks): the error was the one before.
  if ((!raw && !end) || runtime?.state === "working" || runtime?.state === "asks_you") return { items: plain, standing: null };
  let at = items.length - 1;
  while (at >= 0 && !(items[at]!.kind === "note" && items[at]!.text.trim() === raw)) at -= 1;
  // Its note scrolled out of what's held: an older turn's.
  if (at < 0) return { items: plain, standing: items.length === 0 ? { text: turnWords(raw, end), yours: false } : null };
  if (items.slice(at + 1).some((item) => item.kind !== "note")) return { items: plain, standing: null };
  return { items: plain, standing: { text: plain[at]!.text, yours: told[at]!.how !== null } };
}
