import type { PoolMemberRow } from "../../lib/ipc/pools";
import { shortName } from "../../lib/session-titles";
import { GROUP_PAST } from "./useCanvas";

/** What a pool page counts, said precisely: its members (the sessions AI
 * Juggler runs or carried in), how many of them are open, and, beside
 * them, its project's sessions open in other apps now (the 0.2 walk: "0
 * sessions" while three were open in the Claude app; the rc26 P3 walk:
 * "6 sessions in AI Juggler" with 3 open and 3 ended). SK-12: "64 sessions
 * in AI Juggler, 4 open", never "4 open … · 64 in all". */
export function poolCountWords(members: number, open: number, openElsewhere: number): string {
  // Empty, with nothing open elsewhere: its canvas says so, once (design
  // review #85).
  if (members === 0 && openElsewhere === 0) return "";
  const shown = Math.min(open, members);
  const noun = members === 1 ? "session" : "sessions";
  const own =
    members === 0
      ? "0 sessions in AI Juggler"
      : shown === members
        ? `${members} ${noun} open in AI Juggler`
        : shown === 0
          ? `${members} ${noun} in AI Juggler, none open`
          : `${members} sessions in AI Juggler, ${shown} open`;
  return openElsewhere > 0 ? `${own} · ${openElsewhere} open in other apps` : own;
}

/** Under a canvas that draws no member while its pool has some (walk K1):
 * their sessions show as they start; with sessions open but none drawn,
 * the diagram grouped them past {@link GROUP_PAST} (F-rc3g-8: never "No
 * one has joined yet" beside the header's "3 open"). */
function rulesOnlyWords(hasRules: boolean, open: number): string {
  if (open > 0) return `${open} open now, each on its member's card: past ${GROUP_PAST} the diagram groups them. Every one is in the List view.`;
  return hasRules ? "Its sessions show here as they start. Click a member for its role and what it may do." : "None of its sessions is open: they show here as they start, and every one is in the List view.";
}

/** A member a sealed pool holds out of this one: its name, and that pool's. */
export interface HeldMember {
  id: string;
  name: string;
  by: string;
}

/** AJ-E3014 item 9 (P7's rc3a follow-ups): the members a sealed pool holds
 * out of this one, from its rows (the List view shows them greyed, "in
 * Client (sealed)"). The canvas draws none of them, so the page says so. */
export function heldOf(rows: readonly PoolMemberRow[]): HeldMember[] {
  return rows.flatMap((row) => (row.held_by ? [{ id: row.session.id, name: shortName(row.session), by: row.held_by }] : []));
}

/** The pools that hold them, in words: "Client (sealed)". */
export function heldByWords(held: readonly HeldMember[]): string | null {
  const by = [...new Set(held.map((one) => one.by))];
  return by.length === 0 ? null : `${by.join(", ")} (sealed)`;
}

/** The header's count of them: "1 session in Client (sealed)". */
export function heldCountWords(held: readonly HeldMember[]): string | null {
  const by = heldByWords(held);
  return by && `${held.length} ${held.length === 1 ? "session" : "sessions"} in ${by}`;
}

/** Under a canvas that draws none of its sessions because a sealed pool
 * holds them (never a lone "No lead for now" as all it shows). */
function heldCanvasWords(held: readonly HeldMember[]): string {
  const by = heldByWords(held) ?? "a sealed pool";
  const who = held.length === 1 ? `${held[0]!.name} is` : `${held.length} of its sessions are`;
  return `${who} in ${by} for now: a sealed pool keeps its sessions to itself. The List view shows ${held.length === 1 ? "it" : "them"}.`;
}

/** A sealed pool holds every session it has, so the canvas draws none. */
export const heldOnly = (held: readonly HeldMember[], drawn: number): boolean => held.length > 0 && drawn === 0;

/** Under a canvas that draws no member: where its sessions are, when a
 * sealed pool holds them all, else what `rulesOnlyWords` says. */
export function canvasHintWords(held: readonly HeldMember[], drawn: number, hasRules: boolean, open: number): string {
  return heldOnly(held, drawn) ? heldCanvasWords(held) : rulesOnlyWords(hasRules, open);
}
