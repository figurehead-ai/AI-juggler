/**
 * AJ-E704: the pairing wizard's failure states, each in its own words
 * with the one way on (Try again, Start over, Turn pairing back on) and
 * Close, in the dialog's fixed footer.
 */
import type { JSX } from "react";
import { Button } from "../../components/primitives";
import { DialogFooter } from "../../components/DialogFrame";
import type { PairingStateInfo } from "../../types/generated/PairingStateInfo";
import type { PairingMode } from "../PairingWizard";

const FAILURES: Record<string, { title: string; body: string; action: string; reEnable?: boolean }> = {
  expired: {
    title: "The pairing window closed",
    body: "Codes work for 2 minutes. Start again when the other machine is ready.",
    action: "Start again",
  },
  mismatch: {
    title: "The code didn't match",
    body: "Nothing was paired. Check the 6 digits on the other machine — it now shows a new code.",
    action: "Try again",
  },
  cancelled: {
    title: "Pairing was cancelled",
    body: "One of the machines cancelled, so nothing was paired.",
    action: "Start over",
  },
  locked_out: {
    title: "Pairing is paused",
    body: "Too many wrong codes in a row. Wait a minute, then try again.",
    action: "Try again",
  },
  disabled: {
    title: "Pairing is turned off",
    body: "After 20 wrong codes in a day, pairing turns off to protect this computer. Turn it back on only if you're the one pairing.",
    action: "Turn pairing back on",
    reEnable: true,
  },
  unreachable: {
    title: "Couldn't reach that machine",
    body: "Check both machines are on the same network — or connected by Tailscale or WireGuard — with Link on.",
    action: "Try again",
  },
  fingerprint: {
    title: "The fingerprint doesn't match the link",
    body: "Don't pair — something may be intercepting. Copy the pairing link again from the other machine.",
    action: "Start over",
  },
  not_open: {
    title: "Pairing isn't open on that machine",
    body: "Choose Add a machine on it first, then enter the code it shows.",
    action: "Try again",
  },
  busy: {
    title: "Another machine is pairing",
    body: "That machine is pairing with someone else right now. Wait a moment and try again.",
    action: "Try again",
  },
};

export function Failure({
  mode,
  state,
  busy,
  onRetry,
  onClose,
}: {
  mode: PairingMode;
  state: PairingStateInfo;
  busy: boolean;
  onRetry: (reEnable: boolean) => void;
  onClose: () => void;
}): JSX.Element {
  const known = FAILURES[state.reason ?? ""];
  // The re-enable action only exists on the machine that turned off.
  const reEnable = Boolean(known?.reEnable) && mode === "host";
  return (
    <div className="flex flex-col gap-3" role="alert" data-testid="pairing-failure" data-reason={state.reason ?? "unknown"}>
      <p className="text-sm font-medium">{known?.title ?? "Pairing didn't finish"}</p>
      <p className="text-xs leading-relaxed text-ink-2">{known?.body ?? state.message ?? "Nothing was paired."}</p>
      <DialogFooter>
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
        {(known?.reEnable !== true || reEnable) && (
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => onRetry(reEnable)}
            data-autofocus
            data-testid="pairing-retry"
          >
            {known?.action ?? "Try again"}
          </Button>
        )}
      </DialogFooter>
    </div>
  );
}
