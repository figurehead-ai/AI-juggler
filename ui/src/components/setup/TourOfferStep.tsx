/**
 * AJ-E1545 step 5: the tour (AJ-E1546), offered in Bot's voice: what it
 * shows and how long it takes. Take the tour is the foot's primary;
 * Finish without it closes the setup (Help › Take the tour stays).
 */
import type { JSX } from "react";
import { IconBot } from "../bot/bot-icons";
import { IconHelp } from "../icons-menu";

export function TourOfferStep({ stops }: { stops: number }): JSX.Element {
  return (
    <div className="flex flex-col gap-5" data-testid="setup-tour">
      <div className="flex items-start gap-3 rounded-[14px] bg-surface p-5">
        <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-[10px] bg-accent-soft text-accent-hi">
          <IconBot width={22} height={22} />
        </span>
        <span className="flex flex-col gap-1">
          <span className="text-body font-semibold text-ink">Bot</span>
          <span className="text-body text-ink-2">
            You're set up. I can show you around the real thing: {stops} stops, about a minute. I point at each one and say what it's for; Esc ends it any time.
          </span>
        </span>
      </div>
      <p className="flex items-center gap-2 text-meta text-muted">
        <IconHelp width={16} height={16} aria-hidden="true" />
        Later, Help › Take the tour or your menu starts it again.
      </p>
    </div>
  );
}
