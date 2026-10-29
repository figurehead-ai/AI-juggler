/**
 * AJ-E2404 — the Processes view's words: one vocabulary of states (an
 * icon, a word and a tone, never colour alone), roles as a guess
 * ("probably a dev server"), whose a process is, and the plain words for a
 * zombie. Every surface that shows a process (the page, a session's
 * Processes tab, This Mac) reads them from here.
 */
import {
  IconStateFailed,
  IconStateFinished,
  IconStateRunning,
  IconStateStopped,
  IconStateUnknown,
  IconStateWaiting,
} from "../components/icons";
import { IconShield, IconAlert } from "../components/icons";
import { IconReplay } from "../components/icons-actions";

export type ProcessTone = "neutral" | "live" | "warning" | "danger" | "accent";

export interface ProcessStateWords {
  /** The chip's word. */
  label: string;
  /** What it means, in a sentence (the chip's title and the help). */
  hint: string;
  Icon: typeof IconStateRunning;
  tone: ProcessTone;
}

/** The plain words for a zombie, where one shows. */
export const ZOMBIE_WORDS =
  "It has finished, but its parent hasn't collected it yet. It uses no memory or CPU, and only its parent can clear it: by collecting it, or by ending.";

const STATES: Record<string, ProcessStateWords> = {
  working: { label: "Working", hint: "Running and busy now.", Icon: IconStateRunning, tone: "live" },
  idle: { label: "Idle", hint: "Running, quiet now.", Icon: IconStateStopped, tone: "neutral" },
  left_behind: {
    label: "Left behind",
    hint: "Still running after its session ended.",
    Icon: IconAlert,
    tone: "warning",
  },
  left_behind_closed: {
    label: "Left behind while closed",
    hint: "Its session ended while AI Juggler was closed; it is still running.",
    Icon: IconAlert,
    tone: "warning",
  },
  picked_up: {
    label: "Picked up after restart",
    hint: "Running since before AI Juggler restarted; picked up again, never stopped.",
    Icon: IconReplay,
    tone: "accent",
  },
  zombie: { label: "Waiting for its parent", hint: ZOMBIE_WORDS, Icon: IconStateWaiting, tone: "neutral" },
  didnt_stop: {
    label: "Didn't stop",
    hint: "It was asked to stop and is still running.",
    Icon: IconStateFailed,
    tone: "danger",
  },
  wont_touch: {
    label: "Won't be touched",
    hint: "AI Juggler can't confirm it's the process it started, so it never stops it.",
    Icon: IconShield,
    tone: "neutral",
  },
  ended: { label: "Ended", hint: "It has ended. Kept here 7 days.", Icon: IconStateFinished, tone: "neutral" },
  // Sessions' own words.
  ended_session: { label: "Ended", hint: "The session has ended.", Icon: IconStateFinished, tone: "neutral" },
};

/** A state's words (an unknown word reads "Unknown", never a blank). */
export function processState(word: string): ProcessStateWords {
  return STATES[word] ?? { label: "Unknown", hint: "AI Juggler doesn't know this state.", Icon: IconStateUnknown, tone: "neutral" };
}

/** A session's state, in the same words. */
export function sessionState(word: string): ProcessStateWords {
  return word === "ended" ? STATES.ended_session! : processState(word);
}

/** The states the State filter offers, in its order. */
export const STATE_FILTERS = ["left_behind", "left_behind_closed", "didnt_stop", "zombie", "wont_touch", "picked_up", "working", "idle"] as const;

/** A role in words: sure for the session itself and AI Juggler's own,
 * a guess for the rest ("probably a dev server"). */
export function roleWords(role: string, words: string, sure: boolean): string {
  if (role === "tool") return "the session itself";
  if (sure || !words) return words;
  const article = /^[aeiou]/i.test(words) || words.startsWith("MCP") ? "an" : "a";
  return `probably ${article} ${words}`;
}

/** Ports as a row shows them (":5173, :8000"). */
export function portWords(ports: readonly number[]): string {
  return ports.map((port) => `:${port}`).join(", ");
}

/** An age in short words: "40 s", "12 min", "3 h", "2 d". */
export function ageWords(ms: number): string {
  if (ms < 60_000) return `${Math.max(0, Math.round(ms / 1000))} s`;
  if (ms < 3_600_000) return `${Math.round(ms / 60_000)} min`;
  if (ms < 2 * 86_400_000) return `${Math.round(ms / 3_600_000)} h`;
  return `${Math.round(ms / 86_400_000)} d`;
}

/** CPU as a row shows it ("4 %", "0.3 %", "—" before it's measured). */
export function cpuWords(cpu: number | null | undefined): string {
  if (cpu === null || cpu === undefined) return "—";
  if (cpu > 0 && cpu < 1) return `${cpu.toFixed(1)} %`;
  return `${Math.round(cpu)} %`;
}

/** Who stopped it, in the history's words. */
export function actorWords(actor: string): string {
  if (actor === "user:observed") return "You (another app's session)";
  if (actor.startsWith("user:")) return "You";
  if (actor === "auto:tier-1") return "Automatic: tidy left-behind dev servers";
  if (actor === "auto:tier-2") return "Automatic: tidy after a crash";
  if (actor.startsWith("system")) return "AI Juggler, after a restart";
  return actor;
}

/** An outcome in plain words ("pid 8123: exited on TERM" → "Stopped when asked"). */
export function outcomeWords(outcome: string): string {
  const said = outcome.replace(/pid \d+: /g, "");
  if (said.includes("refused")) return "Didn't stop";
  if (said.includes("needed KILL")) return "Needed a force stop";
  if (said.includes("exited on TERM")) return "Stopped when asked";
  if (said.includes("already gone")) return "Had already ended";
  if (said.includes("outcome unknown")) return "Asked to stop; not known how it ended";
  if (said.includes("couldn't signal")) return "Couldn't be stopped from here; it may still be running";
  return said;
}
