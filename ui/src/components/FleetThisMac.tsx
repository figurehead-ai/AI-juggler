/**
 * Fleet's This Mac card: its name (renamed in place), its fingerprint —
 * open while Link is on, folded otherwise (the review of 0.1.4: it stood
 * out with nothing to pair) — and how many Macs it's paired with.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import type { FleetSelfInfo } from "../lib/bridge";

export function ThisMacCard({
  self,
  linkOn = false,
  onRename,
  onResetIdentity,
}: {
  self: FleetSelfInfo;
  /** Link is on: the fingerprint, used to pair, shows open. */
  linkOn?: boolean;
  onRename: (label: string) => Promise<void> | void;
  onResetIdentity: () => void;
}): JSX.Element {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(self.label);
  const [copied, setCopied] = useState(false);
  useEffect(() => setDraft(self.label), [self.label]);
  const valid = draft.trim().length > 0 && draft.trim().length <= 64;

  return (
    <Card className="flex flex-col gap-4 p-card" data-testid="fleet-this-mac">
      <div className="flex flex-wrap items-center gap-3">
        <div className="min-w-0 flex-1">
          <p className="text-section font-semibold text-ink">This Mac</p>
          {editing ? (
            <form
              className="mt-1 flex flex-wrap items-center gap-2"
              onSubmit={(event) => {
                event.preventDefault();
                if (!valid) return;
                void onRename(draft);
                setEditing(false);
              }}
            >
              <input
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                aria-label="Machine name"
                maxLength={64}
                autoFocus
                className="rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 text-sm text-ink outline-none focus:border-accent/50"
                data-testid="fleet-label-input"
              />
              <Button size="sm" variant="primary" type="submit" disabled={!valid}>
                Save
              </Button>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  setDraft(self.label);
                  setEditing(false);
                }}
              >
                Cancel
              </Button>
            </form>
          ) : (
            <div className="mt-0.5 flex items-center gap-2">
              <p className="text-body font-semibold text-ink" data-testid="fleet-label">
                {self.label}
              </p>
              <Button size="sm" variant="ghost" onClick={() => setEditing(true)} data-testid="fleet-rename">
                Rename
              </Button>
            </div>
          )}
          <p className="mt-0.5 text-meta text-muted">
            Shown only here and to the Macs you pair with.
          </p>
        </div>
      </div>

      <details className="group flex flex-col gap-1.5" open={linkOn}>
        <summary className="flex cursor-pointer list-none items-center gap-1.5 text-meta font-medium text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden" id="fleet-fingerprint-label">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" className="shrink-0 text-muted transition-transform group-open:rotate-90 motion-reduce:transition-none">
            <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          Fingerprint, to check when you pair
        </summary>
        <div className="mt-1.5 flex flex-wrap items-center gap-2">
          <code
            className="rounded-lg border border-hairline bg-canvas px-3 py-1.5 font-mono text-meta tracking-wide text-ink"
            aria-label="Machine fingerprint"
            tabIndex={0}
            data-testid="fleet-fingerprint"
          >
            {self.fingerprint}
          </code>
          <Button
            size="sm"
            variant="secondary"
            onClick={async () => {
              await navigator.clipboard?.writeText(self.fingerprint).catch(() => null);
              setCopied(true);
              window.setTimeout(() => setCopied(false), 1500);
            }}
            data-testid="fleet-copy-fingerprint"
          >
            {copied ? "Copied" : "Copy"}
          </Button>
        </div>
        <p className="mt-1.5 text-meta text-muted">
          When you pair, both Macs show their fingerprints: check they match before you confirm.
        </p>
      </details>

      <div className="flex flex-wrap items-center gap-3 border-t border-hairline pt-3">
        <span className="text-meta tabular-nums text-muted">
          {self.peer_count === 1 ? "1 paired machine" : `${self.peer_count} paired machines`}
        </span>
        <span className="flex-1" />
        <Button size="sm" variant="ghost" onClick={onResetIdentity} data-testid="fleet-reset-identity">
          Reset this Mac's identity…
        </Button>
      </div>
    </Card>
  );
}
