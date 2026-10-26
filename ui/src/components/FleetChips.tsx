/**
 * Fleet's words for another Mac: how it's connected (AJ-E703), how its
 * sync stands (AJ-E708), and a clock that's off (each in words and a
 * shape, never colour alone). Split from FleetScreen (the size budget).
 */
import type { JSX } from "react";
import { Chip } from "./primitives";
import type { FleetPeerInfo } from "../lib/bridge";

/** AJ-E703: the link state as words + a shape (never colour alone):
 * `Connected · mTLS · v1.0`, `Degraded — …`, `Offline — last seen …`,
 * `Connecting…`, `Link off`. */
export function ConnectionChip({ peer }: { peer: FleetPeerInfo }): JSX.Element {
  const label =
    peer.connection === "connected"
      ? `● Connected · mTLS · ${peer.protocol ?? "v1"}`
      : peer.connection === "degraded"
        ? `◐ Degraded · ${peer.protocol ?? "older version"}`
        : peer.connection === "connecting"
          ? "◌ Connecting…"
          : peer.connection === "offline"
            ? "○ Offline"
            : "○ Link off";
  const tone =
    peer.connection === "connected"
      ? "success"
      : peer.connection === "degraded"
        ? "warning"
        : "neutral";
  return (
    <span data-testid="fleet-connection" data-state={peer.connection}>
      <Chip tone={tone}>{label}</Chip>
    </span>
  );
}


/** AJ-E708: the sync state, in words + a shape (never colour alone):
 * `✓ Synced` while connected, or — while that machine is away — how many
 * changes on THIS Mac wait for it (`⇡ 12 changes queued`). */
export function SyncChip({ peer, diverged = 0 }: { peer: FleetPeerInfo; diverged?: number }): JSX.Element | null {
  if (diverged > 0) {
    // AJ-E708: a divergence waits for the user — it outranks every
    // other sync state (and blocks switch-back / clean until chosen).
    return (
      <span
        data-testid="fleet-sync"
        data-state="diverged"
        title={`Work that moved between this Mac and ${peer.label} changed on both — open the session to choose what to keep.`}
      >
        <Chip tone="warning">{`⚠ Diverged — ${diverged} ${diverged === 1 ? "needs" : "need"} you`}</Chip>
      </span>
    );
  }
  if (peer.connection === "link_off" && peer.queued_changes === null) return null;
  const live = peer.connection === "connected" || peer.connection === "degraded";
  const queued = peer.queued_changes ?? 0;
  const label = live
    ? "✓ Synced"
    : peer.connection === "connecting"
      ? "◌ Syncing…"
      : queued > 0
        ? `⇡ ${queued} ${queued === 1 ? "change" : "changes"} queued`
        : "✓ Nothing queued";
  const title = live
    ? `${peer.label} receives this Mac's changes as they happen.`
    : queued > 0
      ? `${queued} ${queued === 1 ? "change" : "changes"} on this Mac will reach ${peer.label} when it reconnects — only those, nothing it already has.`
      : `${peer.label} already has every change from this Mac.`;
  return (
    <span data-testid="fleet-sync" data-state={live ? "synced" : queued > 0 ? "queued" : "idle"} title={title}>
      <Chip tone={live ? "success" : queued > 0 ? "warning" : "neutral"}>{label}</Chip>
    </span>
  );
}

/** AJ-E708: a peer clock more than 5 minutes off is flagged, never
 * trusted for order — sync orders by that machine's sequence. */
const SKEW_FLAG_MS = 5 * 60 * 1000;

export function ClockSkewNote({ label, skewMs }: { label: string; skewMs: number | null }): JSX.Element | null {
  if (skewMs === null || Math.abs(skewMs) <= SKEW_FLAG_MS) return null;
  const minutes = Math.round(Math.abs(skewMs) / 60_000);
  const direction = skewMs > 0 ? "ahead of" : "behind";
  return (
    <p className="w-full text-label text-ink" role="note" data-testid="fleet-clock-skew">
      <span aria-hidden="true">⚠ </span>
      {label}'s clock is {minutes} min {direction} this Mac's — times it reports may look off. Order
      always follows its own change sequence, never its clock.
    </p>
  );
}
