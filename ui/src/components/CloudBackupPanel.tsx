/**
 * AJ-E1205 — Cloud backup inside Account & Cloud, once it's on and the
 * recovery code is saved: when the last backup ran (or why it failed),
 * the schedule (off / daily / weekly), Back up now, and Restore from
 * Cloud…. Restoring lists the account's backups, asks for the recovery
 * code when this Mac doesn't have the key, then hands the verified plan
 * to Backup & restore's dialog, so a cloud restore is confirmed exactly
 * like a local one.
 */
import { useEffect, useState } from "react";
import {
  cloudBackupList,
  cloudBackupNow,
  cloudBackupPrepareRestore,
  cloudBackupSetSchedule,
  cloudBackupStatus,
  errorText,
  vaultStatus,
  type CloudBackupInfo,
  type CloudBackupStatusInfo,
} from "../lib/bridge";

function ago(ms: number): string {
  const minutes = Math.max(0, Math.round((Date.now() - ms) / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours} h ago`;
  return `${Math.round(hours / 24)} days ago`;
}

function size(bytes: number): string {
  return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function CloudBackupPanel(): JSX.Element | null {
  const [status, setStatus] = useState<CloudBackupStatusInfo | null>(null);
  const [busy, setBusy] = useState<"uploading" | "listing" | "opening" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [list, setList] = useState<CloudBackupInfo[] | null>(null);
  const [needsCode, setNeedsCode] = useState(false);
  const [code, setCode] = useState("");
  useEffect(() => {
    void cloudBackupStatus().then(setStatus);
    void vaultStatus().then((vault) => setNeedsCode(!vault.has_key));
  }, []);
  if (!status || !status.enabled || !status.code_saved) return null;
  const backUp = async () => {
    setBusy("uploading");
    setError(null);
    try {
      await cloudBackupNow();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(null);
      setStatus(await cloudBackupStatus());
    }
  };
  const openList = async () => {
    setBusy("listing");
    setError(null);
    try {
      setList(await cloudBackupList());
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(null);
    }
  };
  const restore = async (row: CloudBackupInfo) => {
    setBusy("opening");
    setError(null);
    try {
      const plan = await cloudBackupPrepareRestore(row.id, needsCode ? code : null);
      setList(null);
      // The same confirm step as a local restore (Backup & restore).
      window.dispatchEvent(new CustomEvent("aij:restore-plan", { detail: plan }));
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(null);
    }
  };
  const last = status.last;
  return (
    <section
      aria-labelledby="cloud-backup-title"
      className="flex flex-col gap-2 rounded-lg border border-hairline px-3 py-3 text-[11px]"
      data-testid="cloud-backup-panel"
    >
      <h4 id="cloud-backup-title" className="text-xs font-semibold">
        Cloud backup
      </h4>
      <p className="text-ink" aria-live="polite" data-testid="cloud-backup-last">
        {busy === "uploading"
          ? "Backing up… (encrypting on this Mac, then uploading)"
          : last
            ? last.ok
              ? `✓ Last backup ${ago(last.at_ms)}${last.size_bytes ? ` · ${size(last.size_bytes)}` : ""}`
              : `⚠ The last backup didn't finish (${ago(last.at_ms)}): ${last.error ?? "unknown reason"}. Nothing on this Mac was changed.`
            : "No backup yet."}
      </p>
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1.5 text-muted">
          Back up
          <select
            value={status.schedule}
            onChange={(event) => void cloudBackupSetSchedule(event.target.value).then(setStatus)}
            className="rounded-md border border-hairline bg-surface px-1.5 py-0.5 text-[11px] text-ink"
            aria-label="Cloud backup schedule"
            data-testid="cloud-backup-schedule"
          >
            <option value="weekly">every week</option>
            <option value="daily">every day</option>
            <option value="off">only when I ask</option>
          </select>
        </label>
        <span className="flex-1" />
        <button
          type="button"
          disabled={busy !== null}
          onClick={() => void backUp()}
          className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] font-medium text-ink disabled:text-muted"
          data-testid="cloud-backup-now"
        >
          Back up now
        </button>
        <button
          type="button"
          disabled={busy !== null}
          onClick={() => void openList()}
          className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-ink disabled:text-muted"
          data-testid="cloud-backup-restore"
        >
          Restore from Cloud…
        </button>
      </div>
      <p className="text-muted">
        Up to {size(status.limit_bytes)} per backup. Figurehead keeps a limited number, so don't rely on an old one being
        there.
      </p>
      {error && (
        <p role="alert" className="text-ink" data-testid="cloud-backup-error">
          {error}
        </p>
      )}
      {list && (
        <div className="flex flex-col gap-1.5 border-t border-hairline pt-2" data-testid="cloud-backup-list">
          {list.length === 0 ? (
            <p className="text-muted">No backups on Figurehead yet.</p>
          ) : (
            <>
              {needsCode && (
                <label className="flex flex-col gap-1 text-ink">
                  This Mac doesn't have your vault key yet — enter your recovery code
                  <input
                    value={code}
                    onChange={(event) => setCode(event.target.value)}
                    className="rounded border border-hairline bg-canvas px-2 py-1 font-mono text-[11px] text-ink"
                    aria-label="Recovery code"
                    data-testid="cloud-backup-code"
                    autoComplete="off"
                    spellCheck={false}
                  />
                </label>
              )}
              <ul className="flex flex-col gap-1">
                {list.map((row) => (
                  <li key={row.id} className="flex items-center gap-2" data-testid="cloud-backup-row">
                    <span className="min-w-0 flex-1 truncate text-ink">
                      {new Date(row.created_at).toLocaleString([], { dateStyle: "medium", timeStyle: "short" })}
                      <span className="text-muted">
                        {" "}
                        · {size(row.size_bytes)} · {row.this_mac ? "this Mac" : (row.label ?? "another Mac")}
                      </span>
                    </span>
                    <button
                      type="button"
                      disabled={busy !== null || (needsCode && code.trim().length < 20)}
                      onClick={() => void restore(row)}
                      className="rounded border border-hairline px-2 py-0.5 text-[11px] text-ink disabled:text-muted"
                      data-testid="cloud-backup-pick"
                    >
                      {busy === "opening" ? "Opening…" : "Restore…"}
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </section>
  );
}
