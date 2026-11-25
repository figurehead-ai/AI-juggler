/**
 * AJ-E1546 — where the tour lives in the window: it starts when something
 * asks (the setup wizard's last step, Help › Take the tour in the app
 * menu and your menu), loaded on first use; and after an update, while
 * the window has accounts and nothing else is open over it, the calm
 * once-only "New in 0.5.x · Show me" for the steps not seen yet.
 */
import { useEffect, useState, type JSX } from "react";
import { lazyNamed } from "../../lib/lazy";
import { currentOffer, markOffered, startTour, stepsFor, TAKE_TOUR_EVENT, type TourStep } from "../../lib/tour";
import { onTakeTourAsked } from "../../lib/ipc/tour";
import { IconBot } from "../bot/bot-icons";
import { IconClose } from "../icons-controls";

const TourLazy = lazyNamed(() => import("./Tour"), "Tour");

function NewOffer({ label, ids, onDone }: { label: string; ids: readonly string[]; onDone: () => void }): JSX.Element {
  return (
    <output className="fixed bottom-4 left-4 z-40 flex items-center gap-2 rounded-full bg-surface py-1.5 pr-1.5 pl-3 shadow-card inset-ring-1 inset-ring-hairline" data-testid="tour-new">
      <IconBot width={14} height={14} aria-hidden="true" className="text-accent-hi" />
      <span className="text-meta text-ink">{label}</span>
      <button
        type="button"
        className="rounded-full px-2 py-0.5 text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60"
        onClick={() => {
          markOffered(ids);
          onDone();
          startTour(ids);
        }}
        data-testid="tour-new-show"
      >
        Show me
      </button>
      <button
        type="button"
        aria-label={`Not now: ${label}`}
        className="flex size-6 items-center justify-center rounded-full text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60"
        onClick={() => {
          markOffered(ids);
          onDone();
        }}
        data-testid="tour-new-close"
      >
        <IconClose width={12} height={12} aria-hidden="true" />
      </button>
    </output>
  );
}

export function TourHost({ offerAllowed }: { offerAllowed: boolean }): JSX.Element | null {
  const [steps, setSteps] = useState<TourStep[] | null>(null);
  const [offer, setOffer] = useState(currentOffer);
  useEffect(() => {
    const start = (event: Event) => {
      const detail: unknown = event instanceof CustomEvent ? event.detail : null;
      const ids: unknown = detail !== null && typeof detail === "object" && "ids" in detail ? detail.ids : undefined;
      setOffer(null);
      setSteps(stepsFor(Array.isArray(ids) ? ids.filter((id): id is string => typeof id === "string") : undefined));
    };
    window.addEventListener(TAKE_TOUR_EVENT, start);
    const stopNative = onTakeTourAsked(() => startTour());
    return () => {
      window.removeEventListener(TAKE_TOUR_EVENT, start);
      stopNative();
    };
  }, []);
  if (steps) return <TourLazy steps={steps} onEnd={() => setSteps(null)} />;
  if (offer && offerAllowed) return <NewOffer label={offer.label} ids={offer.ids} onDone={() => setOffer(null)} />;
  return null;
}
