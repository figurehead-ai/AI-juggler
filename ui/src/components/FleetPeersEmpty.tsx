/**
 * Fleet's empty list: a picture of what pairing does, drawn from the
 * page's own parts (this Mac, the code, your other Mac; no stock art),
 * how to pair in three steps, and (AJ-E1524) while no account is in, the
 * line back to the welcome on Home.
 */
import type { JSX, ReactNode } from "react";
import { WelcomePointer } from "./Welcome";
import { IconComputer } from "./icons-actions";
import { IconFleet } from "./icons";

/** How to pair, in three steps (the review of 0.1.4: a paragraph). */
const STEPS = [
  "Turn Link on here and on your other Mac.",
  "Choose Add a machine here, and Join a machine on the other Mac.",
  "Enter the 6-digit code, and check both fingerprints match.",
];

function MacTile({ name, sub, waiting = false }: { name: string; sub: string; waiting?: boolean }): JSX.Element {
  return (
    <div className={"flex w-56 min-w-0 shrink items-center gap-3 rounded-xl border px-3 py-2.5 " + (waiting ? "border-dashed border-hairline bg-transparent" : "border-hairline bg-surface-2")}>
      <span className={"flex size-9 shrink-0 items-center justify-center rounded-lg " + (waiting ? "border border-dashed border-hairline text-muted" : "bg-accent-soft text-accent-hi")}>
        <IconComputer width={18} height={18} />
      </span>
      <span className="flex min-w-0 flex-col">
        <span className={"truncate text-meta font-semibold " + (waiting ? "text-muted" : "text-ink")}>{name}</span>
        <span className="truncate text-label text-muted">{sub}</span>
      </span>
    </div>
  );
}

/** This Mac and the other, joined by the pairing code. */
function PairPicture({ selfLabel, linkOn }: { selfLabel: string; linkOn: boolean }): JSX.Element {
  const line: ReactNode = <span className="h-px min-w-4 flex-1 border-t border-dashed border-hairline" />;
  return (
    <div aria-hidden="true" className="rounded-xl bg-surface-2/50 px-4 py-6" data-testid="fleet-pair-picture">
      <div className="mx-auto flex w-full max-w-[680px] items-center justify-center gap-3">
        <MacTile name={selfLabel} sub={linkOn ? "This Mac, ready to pair" : "This Mac"} />
        {line}
        <span className="flex shrink-0 flex-col items-center gap-1">
          <span className="flex size-8 items-center justify-center rounded-full border border-hairline bg-surface text-muted">
            <IconFleet width={15} height={15} />
          </span>
          <span className="font-mono text-label tracking-[0.2em] text-muted">••• •••</span>
        </span>
        {line}
        <MacTile name="Your other Mac" sub="Join a machine…" waiting />
      </div>
    </div>
  );
}

export function FleetPeersEmpty({ onOpenHome, selfLabel, linkOn = false }: { onOpenHome?: () => void; selfLabel?: string; linkOn?: boolean }): JSX.Element {
  return (
    <div className="flex flex-col gap-4" data-testid="fleet-peers-empty">
      {selfLabel && <PairPicture selfLabel={selfLabel} linkOn={linkOn} />}
      <div className="flex flex-col gap-3">
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
        <p className="text-meta text-ink-2">A Mac on another network? Connect them with Tailscale or WireGuard first, then pair by its address.</p>
      </div>
      {onOpenHome && <WelcomePointer onOpenHome={onOpenHome} />}
    </div>
  );
}
