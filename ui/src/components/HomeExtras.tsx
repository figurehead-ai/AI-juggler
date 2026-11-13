/**
 * D-96 B (AJ-E3307 M2) — the heading's "and N more things", named right
 * under it ("Work is almost out of room · ChatGPT needs its sign-in"),
 * each one click to its row in Needs you (`revealNeed`). The counts line
 * under it keeps its own quieter style; these wear the rail's needs-you
 * dot, since they wait on you too.
 */
import type { JSX } from "react";
import { headlineExtras, type HomeNeed } from "../lib/home-needs";
import { EXTRAS_PLACE, keptShape, useKeepShape } from "../lib/needs-place";
import { revealNeed } from "./HomeNeeds";

/** `shown`: the heading names sessions and more things (with no session
 * in it, those things are the card's own rows, the first one large:
 * never said twice). `pending`: the needs haven't loaded; D-103.6: the
 * line keeps its place if it showed last time, so nothing under it moves. */
export function HomeExtras({ shown, needs, more, pending = false }: { shown: boolean; needs: HomeNeed[]; more: { caps: string[]; messages: number; tasks: number }; pending?: boolean }): JSX.Element | null {
  const extras = shown ? headlineExtras(needs, more) : [];
  const box = useKeepShape<HTMLParagraphElement>(EXTRAS_PLACE, !pending, extras.length);
  if (pending) {
    const kept = keptShape(EXTRAS_PLACE);
    if (!kept || kept.rows === 0 || kept.height <= 0) return null;
    return (
      <p aria-hidden="true" className="flex items-center motion-safe:animate-pulse" style={{ height: kept.height }} data-testid="home-extras-place">
        <span className="block h-3 w-1/3 rounded bg-skeleton" />
      </p>
    );
  }
  if (extras.length === 0) return null;
  return (
    <p ref={box} className="flex flex-wrap items-center gap-x-1.5 text-body text-ink" data-testid="home-extras">
      {extras.map((extra, index) => (
        <span key={extra.key} className="flex items-center gap-1.5">
          {index > 0 && <span aria-hidden="true" className="text-muted">·</span>}
          <button type="button" onClick={() => revealNeed(extra.target)} className="flex items-center gap-1.5 rounded outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
            <span aria-hidden="true" className="size-1.5 shrink-0 rounded-full bg-warning" />
            {extra.words}
          </button>
        </span>
      ))}
    </p>
  );
}
