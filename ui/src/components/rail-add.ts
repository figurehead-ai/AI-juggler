/**
 * AJ-E2327 — Add on the rail's "Running, not added" is the user's own
 * change: the rail keeps its scroll position (the hold lets it through,
 * `rail-hold`'s `ownChange`), and the added App's line (its header in
 * Open now, or its line in Inactive apps) is brought into view once it's
 * drawn. 0.5.0's hold scrolled the rail's top out of view on every Add.
 */
import { useEffect, useRef, type RefObject } from "react";
import { LINE_KEY } from "./rail-hold";
import { knownKey } from "../lib/rail-apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { PlaceAddedInfo } from "../types/generated/PlaceAddedInfo";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";

/** How long an Add waits for its App's line before giving up. */
const WAIT_MS = 4000;

/** The rail's Add: `answer` adds; `ownChange` lets the change through. */
export function useAddOffer(
  root: RefObject<HTMLElement | null>,
  ownChange: () => void,
  answer: (offer: PlaceOfferInfo, add: boolean) => Promise<PlaceAddedInfo | null>,
  places: readonly KnownPlaceInfo[] | null,
): (offer: PlaceOfferInfo) => void {
  // The App added, and until when its line is waited for.
  const bring = useRef<{ id: string; until: number } | null>(null);
  const own = useRef(ownChange);
  const known = useRef(places);
  useEffect(() => {
    own.current = ownChange;
    known.current = places;
  });
  /** Brings the added App's line into view, once it's drawn. */
  const tryBring = () => {
    const want = bring.current;
    if (!want) return;
    if (Date.now() > want.until) {
      bring.current = null;
      return;
    }
    const place = known.current?.find((one) => one.id === want.id);
    const key = place ? CSS.escape(knownKey(place)) : null;
    const line = key ? root.current?.querySelector<HTMLElement>(`[${LINE_KEY}="header:${key}"], [${LINE_KEY}="idle:${key}"]`) : null;
    if (!line) return;
    bring.current = null;
    own.current();
    line.scrollIntoView({ block: "nearest" });
  };
  const tryRef = useRef(tryBring);
  useEffect(() => {
    tryRef.current = tryBring;
  });
  // Each read of the places may be the one that draws it.
  useEffect(() => {
    if (places !== null) tryRef.current();
  }, [places]);
  return (offer) => {
    own.current();
    void answer(offer, true).then((added) => {
      if (!added) return;
      bring.current = { id: added.id, until: Date.now() + WAIT_MS };
      window.requestAnimationFrame(() => tryRef.current());
    });
  };
}
