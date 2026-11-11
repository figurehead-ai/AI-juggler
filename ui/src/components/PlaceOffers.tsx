/**
 * D-84 — apps and profiles AI Juggler found but the user hasn't added:
 * offered, never added by themselves. Add makes one an App; Not now
 * stops offering it. One list for the two places it shows: Settings ›
 * Desktop apps ("Found on this computer") and, after Find them, the
 * App page's card (walk A6).
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";
import { onPlacesChanged, placesOfferAnswer, placesOffers } from "../lib/ipc/places";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";

/** The offers (`null` until read), again whenever a place changes;
 * `only` keeps the ones of one app (its icon's id). */
export function usePlaceOffers(only: string | null = null): {
  offers: PlaceOfferInfo[] | null;
  answer: (offer: PlaceOfferInfo, add: boolean) => void;
  note: string | null;
  /** Reads them now (after Find them: its answer, not an older one). */
  reload: () => Promise<void>;
} {
  const [offers, setOffers] = useState<PlaceOfferInfo[] | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    const load = () => void placesOffers().then(setOffers, () => setOffers([]));
    load();
    return onPlacesChanged(load);
  }, []);
  const answer = (offer: PlaceOfferInfo, add: boolean) => {
    setNote(null);
    placesOfferAnswer(offer.id, add).then(
      () => setOffers((now) => (now ?? []).filter((one) => one.id !== offer.id)),
      (cause) => setNote(errorText(cause)),
    );
  };
  const reload = () => placesOffers().then(setOffers, () => setOffers([]));
  return { reload, offers: offers && only !== null ? offers.filter((offer) => offerOf(offer.id) === only) : offers, answer, note };
}

/** The app an offer is of, from its id (`app:<app>:<folder>`). */
function offerOf(id: string): string | null {
  const [kind, app] = id.split(":");
  return kind === "app" && app ? app : null;
}

/** One offer's row: its name, Add and Not now. */
export function OfferRow({ offer, onAnswer }: { offer: PlaceOfferInfo; onAnswer: (add: boolean) => void }): JSX.Element {
  return (
    <li className="flex flex-wrap items-center gap-3 px-4 py-2.5" data-testid="desktop-offer">
      <span className="min-w-0 flex-1 truncate text-sm text-ink">{offer.label}</span>
      <Button size="sm" variant="secondary" onClick={() => onAnswer(true)} data-testid="desktop-offer-add">
        Add
      </Button>
      <Button size="sm" variant="ghost" onClick={() => onAnswer(false)} data-testid="desktop-offer-not-now">
        Not now
      </Button>
    </li>
  );
}
