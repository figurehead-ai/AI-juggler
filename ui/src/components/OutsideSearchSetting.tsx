/**
 * AJ-E1538 — Settings › Privacy: search the words of every session,
 * including the ones started outside AI Juggler. Off by default; the size
 * of the index it would build is said before it's turned on, and the
 * index's real size while it's on. Turning it off deletes the index.
 */
import { useEffect, useState, type JSX } from "react";
import { errorText } from "../lib/bridge";
import {
  outsideSearchEstimate,
  outsideSearchSet,
  outsideSearchStatus,
  type OutsideSearchEstimateInfo,
  type OutsideSearchInfo,
} from "../lib/ipc/outsideSearch";
import { byteSize } from "../lib/compact";

/** The line under the switch, for where the index stands. */
export function indexWords(info: OutsideSearchInfo, estimate: OutsideSearchEstimateInfo | null): string {
  if (!info.enabled) {
    if (!estimate) return "Working out how big its index would be…";
    if (estimate.sessions === 0) return "There are no sessions started outside AI Juggler to index yet.";
    return `Its index would take about ${byteSize(estimate.size_bytes)} on this Mac, for ${estimate.sessions} sessions.`;
  }
  const size = byteSize(info.size_bytes ?? 0);
  return info.indexed < info.sessions
    ? `Index: ${size} so far · ${info.indexed} of ${info.sessions} sessions. It builds in the background.`
    : `Index: ${size} · all ${info.sessions} sessions.`;
}

export function OutsideSearchSetting(): JSX.Element | null {
  const [info, setInfo] = useState<OutsideSearchInfo | null>(null);
  const [estimate, setEstimate] = useState<OutsideSearchEstimateInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    const load = () =>
      void outsideSearchStatus().then(
        (next) => alive && setInfo(next),
        () => null,
      );
    load();
    const timer = window.setInterval(load, 5_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);
  // The estimate only matters while it's off; it's worked out once.
  useEffect(() => {
    if (info && !info.enabled && !estimate) void outsideSearchEstimate().then(setEstimate, () => null);
  }, [info, estimate]);
  if (!info) return null;
  return (
    <div className="flex flex-col gap-1.5 border-t border-hairline pt-3" data-testid="privacy-outside-search">
      <label className="flex items-center gap-2 text-label text-ink">
        <input
          type="checkbox"
          checked={info.enabled}
          disabled={busy}
          onChange={(event) => {
            const on = event.target.checked;
            setBusy(true);
            setError(null);
            void outsideSearchSet(on)
              .then(setInfo, (cause) => setError(errorText(cause)))
              .finally(() => setBusy(false));
          }}
          className="accent-[var(--color-accent)]"
          data-testid="outside-search-switch"
        />
        <span className="font-medium">Search the words of every session</span>
      </label>
      <p className="text-label text-muted">
        History can find what was said in sessions started outside AI Juggler too. For that, AI Juggler keeps an index
        of their words on this Mac, read from the tools&apos; own files, which it never changes. Off by default; turning
        it off deletes the index.
      </p>
      <p className="text-label text-ink" data-testid="outside-search-size" aria-live="polite">
        {indexWords(info, estimate)}
      </p>
      {error && (
        <p role="alert" className="text-label text-ink">
          {error}
        </p>
      )}
    </div>
  );
}
