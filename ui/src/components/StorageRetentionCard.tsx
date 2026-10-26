/**
 * AJ-E1007 — Settings › Storage & retention: per Account, what AI
 * Juggler holds (Instances, history events, search index) and how big
 * the Provider's transcripts are (measured; AI Juggler never deletes
 * them), plus a retention policy — keep forever by default. A policy
 * never deletes on its own: "Review cleanup…" runs a dry run listing
 * every Instance it would remove and every one it keeps (and why), and
 * only that dry run can be applied.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import {
  accountStorage,
  errorText,
  retentionApply,
  retentionPreview,
  retentionSet,
  type AccountStorageInfo,
  type RetentionPreviewInfo,
} from "../lib/bridge";
import { CardTitle } from "./CardTitle";
import { byteSize } from "../lib/compact";

const CHOICES: Array<{ value: string; label: string }> = [
  { value: "forever", label: "Keep forever" },
  { value: "365", label: "1 year" },
  { value: "180", label: "6 months" },
  { value: "90", label: "90 days" },
  { value: "30", label: "30 days" },
];

export function StorageRetentionCard(): JSX.Element {
  const [rows, setRows] = useState<AccountStorageInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<RetentionPreviewInfo | null>(null);
  const [status, setStatus] = useState("");
  const load = useCallback(() => {
    accountStorage()
      .then(setRows)
      .catch((cause) => setError(errorText(cause)));
  }, []);
  useEffect(load, [load]);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-storage">
      <CardTitle>Storage &amp; retention</CardTitle>
      <p className="text-label text-muted">
        What AI Juggler keeps for each account. Retention only ever removes AI Juggler's own records, and only after you
        review the list. The tools' own transcripts are theirs and stay where they are.
      </p>
      {error && (
        <p role="alert" className="text-label text-ink">
          {error}
        </p>
      )}
      {rows?.length === 0 && <p className="text-label text-muted">No Accounts yet.</p>}
      <ul className="flex flex-col gap-2">
        {rows?.map((row) => (
          <li
            key={row.account_id}
            className="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-hairline px-3 py-2 text-label"
            data-testid="storage-row"
          >
            <span className="min-w-0 flex-1 truncate font-medium text-ink">{row.label}</span>
            <span className="order-last w-full text-muted" data-testid="storage-usage">
              {row.instances} {row.instances === 1 ? "session" : "sessions"} · {row.history_events} history events ·{" "}
              {row.index_lines.toLocaleString()} indexed lines · transcripts {byteSize(row.transcript_bytes)}
            </span>
            <label className="flex items-center gap-1.5 text-muted">
              Keep
              <select
                value={row.retention_days === null ? "forever" : String(row.retention_days)}
                onChange={(event) => {
                  const days = event.target.value === "forever" ? null : Number(event.target.value);
                  void retentionSet(row.account_id, days)
                    .then(load)
                    .catch((cause) => setError(errorText(cause)));
                }}
                className="rounded-md border border-hairline bg-surface px-1.5 py-0.5 text-label text-ink"
                aria-label={`Retention for ${row.label}`}
                data-testid="storage-retention"
              >
                {CHOICES.map((choice) => (
                  <option key={choice.value} value={choice.value}>
                    {choice.label}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              disabled={row.retention_days === null}
              onClick={() =>
                void retentionPreview(row.account_id)
                  .then(setPreview)
                  .catch((cause) => setError(errorText(cause)))
              }
              className="rounded-lg border border-hairline px-2 py-0.5 text-label text-ink disabled:text-muted"
              data-testid="storage-review"
            >
              Review cleanup…
            </button>
          </li>
        ))}
      </ul>
      <p className="sr-only" aria-live="polite">
        {status}
      </p>
      {preview && (
        <RetentionDryRun
          preview={preview}
          label={rows?.find((row) => row.account_id === preview.account_id)?.label ?? ""}
          onClose={(deleted) => {
            setPreview(null);
            if (deleted !== null) {
              setStatus(`Removed AI Juggler's records for ${deleted} ${deleted === 1 ? "session" : "sessions"}.`);
              load();
            }
          }}
        />
      )}
    </Card>
  );
}

function RetentionDryRun({
  preview,
  label,
  onClose,
}: {
  preview: RetentionPreviewInfo;
  label: string;
  onClose: (deleted: number | null) => void;
}): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  const n = preview.deletes.length;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="dry-run-title"
        data-testid="retention-dry-run"
        className="flex max-h-full w-full max-w-lg flex-col gap-3 overflow-y-auto rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="dry-run-title" className="text-sm font-semibold">
          Dry run — {label}, older than {preview.retention_days} days
        </h3>
        {n === 0 ? (
          <p className="text-label text-ink" data-testid="dry-run-nothing">
            Nothing to remove — no ended session is past the policy.
          </p>
        ) : (
          <div className="text-label text-ink">
            <p className="font-medium">
              Would permanently remove AI Juggler's records for {n} {n === 1 ? "session" : "sessions"}:
            </p>
            <ul className="ml-4 list-disc" data-testid="dry-run-deletes">
              {preview.deletes.map((row) => (
                <li key={row.session_id}>
                  {row.project ?? "No project"} · last active {new Date(row.last_active_ms).toLocaleDateString()}{" "}
                  <span className="font-mono text-label text-muted">{row.session_id}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
        {preview.kept.length > 0 && (
          <div className="text-label text-muted">
            <p className="font-medium text-ink">Old enough, but kept:</p>
            <ul className="ml-4 list-disc" data-testid="dry-run-kept">
              {preview.kept.map((row) => (
                <li key={row.session_id}>
                  {row.project ?? "No project"} — {row.kept_because}{" "}
                  <span className="font-mono text-label">{row.session_id}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
        {preview.keeps.length > 0 && <p className="text-label text-muted">Always kept: {preview.keeps.join(" · ")}.</p>}
        {error && (
          <p role="alert" className="text-label text-ink">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button type="button" autoFocus onClick={() => onClose(null)} className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink">
            Cancel
          </button>
          {preview.token && (
            <button
              type="button"
              onClick={() =>
                void retentionApply(preview.account_id, preview.token!)
                  .then((deleted) => onClose(deleted))
                  .catch((cause) => setError(errorText(cause)))
              }
              className="rounded-lg border border-danger/60 px-3 py-1.5 text-xs text-ink hover:border-danger"
              data-testid="dry-run-apply"
            >
              Remove {n} {n === 1 ? "record" : "records"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
