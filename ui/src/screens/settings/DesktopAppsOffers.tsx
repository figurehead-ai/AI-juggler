/**
 * Settings › Desktop apps: what's found and not added (D-84), and a row
 * just forgotten, held in its place until the pointer leaves.
 */
import type { JSX } from "react";
import { Button } from "../../components/primitives";
import { OfferRow, usePlaceOffers } from "../../components/PlaceOffers";
import { PaneSection, RowNote } from "./parts";

/** A row just forgotten, held where it was (the go sitting's rerun: Forget
 * took the row out and Find profiles put it back under Found a moment
 * later, so the rows moved under the pointer and the next click opened
 * another App's page). */
export interface Forgotten {
  id: string;
  name: string;
  at: number;
}

export type Offers = ReturnType<typeof usePlaceOffers>;

/** D-84: apps seen running, or profiles found beside one that ran,
 * that the user hasn't added (components/PlaceOffers). One just forgotten
 * is offered in its own row instead (`held`). */
export function FoundOnThisMac({ offers: { offers, answer, note }, held }: { offers: Offers; held: Forgotten[] }): JSX.Element | null {
  const shown = (offers ?? []).filter((offer) => !held.some((one) => one.id === offer.id));
  if (shown.length === 0) return null;
  return (
    <PaneSection id="found" title="Found on this computer" footer="Seen running, or found beside a profile that ran, and not added. AI Juggler adds an app only when you say so; until then it isn't in your apps, the rail or Home.">
      {shown.map((offer) => (
        <OfferRow key={offer.id} offer={offer} onAnswer={(add) => answer(offer, add)} />
      ))}
      {note && <RowNote>{note}</RowNote>}
    </PaneSection>
  );
}

/** A forgotten app in its old place: Add again once it's offered. */
export function ForgottenRow({ one, offers }: { one: Forgotten; offers: Offers }): JSX.Element {
  const offer = offers.offers?.find((found) => found.id === one.id);
  return (
    <li className="flex flex-wrap items-center gap-3 px-4 py-2.5" data-testid="desktop-place-forgotten">
      <span className="min-w-0 flex-1 truncate text-sm text-muted">
        <span className="font-medium text-ink-2">{one.name}</span> · forgotten{offer ? "; it's running, so it's offered again" : ""}
      </span>
      {offer && (
        <Button size="sm" variant="secondary" onClick={() => offers.answer(offer, true)} data-testid="desktop-place-add-again">
          Add again
        </Button>
      )}
    </li>
  );
}

