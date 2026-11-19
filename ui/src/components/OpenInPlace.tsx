/**
 * 0.3.3: an ended session reopens in the app it was last open in (started
 * when it isn't running; Codex at the thread itself). 0.4.1: the session
 * itself, in that profile; when macOS would ask first, only "Open the
 * session itself" lets it.
 *
 * One button for every page that offers it: a conversation's own page,
 * and (0.4.6, the rc3c walk) the page of AI Juggler's run of a
 * conversation an app started, once that run has ended. The guide's
 * "continued in AI Juggler, stopped, then Open in Claude app" had no
 * button there: the run stands for its conversation, so the
 * conversation's page wasn't reached.
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { showToast } from "./Toast";
import { showGoTo } from "../lib/go-to";
import { conversationsGoTo } from "../lib/ipc/conversations";
import { placesOpen } from "../lib/ipc/places";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";

export function OpenInPlace({
  place,
  accountId,
  conversationId,
  size = "md",
  testId,
}: {
  /** Where it was last open (`useLastPlace`); nothing without one. */
  place: KnownPlaceInfo | null;
  accountId: string | null;
  conversationId: string | null;
  size?: "sm" | "md";
  testId: string;
}): JSX.Element | null {
  if (!place || !accountId || !conversationId) return null;
  const open = () => {
    showToast({ id: "go-to", message: `Opening ${place.label}…` });
    placesOpen(place.id, conversationId).then(
      (outcome) => showGoTo(outcome, () => conversationsGoTo(accountId, conversationId, null, true)),
      () => showToast({ id: "go-to", message: `Couldn't open ${place.label}` }),
    );
  };
  return (
    // The app's name only: the profile is the chip under the title, and a
    // long label cut the title short.
    <Button size={size} variant="secondary" onClick={open} disabled={place.cannot_start !== null} title={place.cannot_start ?? `Open in ${place.label}`} aria-label={`Open in ${place.label}`} data-testid={testId}>
      Open in {place.app}
    </Button>
  );
}
