/**
 * D-84 — apps and profiles AI Juggler found but the user hasn't added:
 * offered, never added by themselves. Add makes one an App; Not now
 * stops offering it. One list for the places it shows: Settings ›
 * Desktop apps ("Found on this computer"), after Find them the App
 * page's card (walk A6), and (D-100) the rail's "Running, not added",
 * the running ones (`RailOfferRow`).
 */
import { useEffect, useState, type JSX, type MouseEvent } from "react";
import { AppIcon } from "./AppIcon";
import { AppKindBadge } from "./AppKindBadge";
import { MenuItem, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { appKindOfKey, appKindWords } from "../lib/app-kind";
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
export function offerOf(id: string): string | null {
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

/** ADR-0032 Amendment 1 (D-84, D-100): one line of the rail's "Running,
 * not added": an app an adapter recognises running that the user hasn't
 * added. Offered, never added by itself: its icon and kind (Desktop or
 * CLI) as the Apps around it show theirs, its name, and Add; a
 * right-click (or Shift+F10) also has Not now. */
export function RailOfferRow({ offer, onAnswer }: { offer: PlaceOfferInfo; onAnswer: (add: boolean) => void }): JSX.Element {
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const kind = appKindOfKey(offer.id);
  const words = kind ? `${offer.label}, ${appKindWords(kind).name}, running, not added` : `${offer.label}, running, not added`;
  const run = (add: boolean) => () => (setMenu(null), onAnswer(add));
  return (
    <div
      onContextMenu={(event: MouseEvent<HTMLDivElement>) => {
        event.preventDefault();
        setMenu(menuPoint(event));
      }}
      onKeyDown={(event) => {
        if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
          event.preventDefault();
          const box = event.currentTarget.getBoundingClientRect();
          setMenu({ x: box.left + 8, y: box.bottom + 4 });
        }
      }}
      className="flex min-h-8 min-w-0 items-center gap-1 rounded-md pr-1"
      data-testid="rail-offer"
      data-offer={offer.id}
      data-kind={kind ?? undefined}
    >
      {/* Where a fold sits on the rows around it, so the icons line up. */}
      <span aria-hidden="true" className="w-6 shrink-0" />
      <span className="flex min-w-0 flex-1 items-center gap-1 px-1" title={offer.label}>
        <span className="sr-only">{words}</span>
        <span aria-hidden="true" className="flex min-w-0 items-center gap-1">
          <AppIcon appId={offerOf(offer.id)} name={offer.label} size={16} />
          <AppKindBadge kind={kind} />
          <span className="ml-0.5 truncate text-label font-medium text-ink-2">{offer.label}</span>
        </span>
      </span>
      <button
        type="button"
        onClick={() => onAnswer(true)}
        aria-label={`Add ${offer.label}`}
        title={`Add ${offer.label} to your apps`}
        data-rail-nav
        data-testid="rail-offer-add"
        className="flex h-6 shrink-0 items-center rounded-md border border-hairline px-2 text-label font-medium text-ink outline-none hover:border-accent/40 focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        Add
      </button>
      {menu && (
        <RowMenu heading={offer.label} ariaLabel={`Actions for ${offer.label}`} at={menu} onClose={() => setMenu(null)} testId="rail-offer-menu">
          <MenuItem onClick={run(true)} testId="rail-offer-menu-add">
            Add to your apps
          </MenuItem>
          <MenuItem onClick={run(false)} testId="rail-offer-menu-not-now">
            Not now
          </MenuItem>
        </RowMenu>
      )}
    </div>
  );
}
