/**
 * AJ-E2504 (§3, §8; the approved design of 2026-10-27) — when AI
 * Juggler's own service stops answering: the window keeps showing the
 * last state it knew (never a blank, never a log-out), says since when,
 * pauses what would start something, and offers Retry.
 */
import { useEffect, useState, type JSX } from "react";
import { Banner } from "./Banner";
import { useBackendHealth } from "../lib/backend-health";

function ago(ms: number): string {
  const seconds = Math.max(1, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds} s ago`;
  const minutes = Math.round(seconds / 60);
  return minutes < 60 ? `${minutes} min ago` : `${Math.round(minutes / 60)} h ago`;
}

export function OfflineBanner({ onRetry }: { onRetry: () => void }): JSX.Element | null {
  const { downSince } = useBackendHealth();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (downSince === null) return;
    setNow(Date.now());
    // The "… ago" words, while it lasts (5 s is plenty for them).
    const timer = window.setInterval(() => setNow(Date.now()), 5_000);
    return () => window.clearInterval(timer);
  }, [downSince]);
  if (downSince === null) return null;
  return (
    <div className="mb-4 shrink-0">
      <Banner
        tone="danger"
        role="alert"
        title="Showing the last state AI Juggler knew"
        detail={`It couldn't reach its own service ${ago(now - downSince)}. Actions are paused until it's back.`}
        actions={[{ label: "Retry", onClick: onRetry, testId: "offline-retry" }]}
        testId="offline-banner"
      />
    </div>
  );
}
