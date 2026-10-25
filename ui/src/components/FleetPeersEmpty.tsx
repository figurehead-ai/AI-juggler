/**
 * Fleet's empty list: how to pair, and (AJ-E1524) while no account is in,
 * the line back to the welcome on Home.
 */
import type { JSX } from "react";
import { WelcomePointer } from "./Welcome";

export function FleetPeersEmpty({ onOpenHome }: { onOpenHome?: () => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-1" data-testid="fleet-peers-empty">
      <p className="text-xs text-muted">No paired machines yet.</p>
      <p className="text-label leading-relaxed text-muted">
        Turn Link on here and on the other machine. Then choose Add a machine on one
        and Join a machine on the other, enter the 6-digit code, and compare
        fingerprints. Machines elsewhere? Connect them with Tailscale or WireGuard
        first, then pair by address.
      </p>
      {onOpenHome && <WelcomePointer onOpenHome={onOpenHome} />}
    </div>
  );
}
