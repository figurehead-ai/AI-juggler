/**
 * Figurehead & Cloud's parts (moved out of Settings.tsx, AJ-E2110): what
 * each cloud feature does and sends, its switch, the consent screen, the
 * connected view and the Unlink confirmation.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { RecoveryCodeSheet } from "./RecoveryCodeSheet";
import { CloudBackupPanel } from "./CloudBackupPanel";
import {
  figureheadOpenPage,
  figureheadRename,
  figureheadSaveChoices,
  figureheadUnlink,
  type CloudFeatureInfo,
  type FigureheadStatusInfo,
  vaultStatus,
  type VaultStatusInfo,
} from "../lib/bridge";

/** What each cloud feature does and sends (plain words, D-18 §4.5). */
const FEATURE_COPY: Record<string, { title: string; does: string; sends: string; note?: string; locked?: string }> = {
  cloudBackup: {
    title: "Cloud backup",
    does: "Back up AI Juggler's settings, Launch settings and history, and restore them on another Mac.",
    sends: "An encrypted backup file. Figurehead can't read it.",
  },
  fleetDiscovery: {
    title: "Find my Macs",
    does: "Lists your other linked Macs in Fleet so they can pair without a code — you still confirm each one.",
    sends: "This Mac's name, platform, app version and public key.",
    note: "Arrives in an upcoming beta build — your choice is kept until then.",
  },
  usageSync: {
    title: "Sync usage stats to my dashboard",
    does: "Shows your weekly usage statistics on figurehead.ai.",
    sends: "The weekly usage-statistics report, linked to your Figurehead account.",
    locked: "Turn on in Settings › Privacy, while sharing usage stats",
  },
  alerts: {
    title: "Alerts when you're away",
    does: "Sends AI Juggler's alerts to you when you're away from this Mac.",
    sends: "The alert's text and time.",
  },
  configSync: {
    title: "Sync config across Macs",
    does: "Keeps your rules and settings the same on your Macs — you review each change.",
    sends: "Encrypted config files. Figurehead can't read them.",
  },
  transcriptVault: {
    title: "Transcript vault",
    does: "Keeps an encrypted archive of your transcripts you can search on any of your Macs.",
    sends: "Encrypted transcripts. Figurehead can't read them.",
  },
};

/** The consent screen's defaults (D-18 §4.3): the services you connect
 * for are on; usage sync is opt-in; everything else starts off. */
const CONSENT_DEFAULT_ON = new Set(["cloudBackup", "fleetDiscovery"]);

function FeatureRow({
  feature,
  checked,
  disabled,
  onToggle,
}: {
  feature: CloudFeatureInfo;
  checked: boolean;
  disabled: boolean;
  onToggle: (on: boolean) => void;
}): JSX.Element {
  const copy = FEATURE_COPY[feature.key] ?? {
    title: feature.key,
    does: "A Figurehead cloud feature.",
    sends: "Only what this feature needs.",
  };
  const upgrade = feature.access === "upgrade";
  const missing = feature.access === "not_included";
  const locked = copy.locked;
  const off = upgrade || missing || Boolean(locked);
  const id = `cloud-feature-${feature.key}`;
  return (
    <li className="flex items-start gap-3 py-3" data-testid="cloud-feature" data-key={feature.key} data-access={feature.access}>
      <button
        type="button"
        role="switch"
        aria-checked={checked && !off}
        aria-labelledby={`${id}-title`}
        aria-describedby={`${id}-body`}
        disabled={disabled || off}
        onClick={() => onToggle(!checked)}
        className={
          "relative mt-0.5 h-5 w-9 shrink-0 rounded-full border transition-colors disabled:opacity-40 " +
          (checked && !off ? "border-accent bg-accent" : "border-hairline bg-surface-2")
        }
        data-testid="cloud-feature-switch"
      >
        <span
          aria-hidden="true"
          className={
            "absolute top-0.5 h-3.5 w-3.5 rounded-full bg-white shadow transition-all " +
            (checked && !off ? "left-[18px]" : "left-0.5")
          }
        />
      </button>
      <div className="min-w-0 flex-1">
        <p className="flex flex-wrap items-center gap-2 text-sm font-medium" id={`${id}-title`}>
          {copy.title}
          {feature.tier === "paid" && <Chip tone="accent">Paid</Chip>}
        </p>
        <div id={`${id}-body`} className="flex flex-col gap-0.5 text-[11px] leading-relaxed text-muted">
          <span>{copy.does}</span>
          <span>
            <span className="text-ink">Sends:</span> {copy.sends}
          </span>
          {locked && <span data-testid="cloud-feature-locked">{locked}</span>}
          {!locked && !off && copy.note && <span>{copy.note}</span>}
          {missing && <span>Not included in your plan.</span>}
        </div>
      </div>
      {upgrade && (
        <Button size="sm" variant="secondary" onClick={() => void figureheadOpenPage("upgrade")} data-testid="cloud-upgrade">
          Upgrade
        </Button>
      )}
    </li>
  );
}

export function Consent({
  status,
  busy,
  onSave,
}: {
  status: FigureheadStatusInfo;
  busy: boolean;
  onSave: (features: Record<string, boolean>) => void;
}): JSX.Element {
  const [chosen, setChosen] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(
      status.features.map((feature) => [
        feature.key,
        feature.access === "included" && CONSENT_DEFAULT_ON.has(feature.key),
      ]),
    ),
  );
  return (
    <div className="flex flex-col gap-2" data-testid="cloud-consent">
      <p className="text-sm font-medium">Turn on Cloud for this Mac</p>
      <p className="text-xs text-muted">
        Linked to {status.email ?? "your Figurehead account"}. Choose what this Mac may use — you can change
        any of it here later.
      </p>
      <ul className="flex flex-col divide-y divide-hairline">
        {status.features.map((feature) => (
          <FeatureRow
            key={feature.key}
            feature={feature}
            checked={Boolean(chosen[feature.key])}
            disabled={busy}
            onToggle={(on) => setChosen((now) => ({ ...now, [feature.key]: on }))}
          />
        ))}
      </ul>
      <div className="flex flex-wrap gap-2">
        <Button variant="primary" disabled={busy} onClick={() => onSave(chosen)} data-testid="cloud-turn-on">
          Turn on selected
        </Button>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => onSave(Object.fromEntries(status.features.map((f) => [f.key, false])))}
          data-testid="cloud-not-now"
        >
          Not now
        </Button>
      </div>
    </div>
  );
}

export function Connected({
  status,
  busy,
  run,
}: {
  status: FigureheadStatusInfo;
  busy: boolean;
  run: (action: () => Promise<FigureheadStatusInfo>) => Promise<void>;
}): JSX.Element {
  const [name, setName] = useState(status.device_name);
  const [confirmUnlink, setConfirmUnlink] = useState(false);
  useEffect(() => setName(status.device_name), [status.device_name]);
  const choices = Object.fromEntries(status.features.map((f) => [f.key, f.on]));
  // AJ-E1204: Cloud backup on but the recovery code not saved yet — the
  // sheet comes up (once per visit), and no backup uploads until it's done.
  const backupOn = status.features.some((f) => f.key === "cloudBackup" && f.on && f.access === "included");
  const [vault, setVault] = useState<VaultStatusInfo | null>(null);
  const [showCode, setShowCode] = useState(false);
  useEffect(() => {
    if (!backupOn) return;
    void vaultStatus().then((next) => {
      setVault(next);
      if (!next.code_saved) setShowCode(true);
    });
  }, [backupOn]);
  const renamed = name.trim() !== status.device_name && name.trim().length > 0 && name.trim().length <= 64;
  return (
    <div className="flex flex-col gap-4" data-testid="cloud-connected">
      {backupOn && vault && !vault.code_saved && (
        <p
          className="flex flex-wrap items-center gap-2 rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-[11px] text-ink"
          data-testid="vault-pending"
        >
          <span aria-hidden="true">⚠</span>
          <span className="flex-1">Cloud backup waits until you save your recovery code.</span>
          <Button size="sm" variant="secondary" onClick={() => setShowCode(true)} data-testid="vault-show-code">
            Show recovery code
          </Button>
        </p>
      )}
      {backupOn && vault?.code_saved && <CloudBackupPanel />}
      {showCode && (
        <RecoveryCodeSheet
          onClose={(saved) => {
            setShowCode(false);
            if (saved) void vaultStatus().then(setVault);
          }}
        />
      )}
      <dl className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div>
          <dt className="text-[11px] text-muted">Figurehead account</dt>
          <dd className="text-sm" data-testid="cloud-email">
            {status.email ?? "—"}
          </dd>
        </div>
        <div>
          <dt className="text-[11px] text-muted">Plan</dt>
          <dd className="text-sm">{status.plan ?? "—"}</dd>
        </div>
      </dl>
      {status.offline && (
        <p className="text-[11px] text-warning" role="status" data-testid="cloud-offline">
          ○ Offline — showing what Figurehead said
          {status.refreshed_at_ms ? ` at ${new Date(status.refreshed_at_ms).toLocaleString()}` : " last time"}.
          Everything on this Mac keeps working.
        </p>
      )}
      <form
        className="flex flex-wrap items-end gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (renamed) void run(() => figureheadRename(name.trim()));
        }}
      >
        <label className="flex flex-col gap-1 text-[11px] font-medium">
          This Mac's name on Figurehead
          <input
            value={name}
            maxLength={64}
            onChange={(event) => setName(event.target.value)}
            className="w-64 rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 text-sm font-normal text-ink outline-none focus:border-accent/50"
            data-testid="cloud-rename-input"
          />
        </label>
        <Button size="sm" variant="secondary" type="submit" disabled={busy || !renamed} data-testid="cloud-rename">
          Save name
        </Button>
      </form>
      <div>
        <p className="text-xs font-medium">Cloud features on this Mac</p>
        <ul className="flex flex-col divide-y divide-hairline">
          {status.features.map((feature) => (
            <FeatureRow
              key={feature.key}
              feature={feature}
              checked={feature.on}
              disabled={busy}
              onToggle={(on) => void run(() => figureheadSaveChoices({ ...choices, [feature.key]: on }))}
            />
          ))}
        </ul>
      </div>
      <div className="flex flex-wrap items-center gap-2 border-t border-hairline pt-3">
        <Button size="sm" variant="secondary" onClick={() => void figureheadOpenPage("dashboard")} data-testid="cloud-dashboard">
          Open dashboard
        </Button>
        <Button size="sm" variant="ghost" onClick={() => setConfirmUnlink(true)} data-testid="cloud-unlink">
          Unlink this Mac…
        </Button>
        <span className="flex-1" />
        <button
          type="button"
          className="text-[11px] text-accent-hi underline-offset-2 hover:underline"
          onClick={() => void figureheadOpenPage("dashboard")}
          data-testid="cloud-delete-data"
        >
          Delete cloud data on figurehead.ai
        </button>
      </div>
      {confirmUnlink && (
        <UnlinkSheet
          busy={busy}
          onCancel={() => setConfirmUnlink(false)}
          onConfirm={() => {
            setConfirmUnlink(false);
            void run(figureheadUnlink);
          }}
        />
      )}
    </div>
  );
}

function UnlinkSheet({
  busy,
  onCancel,
  onConfirm,
}: {
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
        aria-labelledby="unlink-title"
        aria-describedby="unlink-body"
        data-testid="cloud-unlink-confirm"
      >
        <p id="unlink-title" className="text-sm font-semibold">
          Unlink this Mac from Figurehead?
        </p>
        <p id="unlink-body" className="mt-1.5 text-xs leading-relaxed text-muted">
          Cloud features stop on this Mac and its sign-in is removed. Everything stored on this Mac
          stays as it is, and backups already on figurehead.ai stay there until you delete them.
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <Button ref={cancelRef} size="sm" variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" variant="danger" disabled={busy} onClick={onConfirm} data-testid="cloud-unlink-yes">
            Unlink
          </Button>
        </div>
      </Card>
    </div>
  );
}
