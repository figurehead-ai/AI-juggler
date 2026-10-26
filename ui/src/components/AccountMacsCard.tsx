/**
 * AJ-E1206 — Fleet › Your Macs on Figurehead. With Fleet discovery on,
 * your other linked Macs are listed by name. Pairing one needs no code,
 * but it does need you: compare the two fingerprints on both Macs and
 * confirm on each. A key from the directory is only a candidate, so this
 * Mac pins it only after you confirm here, and the two connect only after
 * the other Mac confirms this one too. Paired this way, a Mac starts at
 * View and is labelled "via your Figurehead account".
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import {
  errorText,
  figureheadStatus,
  fleetAccountMacs,
  fleetAccountPin,
  type AccountMacInfo,
  type AccountMacsInfo,
} from "../lib/bridge";

const STATE_WORDS: Record<string, string> = {
  not_paired: "Not paired",
  waiting: "Confirmed here — waiting for it to confirm this Mac",
  paired: "Paired via your Figurehead account",
  no_key: "Can't pair this way — use a pairing code",
};

export function AccountMacsCard({
  onPaired,
  onShown,
}: {
  onPaired?: () => void;
  onShown?: (shown: boolean) => void;
}): JSX.Element | null {
  const [on, setOn] = useState(false);
  const [info, setInfo] = useState<AccountMacsInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [comparing, setComparing] = useState<AccountMacInfo | null>(null);
  const load = useCallback(() => {
    fleetAccountMacs()
      .then((next) => {
        setInfo(next);
        setError(null);
      })
      .catch((cause) => setError(errorText(cause)));
  }, []);
  useEffect(() => {
    void figureheadStatus()
      .then((status) => {
        const discovery = status.features.find((f) => f.key === "fleetDiscovery");
        const enabled = Boolean(discovery?.on && discovery.access === "included");
        setOn(enabled);
        onShown?.(enabled);
        if (enabled) load();
      })
      .catch(() => setOn(false));
  }, [load, onShown]);
  // Never faster than once a minute (the backend caches the same way).
  useEffect(() => {
    if (!on) return;
    const timer = window.setInterval(load, 60_000);
    return () => window.clearInterval(timer);
  }, [on, load]);
  if (!on) return null;
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="account-macs">
      <h3 className="text-sm font-semibold">Your Macs on Figurehead</h3>
      <p className="text-label text-muted">
        Linked to your account, found without a code. Figurehead only lists them — your Macs still connect directly to
        each other, and only after you compare fingerprints on both.
      </p>
      {error && (
        <p role="alert" className="text-label text-ink">
          {error}
        </p>
      )}
      {info && info.macs.length === 0 && <p className="text-label text-muted">No other Mac is linked to your account yet.</p>}
      <ul className="flex flex-col gap-1.5">
        {info?.macs.map((mac) => (
          <li key={mac.device_id} className="flex flex-wrap items-center gap-2 text-label" data-testid="account-mac">
            <span className="min-w-0 flex-1">
              <span className="font-medium text-ink">{mac.name || "A Mac"}</span>
              <span className="text-muted"> · {STATE_WORDS[mac.state] ?? mac.state}</span>
            </span>
            {mac.state === "not_paired" && (
              <button
                type="button"
                onClick={() => setComparing(mac)}
                className="rounded-lg border border-hairline px-2.5 py-1 text-label font-medium text-ink hover:border-accent/50"
                data-testid="account-mac-compare"
              >
                Compare and pair…
              </button>
            )}
          </li>
        ))}
      </ul>
      {comparing && info && (
        <CompareSheet
          mac={comparing}
          thisFingerprint={info.this_fingerprint}
          onClose={(paired) => {
            setComparing(null);
            if (paired) {
              load();
              onPaired?.();
            }
          }}
        />
      )}
    </Card>
  );
}

function CompareSheet({
  mac,
  thisFingerprint,
  onClose,
}: {
  mac: AccountMacInfo;
  thisFingerprint: string;
  onClose: (paired: boolean) => void;
}): JSX.Element {
  const [match, setMatch] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  const pair = async () => {
    if (!mac.fingerprint) return;
    try {
      await fleetAccountPin(mac.device_id, mac.fingerprint);
      onClose(true);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="compare-title"
        data-testid="account-compare"
        className="flex w-full max-w-lg flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="compare-title" className="text-sm font-semibold">
          Pair with {mac.name || "this Mac"}
        </h3>
        <p className="text-label text-ink">
          Open Fleet on {mac.name || "the other Mac"} too. Both Macs show the same two fingerprints — check they match,
          then confirm on each.
        </p>
        <dl className="grid grid-cols-1 gap-2 text-label sm:grid-cols-2">
          <div className="rounded-lg border border-hairline p-2.5">
            <dt className="text-muted">This Mac</dt>
            <dd className="select-all break-all font-mono text-ink" data-testid="compare-this">
              {thisFingerprint}
            </dd>
          </div>
          <div className="rounded-lg border border-hairline p-2.5">
            <dt className="text-muted">{mac.name || "The other Mac"}, from your account</dt>
            <dd className="select-all break-all font-mono text-ink" data-testid="compare-other">
              {mac.fingerprint}
            </dd>
          </div>
        </dl>
        <p className="text-label text-muted">
          If they don't match, don't pair — someone may have changed your account's list. It starts at View only; you can
          allow more later.
        </p>
        <label className="flex items-center gap-2 text-label text-ink">
          <input type="checkbox" checked={match} onChange={(event) => setMatch(event.target.checked)} data-testid="compare-match" />
          Both fingerprints match on both Macs
        </label>
        {error && (
          <p role="alert" className="text-label text-ink">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button type="button" onClick={() => onClose(false)} className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink">
            Cancel
          </button>
          <button
            type="button"
            disabled={!match}
            onClick={() => void pair()}
            className={
              "rounded-lg border px-3 py-1.5 text-xs font-medium " +
              (match ? "border-accent/60 text-ink" : "border-hairline text-muted")
            }
            data-testid="compare-pair"
          >
            Pair (View only)
          </button>
        </div>
      </div>
    </div>
  );
}
