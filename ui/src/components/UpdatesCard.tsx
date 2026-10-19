/**
 * AJ-E1010 — Settings › About & updates. The version and the Beta label
 * are always shown. Updates are announced, never installed silently: a
 * check shows the new version and what changed, and nothing happens
 * until Install. Install writes a backup first (the way back), then
 * downloads, verifies the signature and installs. A failure leaves the
 * running version untouched and says why, with Retry. Restart is the
 * user's call and waits until no Instance is running. A development
 * build says plainly that it does not update itself.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { ReleaseNotes } from "./ReleaseNotes";
import {
  errorText,
  onUpdateStatus,
  updateCheck,
  updateInstall,
  updateRestart,
  updateSetChannel,
  updateSetCheckOnLaunch,
  updateStatus,
  type UpdateChannel,
  type UpdateStatusInfo,
} from "../lib/bridge";

/** D-20: the channels AI Juggler chooses between; each says what it
 * means for you. Nightly builds are their own app, AI Juggler Nightly
 * (D-23), so they are never offered here. The backend's `channels`
 * decides which of these this edition shows. */
const CHANNELS: { id: UpdateChannel; title: string; consequence: string }[] = [
  { id: "stable", title: "Stable", consequence: "Well-tested releases. Recommended for everyday work." },
  {
    id: "beta",
    title: "Beta",
    consequence: "Newer features a little earlier. Tested, but less than Stable.",
  },
];

/** D-23: what the Nightly and Dev editions are, in a sentence. */
const EDITION_NOTE: Record<string, string> = {
  nightly:
    "AI Juggler Nightly is built from the newest code every night and updates from the nightly channel. " +
    "It has its own data, separate from AI Juggler, so keep everyday work there.",
  dev:
    "AI Juggler Dev is built from source on this Mac: it doesn't update itself and never checks. " +
    "It has its own data, separate from AI Juggler.",
};

function when(ms: number): string {
  return new Date(ms).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}

export function UpdatesCard(): JSX.Element {
  const [status, setStatus] = useState<UpdateStatusInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void updateStatus().then(setStatus);
    return onUpdateStatus(setStatus);
  }, []);
  const run = async (step: () => Promise<UpdateStatusInfo | void>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await step();
      if (next) setStatus(next);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  if (!status) return <Card className="p-5" data-testid="settings-updates" />;
  const working = busy || ["checking", "backing_up", "downloading"].includes(status.state);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-updates">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold">About &amp; updates</h3>
        <span
          className="rounded-full border border-warning/40 bg-warning-soft px-2 py-0.5 text-[10px] font-medium text-ink"
          data-testid="beta-label"
        >
          Beta
        </span>
      </div>
      <p className="text-[11px] text-ink" data-testid="update-version">
        {status.app_name} {status.current_version} · {status.channel} channel
      </p>
      {EDITION_NOTE[status.edition] && (
        <p className="text-[11px] text-muted" data-testid="update-edition-note">
          {EDITION_NOTE[status.edition]}
        </p>
      )}
      {!status.configured ? (
        status.edition === "dev" ? null : (
          <p className="text-[11px] text-muted" data-testid="update-unconfigured">
            This is a development build: it does not update itself and never checks. Releases are on GitHub.
          </p>
        )
      ) : (
        <>
          {status.channels.length > 1 && (
            <fieldset className="flex flex-col gap-1.5" data-testid="update-channel" disabled={working}>
              <legend className="mb-1 text-[11px] font-medium text-ink">Update channel</legend>
              {CHANNELS.filter((option) => status.channels.includes(option.id)).map((option) => (
                <label key={option.id} className="flex items-start gap-2 text-[11px]">
                  <input
                    type="radio"
                    name="update-channel"
                    value={option.id}
                    checked={status.channel === option.id}
                    onChange={() => void run(() => updateSetChannel(option.id))}
                    aria-describedby={`update-channel-${option.id}`}
                    data-testid={`update-channel-${option.id}`}
                  />
                  <span className="flex flex-col">
                    <span className="font-medium text-ink">{option.title}</span>
                    <span id={`update-channel-${option.id}`} className="text-muted">
                      {option.consequence}
                    </span>
                  </span>
                </label>
              ))}
              <p className="text-[11px] text-muted">
                Moving to a steadier channel never goes back to an older version: you get that channel's next release
                once it's newer than this one.
              </p>
            </fieldset>
          )}
          <label className="flex items-center gap-2 text-[11px] text-ink">
            <input
              type="checkbox"
              checked={status.check_on_launch}
              onChange={(event) => void run(() => updateSetCheckOnLaunch(event.target.checked))}
              data-testid="update-check-on-launch"
            />
            Check for updates when {status.app_name} starts
          </label>
          <p className="text-[11px] text-muted">
            A check only looks. Nothing is downloaded or installed until you choose Install, and AI Juggler never restarts
            by itself.
          </p>
          <div aria-live="polite" className="flex flex-col gap-2 text-[11px]" data-testid="update-state">
            {status.state === "checking" && <p className="text-ink">Checking…</p>}
            {status.state === "up_to_date" && (
              <p className="text-ink">
                <span aria-hidden="true">✓ </span>Up to date
                {status.checked_at_ms ? ` — checked ${when(status.checked_at_ms)}` : ""}.
              </p>
            )}
            {status.available && ["available", "failed", "backing_up", "downloading"].includes(status.state) && (
              <div className="flex flex-col gap-1.5 rounded-lg border border-hairline p-3" data-testid="update-available">
                <p className="font-medium text-ink">
                  {status.app_name} {status.available.version} ({status.channel}) is available
                  {status.available.date_ms ? ` — released ${when(status.available.date_ms)}` : ""}
                </p>
                <div className="max-h-48 overflow-y-auto text-muted" data-testid="update-notes">
                  {status.available.notes ? (
                    <ReleaseNotes text={status.available.notes} />
                  ) : (
                    "No release notes were published with this version."
                  )}
                </div>
                <p className="text-muted">
                  Installing writes a backup of AI Juggler's data first, so you can go back to {status.current_version}.
                </p>
              </div>
            )}
            {status.state === "backing_up" && <p className="text-ink">Backing up AI Juggler's data first…</p>}
            {status.state === "downloading" && (
              <div className="flex flex-col gap-1">
                <span className="text-ink">Downloading and verifying… {status.progress ?? 0}%</span>
                <div
                  role="progressbar"
                  aria-label="Update download"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={status.progress ?? 0}
                  className="h-1.5 w-full overflow-hidden rounded-full bg-surface-2"
                >
                  <div className="h-full rounded-full bg-accent" style={{ width: `${status.progress ?? 0}%` }} />
                </div>
              </div>
            )}
            {status.state === "installed" && (
              <div className="flex flex-col gap-0.5 text-ink" data-testid="update-installed">
                <p>
                  <span aria-hidden="true">✓ </span>
                  {status.available?.version ?? "The update"} is installed. It starts the next time AI Juggler starts.
                </p>
                {status.backup_path && <p className="break-all text-muted">Backup from before: {status.backup_path}</p>}
              </div>
            )}
            {status.state === "failed" && status.error && (
              <p role="alert" className="text-ink" data-testid="update-failed">
                <span aria-hidden="true">⚠ </span>
                {status.error}
              </p>
            )}
            {error && (
              <p role="alert" className="text-ink" data-testid="update-error">
                {error}
              </p>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            {status.state === "installed" ? (
              <button
                type="button"
                disabled={working}
                onClick={() => void run(() => updateRestart())}
                className="rounded-lg border border-accent/60 px-3 py-1.5 text-xs font-medium text-ink"
                data-testid="update-restart"
              >
                Restart now
              </button>
            ) : status.available && ["available", "failed"].includes(status.state) ? (
              <button
                type="button"
                disabled={working}
                onClick={() => void run(() => updateInstall())}
                className="rounded-lg border border-accent/60 px-3 py-1.5 text-xs font-medium text-ink"
                data-testid="update-install"
              >
                {status.state === "failed" ? "Retry" : `Install ${status.available.version}`}
              </button>
            ) : null}
            <button
              type="button"
              disabled={working}
              onClick={() => void run(() => updateCheck())}
              className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink disabled:text-muted"
              data-testid="update-check"
            >
              Check now
            </button>
          </div>
        </>
      )}
    </Card>
  );
}
