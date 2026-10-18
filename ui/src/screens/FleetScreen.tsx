import { useCallback, useEffect, useRef, useState } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconFleet } from "../components/icons";
import {
  errorText,
  fleetPeers,
  fleetResetIdentity,
  fleetResetTrust,
  fleetSelf,
  fleetSetLabel,
  fleetUnpair,
  type FleetPeerInfo,
  type FleetSelfInfo,
} from "../lib/bridge";

/**
 * Fleet (beta) — AJ-E702: this Mac's machine identity and the machines
 * paired with it (ADR-0013 architecture contract). The fingerprint is
 * what you compare when pairing; Unpair and Reset identity are
 * confirmed in a sheet that names what changes. Pairing itself arrives
 * with the next Fleet build — the screen says so instead of offering a
 * button that does nothing.
 */
export function FleetScreen(): JSX.Element {
  const [self, setSelf] = useState<FleetSelfInfo | null>(null);
  const [peers, setPeers] = useState<FleetPeerInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<
    | { kind: "unpair"; peer: FleetPeerInfo }
    | { kind: "reset-identity" }
    | { kind: "reset-trust" }
    | null
  >(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [info, list] = await Promise.all([fleetSelf(), fleetPeers()]);
      setSelf(info);
      setPeers(list);
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
      await refresh();
      setConfirm(null);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-5" data-testid="fleet-screen">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-semibold tracking-tight">Fleet</h2>
            <Chip tone="accent">Beta</Chip>
          </div>
          <p className="text-xs text-muted">
            Your own machines in one console — paired by code, connected directly over
            your network. No relay, and vendor credentials never leave a machine.
          </p>
        </div>
      </header>

      {error && (
        <Card className="border-danger/30 bg-danger-soft p-3 text-xs text-danger" role="alert">
          {error}
        </Card>
      )}

      {self?.trust === "broken" && (
        <Card
          className="flex flex-col gap-2 border-warning/40 p-4"
          role="alert"
          data-testid="fleet-trust-broken"
        >
          <p className="text-sm font-medium">⚠ Trust broken — re-pair required</p>
          <p className="text-xs leading-relaxed text-muted">
            The list of paired machines failed its integrity check, so AI Juggler isn't
            trusting any of them. Nothing connects until you reset trust and pair again.
          </p>
          {self.trust_reason && (
            <details className="text-[11px] text-muted">
              <summary className="cursor-default">Details</summary>
              <p className="mt-1">{self.trust_reason}</p>
            </details>
          )}
          <Button
            size="sm"
            variant="secondary"
            className="self-start"
            onClick={() => setConfirm({ kind: "reset-trust" })}
            data-testid="fleet-reset-trust"
          >
            Reset trust…
          </Button>
        </Card>
      )}

      {self === null ? (
        <Card className="p-5 text-xs text-muted">Loading…</Card>
      ) : (
        <ThisMacCard
          self={self}
          onRename={(label) => run(() => fleetSetLabel(label))}
          onResetIdentity={() => setConfirm({ kind: "reset-identity" })}
        />
      )}

      <Card className="flex flex-col gap-3 p-5" data-testid="fleet-peers">
        <div className="flex items-center gap-2">
          <IconFleet width={14} height={14} />
          <h3 className="text-sm font-semibold">Paired machines</h3>
        </div>
        {peers.length === 0 ? (
          <div className="flex flex-col gap-1" data-testid="fleet-peers-empty">
            <p className="text-xs text-muted">No paired machines yet.</p>
            <p className="text-[11px] leading-relaxed text-muted">
              Pairing arrives in an upcoming beta build: you'll open Fleet on both
              machines, enter a 6-digit code, and compare each machine's fingerprint.
              This Mac's identity is ready for it.
            </p>
          </div>
        ) : (
          <ul className="flex flex-col divide-y divide-hairline">
            {peers.map((peer) => (
              <li
                key={peer.machine_id}
                className="flex flex-wrap items-center gap-3 py-2.5"
                data-testid="fleet-peer"
              >
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium">{peer.label}</p>
                  <p className="font-mono text-[11px] text-muted">{peer.fingerprint}</p>
                </div>
                <Chip tone="neutral">{peer.permission_label}</Chip>
                {peer.paired_via === "account" && (
                  <span className="text-[11px] text-muted">via your Figurehead account</span>
                )}
                <span className="text-[11px] text-muted">
                  {peer.last_seen_ms === null
                    ? "Not seen yet"
                    : `Last seen ${new Date(peer.last_seen_ms).toLocaleString()}`}
                </span>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setConfirm({ kind: "unpair", peer })}
                  aria-label={`Unpair ${peer.label}`}
                  data-testid="fleet-unpair"
                >
                  Unpair…
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      {confirm !== null && (
        <ConfirmSheet
          busy={busy}
          title={
            confirm.kind === "unpair"
              ? `Unpair ${confirm.peer.label}?`
              : confirm.kind === "reset-identity"
                ? "Reset this Mac's identity?"
                : "Reset trust?"
          }
          body={
            confirm.kind === "unpair"
              ? `${confirm.peer.label} will no longer connect to this Mac. To use it again you'll pair it again with a new code.`
              : confirm.kind === "reset-identity"
                ? "This Mac gets a new identity and fingerprint. Every paired machine is removed and must pair again."
                : "Every paired machine is removed. You'll pair each one again with a new code."
          }
          action={
            confirm.kind === "unpair"
              ? "Unpair"
              : confirm.kind === "reset-identity"
                ? "Reset identity"
                : "Reset trust"
          }
          onCancel={() => setConfirm(null)}
          onConfirm={() =>
            void run(() =>
              confirm.kind === "unpair"
                ? fleetUnpair(confirm.peer.machine_id)
                : confirm.kind === "reset-identity"
                  ? fleetResetIdentity()
                  : fleetResetTrust(),
            )
          }
        />
      )}
    </div>
  );
}

function ThisMacCard({
  self,
  onRename,
  onResetIdentity,
}: {
  self: FleetSelfInfo;
  onRename: (label: string) => Promise<void> | void;
  onResetIdentity: () => void;
}): JSX.Element {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(self.label);
  const [copied, setCopied] = useState(false);
  useEffect(() => setDraft(self.label), [self.label]);
  const valid = draft.trim().length > 0 && draft.trim().length <= 64;

  return (
    <Card className="flex flex-col gap-4 p-5" data-testid="fleet-this-mac">
      <div className="flex flex-wrap items-center gap-3">
        <div className="min-w-0 flex-1">
          <p className="text-[11px] uppercase tracking-wider text-muted">This Mac</p>
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
              <p className="text-base font-semibold" data-testid="fleet-label">
                {self.label}
              </p>
              <Button size="sm" variant="ghost" onClick={() => setEditing(true)} data-testid="fleet-rename">
                Rename
              </Button>
            </div>
          )}
          <p className="mt-0.5 text-[11px] text-muted">
            The name is only shown on this Mac and to machines you pair with.
          </p>
        </div>
      </div>

      <div className="flex flex-col gap-1.5">
        <p className="text-[11px] font-medium text-ink" id="fleet-fingerprint-label">
          Fingerprint
        </p>
        <div className="flex flex-wrap items-center gap-2">
          <code
            className="rounded-lg border border-hairline bg-canvas px-3 py-2 font-mono text-sm tracking-wide text-ink"
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
        <p className="text-[11px] leading-relaxed text-muted">
          When you pair, both machines show their fingerprints — check they match what
          each machine shows here before you confirm.
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-3 border-t border-hairline pt-3">
        <span className="text-[11px] text-muted">
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

/** A confirmation sheet for a change that removes trust: names what
 * changes, focuses Cancel by default (non-destructive), Esc cancels. */
function ConfirmSheet({
  title,
  body,
  action,
  busy,
  onCancel,
  onConfirm,
}: {
  title: string;
  body: string;
  action: string;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}): JSX.Element {
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    cancelRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previous?.focus?.();
    };
  }, [onCancel]);
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6">
      <Card
        className="w-full max-w-sm p-5"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="fleet-confirm-title"
        aria-describedby="fleet-confirm-body"
        data-testid="fleet-confirm"
      >
        <p id="fleet-confirm-title" className="text-sm font-semibold">
          {title}
        </p>
        <p id="fleet-confirm-body" className="mt-1.5 text-xs leading-relaxed text-muted">
          {body}
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <Button ref={cancelRef} size="sm" variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" variant="danger" disabled={busy} onClick={onConfirm} data-testid="fleet-confirm-action">
            {action}
          </Button>
        </div>
      </Card>
    </div>
  );
}
