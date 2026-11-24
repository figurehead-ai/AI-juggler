/**
 * AJ-E2319, 0.5.1 F72: what holds a conversation's files, each with its
 * door: an app or profile it runs in ("Go to Claude app · Work", started
 * first when it isn't running) or a session of these accounts open
 * outside the apps ("Go to “Fix the login test” in Terminal", its own
 * window). What the door did is said beside it. A program AI Juggler
 * knows only by name has no door.
 */
import { useState, type JSX } from "react";
import type { MoveHolder } from "../../types/generated/MoveHolder";
import { placesOpen } from "../../lib/ipc/places";
import { conversationsGoTo } from "../../lib/ipc/conversations";
import { errorText } from "../../lib/bridge";
import { Button } from "../primitives";

/** Whether a holder has a door. */
function hasDoor(holder: MoveHolder): boolean {
  return holder.place_id !== null || (holder.account_id !== null && holder.conversation !== null);
}

/** Its door's way there. */
function goTo(holder: MoveHolder) {
  if (holder.place_id) return placesOpen(holder.place_id);
  return conversationsGoTo(holder.account_id!, holder.conversation!);
}

/** The doors of `holders` that have one; nothing when none does. */
export function HolderDoors({ holders, testId }: { holders: MoveHolder[]; testId: string }): JSX.Element | null {
  const [said, setSaid] = useState<string | null>(null);
  const doors = holders.filter(hasDoor);
  if (doors.length === 0) return null;
  const open = (holder: MoveHolder) =>
    void goTo(holder).then(
      (outcome) => setSaid(outcome.outcome),
      (cause) => setSaid(errorText(cause)),
    );
  return (
    <div className="flex flex-wrap items-center gap-2" data-testid={testId}>
      {doors.map((holder) => (
        <Button key={`${holder.place_id ?? ""}|${holder.conversation ?? ""}|${holder.name}`} size="sm" variant="secondary" type="button" onClick={() => open(holder)} data-testid="switch-move-holder-door">
          Go to {holder.name}
        </Button>
      ))}
      {said && <output className="text-meta text-ink-2">{said}</output>}
    </div>
  );
}
