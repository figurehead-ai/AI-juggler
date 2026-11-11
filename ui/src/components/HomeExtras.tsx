/**
 * D-96 B (AJ-E3307 M2) — the heading's "and N more things", named right
 * under it ("Work is almost out of room · ChatGPT needs its sign-in"),
 * each one click to its row in Needs you (`revealNeed`). The counts line
 * under it keeps its own quieter style; these wear the rail's needs-you
 * dot, since they wait on you too.
 */
import type { JSX } from "react";
import { headlineExtras, type HomeNeed } from "../lib/home-needs";
import { revealNeed } from "./HomeNeeds";

/** `shown`: the heading names sessions and more things (with no session
 * in it, those things are the card's own rows, the first one large:
 * never said twice). */
export function HomeExtras({ shown, needs, more }: { shown: boolean; needs: HomeNeed[]; more: { caps: string[]; messages: number; tasks: number } }): JSX.Element | null {
  const extras = shown ? headlineExtras(needs, more) : [];
  if (extras.length === 0) return null;
  return (
    <p className="flex flex-wrap items-center gap-x-1.5 text-body text-ink" data-testid="home-extras">
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
