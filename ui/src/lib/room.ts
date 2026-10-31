/**
 * The one order by room (the 0.2 walk, F1 and F17): Home's "Best for new
 * work", the switch's targets (Continue on another account, the switch
 * card, the limit dialog) and New session's accounts all rank by the room
 * the one runway reads (`runwayOf`), and mark the same one "Suggested".
 * D-45 lets the user's own sheets suggest by room; nothing is picked for
 * them.
 *
 * The order: accounts with room, most first; then ones not measured yet
 * (their room isn't known, so they're never suggested); then low ones,
 * most first; then used up but carrying on with credits (0.3.2: a session
 * there goes on, paid by its credits; never suggested); then out of room;
 * then ones that can't take work now (signed out, the same account). Out
 * of room is listed, never hidden. Pure: the callers hand in each one's
 * room.
 */
import { whenWords } from "./time-words";

/** At or under this much left, an account is low: never suggested. */
export const LOW_LEFT = 25;

/** Where an account stands, by its room. `credits`: a window used up,
 * but its credits carry the work on (`lib/credits`). */
export type RoomKind = "room" | "unknown" | "low" | "credits" | "out" | "blocked";

const ORDER: Record<RoomKind, number> = { room: 0, unknown: 1, low: 2, credits: 3, out: 4, blocked: 5 };

/** An account's kind, from the % it has left (`null`: not measured) and
 * whether its credits carry it on past a used-up window. */
export function roomKind(left: number | null, usable = true, credits = false): RoomKind {
  if (!usable) return "blocked";
  if (credits) return "credits";
  if (left === null) return "unknown";
  if (left <= 0) return "out";
  return left <= LOW_LEFT ? "low" : "room";
}

export interface ByRoom<T> {
  item: T;
  left: number | null;
  kind: RoomKind;
  /** The one with the most room, when one has room (over 25% left). */
  suggested: boolean;
}

/** `items` in the one order by room, the best marked. Stable: ties and
 * the unmeasured keep the order they came in. */
export function rankByRoom<T>(
  items: readonly T[],
  leftOf: (item: T) => number | null,
  usable: (item: T) => boolean = () => true,
  /** Its credits carry it on past a used-up window. */
  credits: (item: T) => boolean = () => false,
): ByRoom<T>[] {
  const rows = items.map((item, index) => {
    const left = leftOf(item);
    return { item, left, kind: roomKind(left, usable(item), credits(item)), index };
  });
  rows.sort((a, b) => {
    const kind = ORDER[a.kind] - ORDER[b.kind];
    if (kind !== 0) return kind;
    const measured = a.kind === "room" || a.kind === "low";
    return measured && a.left !== b.left ? (b.left ?? 0) - (a.left ?? 0) : a.index - b.index;
  });
  return rows.map(({ item, left, kind }, index) => ({ item, left, kind, suggested: index === 0 && kind === "room" }));
}

/** "Out of room until 3:10 PM", "… until 8:00 AM tomorrow", "… until
 * Thu 8:00 AM" (or "Out of room for now" when the reset isn't known). */
export function outOfRoomWords(resetsAtMs: number | null | undefined, now: number = Date.now()): string {
  return resetsAtMs != null ? `Out of room until ${whenWords(resetsAtMs, now)}` : "Out of room for now";
}

/** What picking an account out of room does, in one line. */
export function outOfRoomConsequence(name: string, resetsAtMs: number | null | undefined, now: number = Date.now()): string {
  const until = resetsAtMs != null ? ` until ${whenWords(resetsAtMs, now)}` : " until its limit resets";
  return `You can still pick it, but the conversation stops at ${name}'s limit straight away and waits there${until}.`;
}

/** A low account, in one line. */
export function lowRoomWords(name: string, left: number): string {
  return `${name} has only ${left}% left: the conversation may stop at its limit soon.`;
}

/** "Why this order?", said once for every list that uses it. */
export const ROOM_ORDER_WORDS =
  "By room left, as Home's Runway reads it: most first, then accounts not measured yet, then low ones, then any using credits, then any out of room. Suggested is the one with the most room, over 25% left. Under each: what else AI Juggler weighed.";
