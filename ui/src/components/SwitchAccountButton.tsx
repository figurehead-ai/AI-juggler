/**
 * D-143 (DESIGN-SYSTEM PAT-SW): switching is the product's main action,
 * always one visible click away. The maintainer, 2026-11-21: "this is our
 * main feature, we don't want it to be hard to find"; on a page whose
 * account was out of room, the only way to switch was a folded card's
 * chevron, ⌘⇧H or a line in ⋯.
 *
 * The first thing in a session's or conversation's side column, above
 * Where it runs: "Switch account…" with the switch icon. Accent (tinted,
 * an action at a glance) while its account has room; the page's one
 * filled primary when its account is out of room or low, when the page's
 * other primary steps down (`ownRoomWantsSwitch`). Its press opens the
 * ranked list of Apps to continue in (the same dialog as ⌘⇧H).
 */
import type { JSX } from "react";
import { focusHandover } from "../lib/handover-focus";
import { useOwnRoom, type OwnRoom } from "../lib/own-room-notices";
import { IconStateHandover } from "./icons";
import { WhyButton } from "./WhyTip";

/** Whether the switch takes the page's one primary (BTN-1): its account is
 * out of room or low. The page's own primary (Continue in AI Juggler,
 * Resume) is secondary then. */
export function ownRoomWantsSwitch(room: OwnRoom): boolean {
  return room === "out" || room === "low";
}

export function SwitchAccountButton({
  switchKey,
  accountId,
  why = null,
}: {
  /** The key its switch list listens on (`useHandoverFocus`). */
  switchKey: string;
  /** The session's own account (or folder), whose room decides its look. */
  accountId: string | null;
  /** Why it can't switch yet, when its list said so (the button stays
   * focusable and says why, BTN-5). */
  why?: string | null;
}): JSX.Element {
  const room = useOwnRoom(accountId);
  const primary = ownRoomWantsSwitch(room);
  return (
    <WhyButton
      size="md"
      variant={primary ? "primary" : "accent"}
      why={why}
      onClick={() => focusHandover(switchKey)}
      className="w-full justify-center gap-2"
      data-testid="switch-account"
      data-room={room ?? "ok"}
    >
      <IconStateHandover aria-hidden="true" width={16} height={16} />
      Switch account…
    </WhyButton>
  );
}
