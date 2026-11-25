/**
 * AJ-E2326 R4 and R5 — every live place of a conversation AI Juggler's
 * session holds is shown. The list lets AI Juggler's running session
 * stand for its conversation (one row, its session's); but when the
 * tool's own desktop app opens that conversation too (its Code child
 * resumes it), the desktop App's group had no row for it and read
 * "nothing open" while its child held it (open now means running).
 *
 * The desktop place is the backend's (`conversations::places::
 * session_host`: a Code child is its App's only by the shared ownership
 * module's evidence, AJ-E2315), never worked out here. Here: that place
 * gets the conversation's row too, and while AI Juggler's session runs it
 * as well, both rows say "Open in two places", naming the other place,
 * with AI Juggler's session to stop (the other app's process is never
 * touched, D-28).
 */
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";

/** Both rows' words. */
export const TWO_PLACES = "Open in two places";

/** A conversation live both in AI Juggler's session and another app. */
export interface TwoPlaces {
  /** AI Juggler's session on it: what "Stop ours" stops. */
  ours: InstanceInfo;
  /** The other place, in words ("Claude app · b-work", "AI Juggler"). */
  alsoIn: string;
}

/** What the rail needs of an item here (rail-items' `RailItem`). */
interface Item {
  key: string;
  session: InstanceInfo | null;
  need: unknown;
  state: string;
  twoPlaces?: TwoPlaces | null;
}

/** A conversation AI Juggler's (terminal) session stands for that a
 * desktop app's own process holds too: that place, and whether AI
 * Juggler's session runs it as well (any other place on it). */
function heldInApp(c: ConversationInfo, sessions: ReadonlyMap<string, InstanceInfo>): { ours: InstanceInfo; app: PlaceInfo; two: boolean } | null {
  if (!c.live || c.session_id === null) return null;
  const ours = sessions.get(c.session_id);
  // A desktop session AI Juggler started is that app's row already.
  if (!ours || ours.kind === "desktop" || ours.went_on_in) return null;
  const places = c.places ?? [];
  const app = places.find((place) => place.kind === "desktop");
  if (!app) return null;
  return { ours, app, two: places.length > 1 };
}

/** `items` with every live place shown: each conversation a desktop app
 * holds besides AI Juggler's session gets its row under that App
 * (`mirror`, with that place first), and both rows the note when both
 * run it. The mirrored row never needs you: the session's row carries
 * its need (one list, one count). */
export function withEveryPlace<T extends Item>(items: T[], sessions: readonly InstanceInfo[], conversations: readonly ConversationInfo[], mirror: (c: ConversationInfo) => T, ownApp: string): T[] {
  const byId = new Map(sessions.map((row) => [row.id, row]));
  const noted = new Map<string, TwoPlaces>();
  const added: T[] = [];
  for (const c of conversations) {
    const held = heldInApp(c, byId);
    if (!held) continue;
    const places = [held.app, ...(c.places ?? []).filter((place) => place !== held.app)];
    const row = mirror({ ...c, places, pid: held.app.pid });
    const two = held.two ? { ours: held.ours, alsoIn: ownApp } : null;
    // Calm: what asks or waits for review is said once, on the session's row.
    const state = row.state === "asks_you" || row.state === "done_review" ? "idle" : row.state;
    added.push({ ...row, need: null, state, twoPlaces: two, ...(two ? { alsoIn: [] } : {}) });
    if (two) noted.set(`s:${held.ours.id}`, { ours: held.ours, alsoIn: held.app.label || held.app.shown || held.app.app });
  }
  if (added.length === 0) return items;
  return [...items.map((item) => (noted.has(item.key) ? { ...item, twoPlaces: noted.get(item.key)! } : item)), ...added];
}
