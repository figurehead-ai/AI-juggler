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
import { RowChip, SwitchBox } from "./controls";
import { IconSearch } from "./bot/bot-icons";

/** The line under the switch, for where the index stands. */
export function indexWords(info: OutsideSearchInfo, estimate: OutsideSearchEstimateInfo | null): string {
  if (!info.enabled) {
    if (!estimate) return "Working out how big its index would be…";
    if (estimate.sessions === 0) return "There are no sessions started outside AI Juggler to index yet.";
    return `Its index would take about ${byteSize(estimate.size_bytes)} on this computer, for ${estimate.sessions} sessions.`;
  }
  const size = byteSize(info.size_bytes ?? 0);
  return info.indexed < info.sessions
    ? `Index: ${size} so far · ${info.indexed} of ${info.sessions} sessions. It builds in the background.`
    : `Index: ${size} · all ${info.sessions} sessions.`;
}

/** "Settings › Privacy", opening it at this setting (persona D3 on rc33:
 * History's empty search named it with no way to it). It only shows the
 * setting, with its own explanation: turning it on is the user's choice
 * there, never made from a link. */
export function OutsideSearchLink(): JSX.Element {
  return (
    <button
      type="button"
      onClick={() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "search" } }))}
      className="font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="history-privacy-link"
    >
      Settings › Privacy
    </button>
  );
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
    // AJ-E1558 (D-126, TGL-1): a setting that takes effect at once: the
    // kit's switch row, its chip; spacing, not a rule.
    <div className="flex flex-col gap-1.5 pt-2" data-testid="privacy-outside-search" data-settings-row>
      <label className="flex items-center gap-3 text-body text-ink">
        <RowChip icon={IconSearch} on={info.enabled} />
        <span className="flex-1 font-medium">Search the words of every session</span>
        <SwitchBox
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
          data-testid="outside-search-switch"
        />
      </label>
      <p className="pl-[42px] text-label text-muted">
        History can find what was said in sessions started outside AI Juggler too. For that, AI Juggler keeps an index
        of their words on this computer, read from the tools&apos; own files, which it never changes. Off by default; turning
        it off deletes the index.
      </p>
      <p className="pl-[42px] text-label text-ink" data-testid="outside-search-size" aria-live="polite">
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
