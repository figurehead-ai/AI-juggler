/**
 * The New session sheet's state: whether it's open, and the folder and
 * account it opens on. `aij:new-session` opens it from anywhere; 0.3.4: its
 * `detail.accountId` names the account (a switch target's "Start a session
 * on it", an inactive CLI app's New session), and `detail.dir` the folder
 * (a pool's "Start one now", AJ-E3011 round 2). D-96 F: opened from "Start
 * another", it also holds the starts just before (`after`).
 */
import { useLayoutEffect, useState } from "react";
import { dismissToast } from "../components/Toast";

/** The starts just before this one, in one run of Start another. */
export interface StartedBefore {
  /** Their folders: never offered again in this run. */
  roots: string[];
  /** The last one's folder name ("as you chose for billing"). */
  name: string;
  /** The App the user chose for it, kept for this one; `null`: none. */
  app: string | null;
}

/** D-96 F: a "Start another" offer while it stands: where it opens, and
 * the session just started (the page the user is reading). */
export interface AnotherOffer {
  dir: string;
  after: StartedBefore;
  sessionId: string | null;
  /** Its toast, gone with it. */
  toastId: string;
}

let standing: AnotherOffer | null = null;

/** The offer stands (its toast is up): the next New session opens there. */
export function standOffer(offer: AnotherOffer): void {
  standing = offer;
}

/** The offer, while it stands. */
export function standingOffer(): AnotherOffer | null {
  return standing;
}

/** The offer is over: taken, closed, or the user moved on. Its toast goes
 * with it, unless `toastGone`. */
export function withdrawOffer(toastGone = false): void {
  const offer = standing;
  standing = null;
  if (offer && !toastGone) dismissToast(offer.toastId);
}

/** The user is on `sessionId`'s page (`null`: elsewhere): an offer made
 * for another page is over, they started working elsewhere. */
export function offerFollows(sessionId: string | null): void {
  if (standing && standing.sessionId !== sessionId) withdrawOffer();
}

interface SheetState {
  open: boolean;
  dir: string | null;
  account: string | null;
  after: StartedBefore | null;
}

export interface NewSessionSheet extends SheetState {
  show: (at?: Opening) => void;
  close: () => void;
}

const CLOSED: SheetState = { open: false, dir: null, account: null, after: null };

/** `here`: the project of the page it was opened from (⌘N, the header's
 * New session; AJ-E2311 item 2), weaker than a door's `dir` and than a
 * standing "Start another". */
type Opening = { dir?: string | null; here?: string | null; account?: string | null; after?: StartedBefore | null };

/** The sheet opened `at`. Opened plainly (⌘N, the header's New session)
 * while "Start another" stands, it opens where the offer would (the
 * 0.4.1 persona walk); else in the project of the page it was opened
 * from. Any opening ends the offer. */
function opening(at: Opening): SheetState {
  const offer = !at.dir && !at.after && !at.account ? standingOffer() : null;
  withdrawOffer();
  return offer ? { open: true, dir: offer.dir, account: null, after: offer.after } : { open: true, dir: at.dir ?? at.here ?? null, account: at.account ?? null, after: at.after ?? null };
}

export function useNewSessionSheet(): NewSessionSheet {
  const [state, setState] = useState<SheetState>(CLOSED);
  const show = (at: Opening = {}) => setState(opening(at));
  useLayoutEffect(() => {
    const on = (event: Event) => {
      const detail = (event as CustomEvent<{ accountId?: unknown; dir?: unknown } | null>).detail;
      // AJ-E3011 round 2: a pool's "Start one now" names the folder too.
      setState(opening({ account: typeof detail?.accountId === "string" ? detail.accountId : null, dir: typeof detail?.dir === "string" ? detail.dir : null }));
    };
    window.addEventListener("aij:new-session", on);
    return () => window.removeEventListener("aij:new-session", on);
  }, []);
  return { ...state, show, close: () => setState(CLOSED) };
}
