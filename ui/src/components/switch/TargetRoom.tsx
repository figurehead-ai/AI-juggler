/**
 * The target's room in the switch dialog (the 0.2 walk, F1: an account
 * with 0% left read "Ready to switch … Nothing needs you"). Read from the
 * one runway (`runwayOf` on its account, as Home's Runway), never a second
 * reading: every step says how much room it has; out of room or low says
 * until when and what a switch there does, explained, and the last button
 * reads "Switch anyway".
 */
import type { JSX } from "react";
import { lowRoomWords, outOfRoomConsequence, outOfRoomWords, roomKind } from "../../lib/room";
import { creditsConsequence, usingCreditsWords } from "../../lib/credits";
import { NOT_SIGNED_IN, roomOfProfile, type TargetRoom } from "../../lib/room-targets";
import { canStart } from "../../lib/new-session-choice";
import { GUIDE } from "../../lib/switch-words";
import { useSubscriptions } from "../../lib/subscriptions";
import { useProfiles, useRunways } from "../../pages/session/data";
import { Explain } from "./Explain";

/** The target account's room, once the preflight names it. */
export function useTargetRoom(accountId: string | undefined): TargetRoom | null {
  const accounts = useSubscriptions();
  const displays = useRunways();
  const folders = useProfiles();
  if (!accountId) return null;
  // 0.4.1 (the rc12 walk: "Room on f…@…: 78% left" for an App New session
  // calls not signed in): New session's rule, and no account's details.
  if (!canStart(folders, accountId)) return { left: null, kind: "blocked", words: NOT_SIGNED_IN, resets: null, resetsAtMs: null, credits: null, account: null };
  const room = roomOfProfile(accountId, accounts, displays);
  return { ...room, kind: roomKind(room.left, true, room.credits !== null) };
}

/** A switch there walks into a limit: the last button says so. */
export function risky(room: TargetRoom | null): boolean {
  return room?.kind === "out" || room?.kind === "low";
}

/** The room line every step shows; a warning, explained, when it's out
 * or low. */
export function RoomNote({ room, target }: { room: TargetRoom | null; target: string }): JSX.Element | null {
  if (room === null) return null;
  // D-62: the room is the Account's, so it's said by its name; a folder
  // no Account lists, by the target's own.
  if (room.kind === "blocked") {
    return (
      <p className="text-body text-ink" data-testid="switch-room" data-room="blocked">
        {target}: {NOT_SIGNED_IN}
      </p>
    );
  }
  const who = room.account ?? target;
  // 0.3.2: its window is used up, but its credits carry a session on.
  if (room.kind === "credits" && room.credits) {
    return (
      <p className="text-body text-ink" data-testid="switch-room" data-room="credits">
        Room on {who}: {usingCreditsWords(room.credits)}. {creditsConsequence(room.resetsAtMs)}
      </p>
    );
  }
  if (!risky(room)) {
    return (
      <p className="text-body text-ink" data-testid="switch-room" data-room={room.kind}>
        Room on {who}: {room.left === null ? room.words : `${room.words}${room.resets ? ` · ${room.resets}` : ""}`}
      </p>
    );
  }
  const out = room.kind === "out";
  return (
    <div className="flex flex-col gap-1.5 rounded-lg border border-warning/40 px-3 py-2" data-testid="switch-room" data-room={room.kind}>
      <p className="text-body font-semibold text-warning">
        {out ? `${who}: ${outOfRoomWords(room.resetsAtMs)}` : lowRoomWords(who, room.left ?? 0)}
      </p>
      <p className="text-body text-ink">
        {out ? outOfRoomConsequence(who, room.resetsAtMs) : `${room.resets ? `${room.resets}. ` : ""}You can still switch; it stops there if it reaches the limit.`}
      </p>
      <Explain
        guide={GUIDE.room}
        testId="switch-room-explain"
        tips={[
          "Each account has its own limits: a 5-hour window and a week. When one runs out, that account answers nothing more until it resets.",
          `Example: switching to ${target} while it's out of room starts the conversation there, but the first message stops at the limit and waits for the reset.`,
          "To keep working now, Cancel and pick the app marked Suggested: it has the most room.",
        ]}
      />
    </div>
  );
}
