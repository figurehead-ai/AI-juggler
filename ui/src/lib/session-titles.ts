/**
 * A title for a session nothing named (the review of 0.1.5: Home's rows
 * all read "Untitled session · payments-api"): the first words of the
 * last thing you asked it, read from the end of its transcript (already
 * redacted, and only while Last messages is on), else its branch or
 * project and when it started. Read once per conversation, after Home
 * has painted, and kept for the window's life only.
 */
import { useEffect, useState } from "react";
import { conversationsLastMessages } from "./ipc/conversations";
import { dayWords } from "./time-words";
import { tellingBranch } from "./project-words";

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

/** When it started, in a few words: "started 16:19", "started yesterday
 * 16:19", "started Thursday 23 October 16:19". */
export function startedWords(ms: number, now: number): string {
  const clock = new Date(ms).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const day = dayWords(ms, now);
  return day === "Today" ? `started ${clock}` : `started ${day === "Yesterday" ? "yesterday" : day} ${clock}`;
}

/** An untitled session's name when nothing it was asked is known: its
 * telling branch, else its place (a list with a project column leaves
 * it out), then when it started: "payments-api · started 3:56 PM",
 * "Session started yesterday 3:56 PM". */
export function untitledName(branch: string | null | undefined, place: string | null, startedMs: number, now: number): string {
  const lead = tellingBranch(branch) ?? place;
  return lead ? `${lead} · ${startedWords(startedMs, now)}` : `Session ${startedWords(startedMs, now)}`;
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
    if (w.requestIdleCallback) w.requestIdleCallback(start, { timeout: 1500 });
    else setTimeout(start, 50);
    return () => {
      alive = false;
    };
    // `wanted` stands for `rows` (a new array each render).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wanted]);
  return asked;
}
