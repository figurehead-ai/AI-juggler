/**
 * Fleet's empty list: how to pair, and (AJ-E1524) while no account is in,
 * the line back to the welcome on Home.
 */
import type { JSX } from "react";
import { WelcomePointer } from "./Welcome";

/** How to pair, in three steps (the review of 0.1.4: a paragraph). */
const STEPS = [
  "Turn Link on here and on your other Mac.",
  "Choose Add a machine here, and Join a machine on the other Mac.",
  "Enter the 6-digit code, and check both fingerprints match.",
];

export function FleetPeersEmpty({ onOpenHome }: { onOpenHome?: () => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-3 rounded-lg border border-dashed border-hairline px-5 py-5" data-testid="fleet-peers-empty">
      <p className="text-body font-medium text-ink">No paired Macs yet</p>
      <ol className="flex flex-col gap-2">
        {STEPS.map((step, index) => (
          <li key={step} className="flex items-start gap-3 text-body text-muted">
            <span aria-hidden="true" className="flex size-6 shrink-0 items-center justify-center rounded-full bg-accent-soft text-label font-semibold tabular-nums text-accent-hi">
              {index + 1}
            </span>
            <span className="pt-0.5">{step}</span>
          </li>
        ))}
      </ol>
      <p className="text-meta text-muted">A Mac on another network? Connect them with Tailscale or WireGuard first, then pair by its address.</p>
      {onOpenHome && <WelcomePointer onOpenHome={onOpenHome} />}
    </div>
  );
}
