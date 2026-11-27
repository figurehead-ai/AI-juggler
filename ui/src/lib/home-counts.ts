/**
 * Home's counts line, beside its title (D-118's mock: "7 open · 6
 * finished today · 1 to review"), then what else is set aside (inactive
 * sessions, an App's hidden ones), each part a link to where they are.
 * A part at 0 is left out.
 */
import { showInFeed } from "./feed-state";
import { sessionState } from "./rail-items";
import { showInRail } from "./rail-view";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** 0.5.2 G73 (RW1 Part T, T4): AI Juggler's sessions working now, as their
 * rows read (`sessionState`, the rail's rule), for Home's Running chip: it
 * counted every stored "running" one, a session whose turn had ended (Idle
 * on the rail) too. The backend's equal is
 * `sessions::read_model::working_now`. */
export function runningNow(rows: readonly InstanceInfo[]): number {
  return rows.filter((row) => sessionState(row).state === "working").length;
}

export interface HomeCount {
  key: string;
  words: string;
  run: () => void;
}

const sessionsWords = (count: number, what: string) => `${count} ${what} ${count === 1 ? "session" : "sessions"}`;

export function homeCounts(facts: { open: number; finishedToday: number; toReview: number; inactive: number; hidden: readonly { app: string; count: number }[] }): HomeCount[] {
  return [
    { key: "open", words: `${facts.open} open`, run: () => document.querySelector<HTMLElement>("[data-testid='rail'] [data-rail-item]")?.focus() },
    // The backend's count of today's finished sessions (the digest the
    // feed's Finished items come from): it shows the Finished chip.
    { key: "finished", words: `${facts.finishedToday} finished today`, run: () => showInFeed("finished") },
    { key: "review", words: `${facts.toReview} to review`, run: () => document.querySelector<HTMLElement>("[data-testid='home-needs']")?.scrollIntoView?.({ block: "nearest" }) },
    // "23 inactive" read as apps or anything (the maintainer, 0.3.3).
    { key: "inactive", words: sessionsWords(facts.inactive, "inactive"), run: () => showInRail("inactive") },
    ...facts.hidden.map((part) => ({ key: `hidden-${part.app}`, words: `${part.count} in ${part.app} hidden`, run: () => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "settings" } })) })),
  ].filter((part) => !part.words.startsWith("0 "));
}
