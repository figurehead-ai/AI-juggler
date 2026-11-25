/**
 * 0.5.1 F74: the page of a session a switch just made, until the session
 * list holds it. It says what happened and where the conversation is now
 * ("Starting on Personal…", switched from the original, which stays
 * resumable), with the page's first card as a skeleton (EMP-2: never a
 * flash of empty). Past `SLOW` it stops pretending to load and says where
 * the conversation is, in one line. The real page replaces it as soon as
 * the list has the session.
 */
import type { JSX } from "react";
import type { Arrival } from "../lib/arriving";
import { PageHeader } from "../pages/PageHeader";
import { IconStateHandover } from "./icons";
import { Card } from "./primitives";
import { LoadingState } from "./LoadingState";

/** How long it looks like loading before it says so plainly. */
const SLOW = 20_000;
/** When it says "Still loading…" (lib/loading's 3 s). */
const STILL = 3_000;

function Bar({ className }: { className: string }): JSX.Element {
  return <span className={`block rounded bg-skeleton ${className}`} />;
}

export function ArrivingSession({ arrival, now }: { arrival: Arrival; now: number }): JSX.Element {
  const slow = now - arrival.at > SLOW;
  const facts = (
    <span data-testid="arriving-from">
      {arrival.from ? <>Switched from “{arrival.from}”. </> : null}
      The original stays resumable.
    </span>
  );
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4" data-testid="arriving-session" data-slow={slow ? "true" : undefined}>
      <PageHeader kind="session" layout="session" mark={<IconStateHandover width={18} height={18} />} name={slow ? `On ${arrival.target}` : `Starting on ${arrival.target}…`} facts={facts} />
      {/* Loading until the list holds it; past its wait, "empty" is the
          plain line of where the conversation is (AJ-E1552). */}
      <LoadingState
        state={slow ? "empty" : "loading"}
        slow={now - arrival.at > STILL}
        what={`the session on ${arrival.target}`}
        onRetry={() => undefined}
        empty={
          <p className="text-body text-ink-2" data-testid="arriving-slow">
            The conversation continues on {arrival.target}. AI Juggler doesn’t list it yet; its page opens here once it does.
          </p>
        }
        skeleton={
          <output aria-label={`Loading the session on ${arrival.target}`} aria-busy="true" data-testid="arriving-skeleton" className="flex flex-col motion-safe:animate-pulse">
            <Card aria-hidden="true" className="flex flex-col gap-3 p-4">
              <span className="flex items-center gap-2">
                <Bar className="h-3 w-1/4" />
                <Bar className="h-3 w-16 rounded-full" />
              </span>
              <Bar className="h-2.5 w-3/4" />
              <Bar className="h-2.5 w-2/3" />
              <Bar className="h-2.5 w-1/2" />
              <Bar className="mt-2 h-24 w-full rounded-lg" />
            </Card>
          </output>
        }
      >
        {null}
      </LoadingState>
    </div>
  );
}
