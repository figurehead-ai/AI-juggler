import { FleetPeersEmpty } from "../components/FleetPeersEmpty";
import { ClockSkewNote, ConnectionChip, SyncChip } from "../components/FleetChips";
import { AccountMacsCard } from "../components/AccountMacsCard";
import { useCallback, useEffect, useRef, useState, type JSX } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconFleet } from "../components/icons";
import { PairingWizard, type PairingMode } from "./PairingWizard";
import {
  errorText,
  fleetPeers,
  fleetRemote,
  fleetDivergences,
  type DivergenceRecordInfo,
  fleetSetPermission,
  linkSetEnabled,
  linkStatus,
  type RemoteMachineInfo,
  type LinkStatusInfo,
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
 * confirmed in a sheet that names what changes.
 *
 * AJ-E704: Link is off by default — while off this Mac is neither
 * visible to nor reachable from other machines. With Link on, "Add a
 * machine" shows a pairing code and "Join a machine" enters one
 * (`PairingWizard`).
 */
export function FleetScreen({
  onOpenCloud,
  onInspectRemote,
  onOpenHome,
}: {
  onOpenCloud?: () => void;
  /** AJ-E1524: set while no account is in — the empty list points back to
   * the welcome on Home. */
  onOpenHome?: () => void;
  /** AJ-E705: open a paired machine (or an Instance on it) in the inspector. */
  onInspectRemote?: (machineId: string, instanceId: string | null) => void;
} = {}): JSX.Element {
  const [self, setSelf] = useState<FleetSelfInfo | null>(null);
  // AJ-E1206: once your Macs are listed, the "find them" chip is redundant.
  const [discoveryShown, setDiscoveryShown] = useState(false);
  const [peers, setPeers] = useState<FleetPeerInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<
    | { kind: "unpair"; peer: FleetPeerInfo }
    | { kind: "reset-identity" }
    | { kind: "reset-trust" }
    | null
  >(null);
  const [busy, setBusy] = useState(false);
  const [link, setLink] = useState<LinkStatusInfo | null>(null);
  const [linkBusy, setLinkBusy] = useState(false);
  const [wizard, setWizard] = useState<PairingMode | null>(null);
  const [remote, setRemote] = useState<RemoteMachineInfo[]>([]);
  const [divergences, setDivergences] = useState<DivergenceRecordInfo[]>([]);
  const [access, setAccess] = useState<FleetPeerInfo | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [info, list, status] = await Promise.all([fleetSelf(), fleetPeers(), linkStatus()]);
      setSelf(info);
      setPeers(list);
      setLink(status);
      setError(null);
      // Last-known rows are shown even with Link off (AJ-E708: kept in
      // this Mac's replica, labelled as last known).
      setRemote(await fleetRemote().catch(() => []));
      setDivergences(await fleetDivergences().catch(() => []));
    } catch (cause) {
      setError(errorText(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Connection chips stay honest while Link is on.
  useEffect(() => {
    if (!link?.enabled) return;
    const timer = window.setInterval(() => void refresh(), 3000);
    return () => window.clearInterval(timer);
  }, [link?.enabled, refresh]);

  const toggleLink = async () => {
    setLinkBusy(true);
    try {
      setLink(await linkSetEnabled(!link?.enabled));
      await refresh();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setLinkBusy(false);
    }
  };
  const onPaired = useCallback(() => void refresh(), [refresh]);

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
        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            variant="secondary"
            disabled={!link?.enabled || self?.trust === "broken"}
            onClick={() => setWizard("join")}
            data-testid="fleet-join"
          >
            Join a machine…
          </Button>
          <Button
            size="sm"
            variant="primary"
            disabled={!link?.enabled || self?.trust === "broken"}
            onClick={() => setWizard("host")}
            data-testid="fleet-add"
          >
            Add a machine…
          </Button>
        </div>
        {onOpenCloud && !discoveryShown && (
          <button
            type="button"
            onClick={onOpenCloud}
            className="flex items-center gap-2 rounded-lg border border-hairline px-2.5 py-1.5 text-[11px] text-muted hover:text-ink"
            data-testid="fleet-cloud-chip"
          >
            Find your Macs through your Figurehead account
            <Chip tone="accent">Cloud</Chip>
          </button>
        )}
      </header>

      <Card className="flex flex-wrap items-center gap-3 p-4" data-testid="fleet-link">
        <button
          type="button"
          role="switch"
          aria-checked={link?.enabled ?? false}
          aria-labelledby="fleet-link-label"
          aria-describedby="fleet-link-detail"
          disabled={link === null || linkBusy}
          onClick={() => void toggleLink()}
          className={
            "relative h-5 w-9 shrink-0 rounded-full border transition-colors disabled:opacity-40 " +
            (link?.enabled ? "border-accent bg-accent" : "border-hairline bg-surface-2")
          }
          data-testid="fleet-link-switch"
        >
          <span
            aria-hidden="true"
            className={
              "absolute top-0.5 h-3.5 w-3.5 rounded-full bg-white shadow transition-all " +
              (link?.enabled ? "left-[18px]" : "left-0.5")
            }
          />
        </button>
        <div className="min-w-0 flex-1">
          <p id="fleet-link-label" className="text-sm font-medium">
            Link {link?.enabled ? "on" : "off"}
          </p>
          <p id="fleet-link-detail" className="text-[11px] leading-relaxed text-muted" data-testid="fleet-link-detail">
            {link?.enabled
              ? `Paired machines can connect to this Mac (port ${link.port ?? "—"}); machines on this network see an unnamed entry until you pair.`
              : "This Mac isn't visible to, or reachable from, other machines. Turn Link on to pair or connect."}
          </p>
          {link?.error && (
            <p className="mt-1 text-[11px] text-warning" role="status" data-testid="fleet-link-error">
              {link.error}
            </p>
          )}
        </div>
      </Card>

      <AccountMacsCard onShown={setDiscoveryShown} />

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
              <summary className="cursor-pointer">Details</summary>
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
          <FleetPeersEmpty onOpenHome={onOpenHome} />
        ) : (
          <>
          {peers
            .filter((peer) => peer.connection === "degraded" || peer.connection === "offline")
            .map((peer) => (
              <div
                key={`banner-${peer.machine_id}`}
                className={
                  "flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-xs " +
                  (peer.connection === "degraded" ? "border-warning/40" : "border-hairline")
                }
                role="status"
                data-testid="fleet-connection-banner"
                data-state={peer.connection}
              >
                <span aria-hidden="true">{peer.connection === "degraded" ? "◐" : "○"}</span>
                <span className="min-w-0 flex-1">
                  <span className="font-medium">{peer.label}</span>
                  {" — "}
                  {peer.connection === "degraded"
                    ? (peer.connection_detail ?? "telemetry only until both Macs are updated")
                    : peer.last_seen_ms !== null
                      ? `offline, last seen ${new Date(peer.last_seen_ms).toLocaleString()}`
                      : "offline"}
                </span>
                <Button size="sm" variant="secondary" onClick={() => void refresh()}>
                  Retry
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setConfirm({ kind: "unpair", peer })}
                  aria-label={`Unpair ${peer.label}`}
                >
                  Unpair…
                </Button>
              </div>
            ))}
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
                <ConnectionChip peer={peer} />
                <SyncChip
                  peer={peer}
                  diverged={divergences.filter((row) => row.machine_id === peer.machine_id).length}
                />
                <Chip tone="neutral">
                  {peer.permission === "control" && peer.ask_each_time
                    ? "View + control · asks each time"
                    : peer.permission_label}
                </Chip>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setAccess(peer)}
                  data-testid="fleet-access"
                >
                  Access…{" "}<span className="sr-only">for {peer.label} on this Mac</span>
                </Button>
                {peer.paired_via === "account" && (
                  <span className="text-[11px] text-muted">via your Figurehead account</span>
                )}
                {!peer.pairing_unfinished && (
                  <span className="text-[11px] text-muted">
                    {peer.last_seen_ms === null
                      ? "Not seen yet"
                      : `Last seen ${new Date(peer.last_seen_ms).toLocaleString()}`}
                  </span>
                )}
                {onInspectRemote && (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => onInspectRemote(peer.machine_id, null)}
                    aria-label={`Inspect ${peer.label}`}
                    data-testid="fleet-inspect"
                  >
                    Inspect
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setConfirm({ kind: "unpair", peer })}
                  aria-label={`Unpair ${peer.label}`}
                  data-testid="fleet-unpair"
                >
                  Unpair…
                </Button>
                {peer.pairing_unfinished && (
                  // Its own line in the row: the longer note never squeezes
                  // the fingerprint.
                  <div className="flex basis-full flex-wrap items-center gap-3">
                    <span
                      className="text-[11px] text-muted"
                      role="status"
                      data-testid="fleet-pairing-unfinished"
                    >
                      Pairing may not have finished on {peer.label}. Pair again to be sure.
                    </span>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={!link?.enabled || self?.trust === "broken"}
                      onClick={() => setWizard("host")}
                      data-testid="fleet-pair-again"
                    >
                      Pair again<span className="sr-only"> with {peer.label}</span>
                    </Button>
                  </div>
                )}
                <ClockSkewNote
                  label={peer.label}
                  skewMs={remote.find((entry) => entry.machine_id === peer.machine_id)?.clock_skew_ms ?? null}
                />
                <RemoteInstances
                  machine={remote.find((entry) => entry.machine_id === peer.machine_id) ?? null}
                  onInspect={(instanceId) => onInspectRemote?.(peer.machine_id, instanceId)}
                />
              </li>
            ))}
          </ul>
          </>
        )}
      </Card>

      {wizard !== null && self !== null && (
        <PairingWizard
          mode={wizard}
          selfLabel={self.label}
          selfFingerprint={self.fingerprint}
          onPaired={onPaired}
          onClose={() => {
            setWizard(null);
            void refresh();
          }}
        />
      )}

      {access !== null && (
        <AccessSheet
          peer={access}
          busy={busy}
          onCancel={() => setAccess(null)}
          onSave={(permission, ask) =>
            void run(async () => {
              await fleetSetPermission(access.machine_id, permission, ask);
              setAccess(null);
            })
          }
        />
      )}

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

/** AJ-E705: a connected machine's Instances, one click from the
 * inspector. Nothing is shown for a machine that hasn't reported. AJ-E708:
 * a machine that is away keeps its last-known rows, labelled as such. */
function RemoteInstances({
  machine,
  onInspect,
}: {
  machine: RemoteMachineInfo | null;
  onInspect: (instanceId: string) => void;
}): JSX.Element | null {
  if (!machine || machine.instances.length === 0) return null;
  const lastKnown = machine.freshness !== "live";
  const asOf = machine.last_heard_ms ?? machine.synced_at_ms;
  return (
    <>
    {lastKnown && (
      <p className="w-full pl-1 text-[11px] text-muted" data-testid="fleet-last-known">
        Last known{asOf !== null ? ` · as of ${new Date(asOf).toLocaleString()}` : ""} — may have changed since.
      </p>
    )}
    <ul className="flex w-full flex-col gap-1 pl-1" aria-label={`Sessions on ${machine.label}${lastKnown ? " (last known)" : ""}`} data-testid="fleet-remote-instances">
      {machine.instances.map((row) => (
        <li key={row.id}>
          <button
            type="button"
            className="flex w-full flex-wrap items-center gap-2 rounded-lg border border-hairline px-2.5 py-1.5 text-left text-xs hover:border-accent/40"
            onClick={() => onInspect(row.id)}
            data-testid="fleet-remote-instance"
          >
            <span className="font-medium">{row.account_label}</span>
            <span className="text-muted">{row.provider}</span>
            {row.project_name && <span className="text-muted">· {row.project_name}</span>}
            {row.model && <span className="text-muted">· {row.model}</span>}
            <span className="flex-1" />
            <Chip tone="neutral">{lastKnown ? `was ${row.status.replace(/_/g, " ")}` : row.status.replace(/_/g, " ")}</Chip>
          </button>
        </li>
      ))}
    </ul>
    </>
  );
}

/** AJ-E709: what a paired machine may do on THIS Mac. Changed only here
 * — the click on this Mac is the confirmation; a console can never raise
 * its own access (ADR-0013). */
export function AccessSheet({
  peer,
  busy,
  onCancel,
  onSave,
}: {
  peer: FleetPeerInfo;
  busy: boolean;
  onCancel: () => void;
  onSave: (permission: "view" | "control", askEachTime: boolean) => void;
}): JSX.Element {
  const initial =
    peer.permission === "control" ? (peer.ask_each_time ? "ask" : "control") : "view";
  const [choice, setChoice] = useState<"view" | "ask" | "control">(initial);
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
  const options: Array<{ id: "view" | "ask" | "control"; title: string; body: string }> = [
    { id: "view", title: "View only", body: `${peer.label} can see this Mac's accounts, sessions and usage — it can't change anything.` },
    {
      id: "ask",
      title: "View + control, ask each time",
      body: `${peer.label} can ask to start or stop work here; each request waits for you to Allow it on this Mac.`,
    },
    {
      id: "control",
      title: "View + control",
      body: `${peer.label} can start and stop work here using Launch settings that already exist on this Mac — it can never read credentials.`,
    },
  ];
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6">
      <Card
        className="flex w-full max-w-md flex-col gap-3 p-5"
        role="dialog"
        aria-modal="true"
        aria-labelledby="fleet-access-title"
        data-testid="fleet-access-sheet"
      >
        <p id="fleet-access-title" className="text-sm font-semibold">
          What may {peer.label} do on this Mac?
        </p>
        <div role="radiogroup" aria-labelledby="fleet-access-title" className="flex flex-col gap-2">
          {options.map((option) => (
            <label
              key={option.id}
              className={
                "flex items-start gap-2.5 rounded-lg border px-3 py-2.5 " +
                (choice === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
              }
              data-testid={`fleet-access-${option.id}`}
            >
              <input
                type="radio"
                name="fleet-access"
                className="mt-0.5"
                checked={choice === option.id}
                onChange={() => setChoice(option.id)}
              />
              <span className="flex flex-col gap-0.5">
                <span className="text-sm font-medium">{option.title}</span>
                <span className="text-[11px] leading-relaxed text-muted">{option.body}</span>
              </span>
            </label>
          ))}
        </div>
        <p className="text-[11px] text-muted">
          Only this Mac can change this. {peer.label} sees the change within half a minute.
        </p>
        <div className="flex justify-end gap-2">
          <Button ref={cancelRef} size="sm" variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            size="sm"
            variant="primary"
            disabled={busy || choice === initial}
            onClick={() => onSave(choice === "view" ? "view" : "control", choice === "ask")}
            data-testid="fleet-access-save"
          >
            Change on this Mac
          </Button>
        </div>
      </Card>
    </div>
  );
}
