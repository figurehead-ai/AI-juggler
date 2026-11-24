/**
 * AJ-E2326 R4 — under a row whose conversation is live both in AI
 * Juggler's session and in another app: "Open in two places", where else,
 * and one click to stop ours (the session's own Stop, `host.stopSession`;
 * the other app's process is never touched, D-28). Two writers on one
 * conversation is a real need, so its words are amber (PAT-COLOR); the
 * button is a ghost, a tone on hover (PAT-SURF).
 */
import type { JSX } from "react";
import { useActionHost } from "../ItemMenu";
import { TWO_PLACES, type TwoPlaces } from "../../lib/two-places";

/** The note's words: "Open in two places · also in Claude app · b-work". */
export function twoPlacesWords(two: TwoPlaces): string {
  return `${TWO_PLACES} · also in ${two.alsoIn}`;
}

export function TwoPlacesNote({ two, title }: { two: TwoPlaces; title: string }): JSX.Element {
  const host = useActionHost();
  const stop = host.stopSession;
  return (
    <div className="pointer-events-none relative ml-[1.625rem] flex min-w-0 items-center gap-1.5 pb-1 pr-1 text-label" data-testid="two-places">
      <span className="shrink-0 font-medium text-warning">{TWO_PLACES}</span>
      <span className="min-w-0 truncate text-muted" title={twoPlacesWords(two)}>· also in {two.alsoIn}</span>
      {stop && (
        <button
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            stop(two.ours);
          }}
          aria-label={`Stop AI Juggler's session on ${title}`}
          title={`Stops AI Juggler's session on it; ${two.alsoIn === "AI Juggler" ? "this app" : two.alsoIn} keeps its own`}
          className="btn-ghost pointer-events-auto ml-auto shrink-0 rounded-md px-2 py-0.5 text-label font-medium outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid="two-places-stop"
        >
          Stop ours
        </button>
      )}
    </div>
  );
}
