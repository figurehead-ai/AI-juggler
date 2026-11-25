/**
 * D-84 — apps and profiles AI Juggler found but the user hasn't added:
 * offered, never added by themselves. Add makes one an App; Not now
 * stops offering it. One list for the places it shows: Settings ›
 * Desktop apps ("Found on this computer"), after Find them the App
 * page's card (walk A6), and (D-100) the rail's "Running, not added",
 * the running ones (`RailOfferRow`).
 */
import { useEffect, useState, type JSX } from "react";
import { AppIcon } from "./AppIcon";
import { AppKindBadge } from "./AppKindBadge";
import { appKindOfKey, appKindWords } from "../lib/app-kind";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";
import { onPlacesChanged, placesOfferAnswer, placesOffers } from "../lib/ipc/places";
import { addedNote } from "../lib/app-folder";
import { chooseAppFolder } from "../lib/choose-app-folder";
import { showToast } from "./Toast";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";
import type { PlaceAddedInfo } from "../types/generated/PlaceAddedInfo";

/** AJ-E2327: Add's confirmation, wherever Add is (the rail, Settings,
 * first run, an App page's Find them): the App and the tool folder that
 * came with it, where it is; or why none did, with Choose its folder…. */
function sayAdded(offer: PlaceOfferInfo, added: PlaceAddedInfo): void {
  const note = addedNote(offer.label, added.folder);
  showToast({
    id: `place-added-${offer.id}`,
    tone: note.unlinked ? "neutral" : "success",
    message: note.message,
    detail: note.detail ?? undefined,
    action: note.unlinked ? { label: "Choose its folder…", run: () => void chooseAppFolder(offer.id, offer.label) } : undefined,
    actionBelow: note.unlinked,
    waits: note.unlinked,
  });
}

/** The offers (`null` until read), again whenever a place changes;
 * `only` keeps the ones of one app (its icon's id). `answer` says what
 * Add did (`sayAdded`) and resolves with it (`null` when it failed). */
export function usePlaceOffers(only: string | null = null): {
  offers: PlaceOfferInfo[] | null;
  answer: (offer: PlaceOfferInfo, add: boolean) => Promise<PlaceAddedInfo | null>;
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
    return placesOfferAnswer(offer.id, add).then(
      (added) => {
        setOffers((now) => (now ?? []).filter((one) => one.id !== offer.id));
        if (add) sayAdded(offer, added);
        return added;
      },
      (cause) => {
        setNote(errorText(cause));
        return null;
      },
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

/** 0.5.1 F99 (the F95 class): an offer's button named with its App ("Add
 * Claude app · personal", "Not now, Claude app · personal"), its visible
 * words first, so a screen reader tells the rows' buttons apart. */
function offerActionName(action: "Add" | "Not now", label: string): string {
  return action === "Add" ? `Add ${label}` : `Not now, ${label}`;
}

/** One offer's row: its name, Add and Not now. */
export function OfferRow({ offer, onAnswer }: { offer: PlaceOfferInfo; onAnswer: (add: boolean) => void }): JSX.Element {
  return (
    <li className="flex flex-wrap items-center gap-3 px-4 py-2.5" data-testid="desktop-offer">
      <span className="min-w-0 flex-1 truncate text-sm text-ink">{offer.label}</span>
      <Button size="sm" variant="secondary" onClick={() => onAnswer(true)} aria-label={offerActionName("Add", offer.label)} data-testid="desktop-offer-add">
        Add
      </Button>
      <Button size="sm" variant="ghost" onClick={() => onAnswer(false)} aria-label={offerActionName("Not now", offer.label)} data-testid="desktop-offer-not-now">
        Not now
      </Button>
    </li>
  );
}

/** ADR-0032 Amendment 1 (D-84, D-100): one line of the rail's "Running,
 * not added": an app an adapter recognises running that the user hasn't
 * added. Offered, never added by itself: its icon and kind (Desktop or
 * CLI) as the Apps around it show theirs, its name, and Add (Not now is
 * where every offer is, Settings › Desktop apps). */
export function RailOfferRow({ offer, onAdd }: { offer: PlaceOfferInfo; onAdd: () => void }): JSX.Element {
  const kind = appKindOfKey(offer.id);
  const words = [offer.label, kind ? appKindWords(kind).name : null, "running, not added"].filter(Boolean).join(", ");
  return (
    <div className="flex min-h-8 min-w-0 items-center gap-1 rounded-md pr-1" data-testid="rail-offer" data-offer={offer.id} data-kind={kind ?? undefined}>
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
        onClick={onAdd}
        aria-label={`Add ${offer.label}`}
        title={`Add ${offer.label} to your apps`}
        data-rail-nav
        data-testid="rail-offer-add"
        // AJ-E1554 (PAT-SURF): a fill and its hover by tone, never a line.
        className="flex h-6 shrink-0 items-center rounded-md bg-control px-2 text-label font-medium text-ink outline-none transition-colors hover:bg-control-hi focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        Add
      </button>
    </div>
  );
}
