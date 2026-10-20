/**
 * AJ-E1010 — Settings › About & updates. The version and the Beta label
 * are always shown. Updates are announced, never installed silently: a
 * check shows the new version and what changed, and nothing happens
 * until Install. Install writes a backup first (the way back), then
 * downloads, verifies the signature and installs. A failure leaves the
 * running version untouched and says why in plain words, with Try
 * again (the technical reason is behind Details). "Restart to update" is
 * the user's call and is offered only when it is safe — no session
 * running, no switch in flight, no bring-in under way; until then the
 * card says what it is waiting for. A development build says plainly
 * that it does not update itself.
 */
import { useEffect, useRef, useState, type JSX } from "react";
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

/** The channel's name as the update prompt shows it (D-04/D-11: the
 * beta channel is always labelled "Beta"). */
export function channelLabel(channel: string): string {
  return channel === "stable" ? "Stable" : channel === "nightly" ? "Nightly" : "Beta";
}

/** The channel chip: the same look in About and in every update prompt. */
export function ChannelChip({ channel, testId }: { channel: string; testId?: string }): JSX.Element {
  return (
    <span
      className="rounded-full border border-warning/40 bg-warning-soft px-2 py-0.5 text-[10px] font-medium text-ink"
      data-testid={testId}
    >
      {channelLabel(channel)}
    </span>
  );
}

/** A failure's headline, by kind (the sentence under it says why). */
const FAILED_TITLE: Record<string, string> = {
  network: "The download didn't finish",
  feed: "Couldn't read the update",
  signature: "Not installed — the download didn't check out",
  wrong_version: "Not installed — the download didn't check out",
  install: "Couldn't install",
  not_restored: "The update didn't finish",
  disk: "Couldn't make the backup",
};

/** A refused update step in plain words: the backend's "invalid update:"
 * framing (and a thrown Error's prefix) dropped, the sentence capitalised. */
export function updateErrorText(cause: unknown): string {
  const text = errorText(cause)
    .replace(/^Error:\s*/, "")
    .replace(/^invalid update( channel)?:\s*/, "");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function when(ms: number): string {
  return new Date(ms).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}

export function UpdatesCard({ focus = false }: { focus?: boolean } = {}): JSX.Element {
  const [status, setStatus] = useState<UpdateStatusInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    void updateStatus().then(setStatus);
    return onUpdateStatus(setStatus);
  }, []);
  // Opened from an update prompt: bring the card into view, focus on it.
  const loaded = status !== null;
  useEffect(() => {
    if (!focus || !loaded) return;
    heading.current?.scrollIntoView?.({ block: "start" });
    heading.current?.focus({ preventScroll: true });
  }, [focus, loaded]);
  const run = async (step: () => Promise<UpdateStatusInfo | void>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await step();
      if (next) setStatus(next);
    } catch (cause) {
      setError(updateErrorText(cause));
    } finally {
      setBusy(false);
    }
  };
  if (!status) return <Card className="p-5" data-testid="settings-updates" />;
  const working = busy || ["checking", "backing_up", "downloading"].includes(status.state);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-updates">
      <div className="flex flex-wrap items-center gap-2">
        <h3 ref={heading} tabIndex={-1} id="settings-updates-title" className="scroll-mt-4 text-sm font-semibold outline-none">
          About &amp; updates
        </h3>
        <ChannelChip channel="beta" testId="beta-label" />
      </div>
      <p className="text-[11px] text-ink" data-testid="update-version">
        {status.app_name} {status.current_version} ·{" "}
        {status.edition === "dev" ? "built on this Mac" : `${status.channel} channel`}
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
            A check only looks. Nothing is downloaded or installed until you choose Install, and {status.app_name} never restarts
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
                <p className="flex flex-wrap items-center gap-2 font-medium text-ink">
                  <span>
                    {status.app_name} {status.available.version} is available
                    {status.available.date_ms ? ` — released ${when(status.available.date_ms)}` : ""}
                  </span>
                  <ChannelChip channel={status.channel} testId="update-available-channel" />
                </p>
                <div
                  className="max-h-48 overflow-y-auto rounded text-muted focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  data-testid="update-notes"
                  role="region"
                  aria-label={`What's new in ${status.available.version}`}
                  tabIndex={0}
                >
                  {status.available.notes ? (
                    <ReleaseNotes text={status.available.notes} />
                  ) : (
                    "No release notes were published with this version."
                  )}
                </div>
                <p className="text-muted">
                  Installing writes a backup of {status.app_name}'s data first, so you can go back to {status.current_version}.
                </p>
              </div>
            )}
            {status.state === "backing_up" && <p className="text-ink">Backing up {status.app_name}'s data first…</p>}
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
                  {status.available?.version ?? "The update"} is installed. It starts the next time {status.app_name} starts.
                </p>
                {status.restart_wait && (
                  <p className="text-muted" data-testid="update-restart-wait">
                    {status.restart_wait} Nothing restarts by itself.
                  </p>
                )}
                {status.backup_path && <p className="break-all text-muted">Backup from before: {status.backup_path}</p>}
              </div>
            )}
            {status.state === "failed" && status.error && (
              <div role="alert" className="flex flex-col gap-1 text-ink" data-testid="update-failed">
                <p className="font-medium">
                  <span aria-hidden="true">⚠ </span>
                  {status.available
                    ? (FAILED_TITLE[status.error_kind ?? ""] ?? "The update didn't finish")
                    : "Couldn't check for updates"}
                </p>
                <p>{status.error}</p>
                {status.error_detail && (
                  <details className="text-muted" data-testid="update-failed-details">
                    <summary className="w-fit cursor-pointer text-ink underline">Details</summary>
                    <p className="mt-1 break-all font-mono text-[10px]">{status.error_detail}</p>
                  </details>
                )}
              </div>
            )}
            {error && (
              <p role="alert" className="text-ink" data-testid="update-error">
                {error}
              </p>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            {status.state === "installed" ? (
              status.restart_wait ? null : (
                <button
                  type="button"
                  disabled={working}
                  onClick={() => void run(() => updateRestart())}
                  className="rounded-lg border border-accent/60 px-3 py-1.5 text-xs font-medium text-ink"
                  data-testid="update-restart"
                >
                  Restart to update
                </button>
              )
            ) : status.available && ["available", "failed"].includes(status.state) ? (
              <button
                type="button"
                disabled={working}
                onClick={() => void run(() => updateInstall())}
                className="rounded-lg border border-accent/60 px-3 py-1.5 text-xs font-medium text-ink"
                data-testid="update-install"
              >
                {status.state === "failed" ? "Try again" : `Install ${status.available.version}`}
              </button>
            ) : null}
            {status.state === "installed" ? null : (
              <button
                type="button"
                disabled={working}
                onClick={() => void run(() => updateCheck())}
                className={
                  status.state === "failed" && !status.available
                    ? "rounded-lg border border-accent/60 px-3 py-1.5 text-xs font-medium text-ink"
                    : "rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink disabled:text-muted"
                }
                data-testid="update-check"
              >
                {status.state === "failed" && !status.available ? "Try again" : "Check now"}
              </button>
            )}
          </div>
        </>
      )}
    </Card>
  );
}
