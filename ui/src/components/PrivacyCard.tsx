/**
 * AJ-E1105 — Settings › Privacy (spec §8): the usage-statistics choice
 * with what actually applies (and why, when an override applies), then
 * everything that was counted and kept, readable: this week's counts by
 * category, every report with its week, state and reason — each opens
 * its exact JSON, read-only, with Copy — the next upload time and the
 * last upload problem in plain words. "Delete local stats" and the
 * public metrics list are always one click away.
 *
 * States: off, local, share, overridden (controls disabled for
 * DO_NOT_TRACK), not included in this build, and an upload error.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { UsageStatsChooser } from "./UsageStatsChooser";
import { CrashReportsSetting } from "./CrashReports";
import {
  errorText,
  usageStatsDeleteLocal,
  usageStatsOpenDocs,
  usageStatsOverview,
  usageStatsReport,
  usageStatsSet,
  usageStatsSetSync,
  type UsageReportRow,
  type UsageStatsMode,
  type UsageStatsOverview,
} from "../lib/bridge";

const CATEGORY_LABEL: Record<string, string> = {
  environment: "Your setup",
  features: "Features used",
  reliability: "Reliability and performance",
};

const STATUS_LABEL: Record<string, string> = {
  pending: "Waiting to send",
  sent: "Sent",
  skipped: "Kept here",
};

function when(ms: number): string {
  return new Date(ms).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}

function asMode(value: string): UsageStatsMode {
  return value === "share" || value === "local" ? value : "off";
}

/** What applies, in one line. */
function stateLine(overview: UsageStatsOverview): string {
  const { info } = overview;
  if (!info.included) return "This build doesn't include usage statistics.";
  if (info.reason) return `${info.reason}.`;
  if (info.effective === "share") return "Sharing: one small report a week. You can read each one here before it goes.";
  if (info.effective === "local") return "Counting on this Mac only. Nothing is sent.";
  return "Off. Nothing is counted, and there is nothing on this Mac.";
}

export function PrivacyCard(): JSX.Element {
  const [overview, setOverview] = useState<UsageStatsOverview | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [viewing, setViewing] = useState<{ row: UsageReportRow; raw: string; json: string } | null>(null);
  const [copied, setCopied] = useState(false);
  const reload = useCallback(async () => {
    try {
      setOverview(await usageStatsOverview());
    } catch (cause) {
      setError(errorText(cause));
    }
  }, []);
  useEffect(() => {
    void reload();
  }, [reload]);
  const choose = async (mode: UsageStatsMode) => {
    setError(null);
    try {
      const result = await usageStatsSet(mode);
      setMessage(result.message);
      setViewing(null);
      await reload();
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const deleteLocal = async () => {
    setError(null);
    try {
      const result = await usageStatsDeleteLocal();
      setMessage(result.message);
      setViewing(null);
      await reload();
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const setSync = async (on: boolean) => {
    setError(null);
    try {
      setOverview(await usageStatsSetSync(on));
      setMessage(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const view = async (row: UsageReportRow) => {
    setCopied(false);
    try {
      const raw = await usageStatsReport(row.status, row.week);
      let json = raw;
      try {
        json = JSON.stringify(JSON.parse(raw), null, 2);
      } catch {
        /* show it exactly as stored */
      }
      setViewing({ row, raw, json });
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  if (!overview) return <Card className="p-5" data-testid="settings-privacy" />;
  const { info } = overview;
  const locked = !info.included || info.reason?.includes("DO_NOT_TRACK") === true;
  const counting = info.included && info.effective !== "off";
  const categories = ["environment", "features", "reliability"].filter((category) =>
    overview.rows.some((row) => row.category === category),
  );
  const nextTry = overview.reports.find((row) => row.status === "pending" && row.next_try_ms)?.next_try_ms;
  return (
    <Card
      className="flex flex-col gap-3 p-5"
      data-testid="settings-privacy"
      data-mode={info.chosen}
      data-effective={info.effective}
    >
      <h3 className="text-sm font-semibold" id="privacy-title">
        Privacy
      </h3>
      <p className="text-[11px] text-ink" data-testid="privacy-state" aria-live="polite">
        {stateLine(overview)}
      </p>
      {info.included && (
        <UsageStatsChooser
          value={info.decided_at_ms === null ? null : asMode(info.chosen)}
          onChange={(mode) => void choose(mode)}
          disabled={locked}
        />
      )}
      {info.included && overview.sync.offered && (
        <div className="flex flex-col gap-1 rounded-lg border border-hairline px-3 py-2" data-testid="privacy-sync">
          <label className="flex items-start gap-2 text-[11px] text-ink">
            <input
              type="checkbox"
              className="mt-0.5"
              checked={overview.sync.on}
              onChange={(event) => void setSync(event.target.checked)}
              data-testid="privacy-sync-toggle"
            />
            <span className="flex flex-col gap-0.5">
              <span className="font-medium">Show my stats in my Figurehead dashboard</span>
              <span className="text-muted">
                Links this Mac's weekly reports to your Figurehead account so you can see them at figurehead.ai. Only
                you can see them, and they're deleted with your account. A synced week is never also sent anonymously.
              </span>
            </span>
          </label>
          {overview.sync.on && overview.sync.email && (
            <p className="pl-5 text-[11px] text-muted" data-testid="privacy-sync-email">
              Synced to {overview.sync.email}
            </p>
          )}
        </div>
      )}
      {overview.sync.notice && (
        <p className="text-[11px] text-ink" role="status" data-testid="privacy-sync-notice">
          {overview.sync.notice}
        </p>
      )}
      {message && (
        <p className="text-[11px] text-ink" role="status" data-testid="privacy-message">
          {message}
        </p>
      )}
      {error && (
        <p className="text-[11px] text-ink" role="alert" data-testid="privacy-error">
          <span aria-hidden="true">⚠ </span>
          {error}
        </p>
      )}
      {counting && overview.last_error && (
        <p className="text-[11px] text-ink" role="alert" data-testid="privacy-upload-error">
          <span aria-hidden="true">⚠ </span>
          {overview.last_error}
          {nextTry ? ` Will retry ${when(nextTry)}.` : ""}
        </p>
      )}
      {counting && (
        <section aria-labelledby="privacy-collected" className="flex flex-col gap-2" data-testid="privacy-collected">
          <h4 id="privacy-collected" className="text-xs font-semibold">
            What's collected
          </h4>
          {info.effective === "share" && overview.next_upload_ms && (
            <p className="text-[11px] text-muted" data-testid="privacy-next-upload">
              Next report may be sent after {when(overview.next_upload_ms)}.
            </p>
          )}
          {categories.length === 0 ? (
            <p className="text-[11px] text-muted">Nothing counted yet this week ({overview.week}).</p>
          ) : (
            categories.map((category) => (
              <div key={category} className="flex flex-col gap-1">
                <p className="text-[11px] font-medium text-ink">
                  {CATEGORY_LABEL[category]} · {overview.week}
                </p>
                <table className="w-full table-fixed text-[11px]" data-testid={`privacy-rows-${category}`}>
                  <tbody>
                    {overview.rows
                      .filter((row) => row.category === category)
                      .map((row) => (
                        <tr key={`${row.metric} ${row.dims}`} className="border-t border-hairline">
                          <td className="w-2/5 truncate py-0.5 pr-2 font-mono text-ink">{row.metric}</td>
                          <td className="truncate py-0.5 pr-2 text-muted" title={row.dims || undefined}>
                            {row.dims || "—"}
                          </td>
                          <td className="w-16 py-0.5 text-right font-mono text-ink">{row.value}</td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
            ))
          )}
          <p className="text-[11px] font-medium text-ink">Reports</p>
          {overview.reports.length === 0 ? (
            <p className="text-[11px] text-muted">No reports yet — the first is made when this week ends.</p>
          ) : (
            <ul className="flex flex-col gap-1" data-testid="privacy-reports">
              {overview.reports.map((row) => (
                <li
                  key={`${row.status}/${row.week}`}
                  className="flex items-center gap-2 text-[11px]"
                  data-testid="privacy-report"
                  data-status={row.status}
                >
                  <span className="w-20 shrink-0 font-mono text-ink">{row.week}</span>
                  <span className="w-28 shrink-0 text-ink">
                    {row.synced ? "Synced to your account" : (STATUS_LABEL[row.status] ?? row.status)}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-muted">
                    {row.reason ??
                      (row.status === "pending" && row.upload_at_ms ? `after ${when(row.upload_at_ms)}` : "")}
                  </span>
                  <button
                    type="button"
                    onClick={() => void view(row)}
                    className="rounded border border-hairline px-2 py-0.5 text-ink focus:outline-none focus:ring-2 focus:ring-accent"
                    data-testid="privacy-report-view"
                  >
                    View
                  </button>
                </li>
              ))}
            </ul>
          )}
          {viewing && (
            <div className="flex flex-col gap-1.5 rounded-lg border border-hairline p-2" data-testid="privacy-report-json">
              <div className="flex items-center gap-2 text-[11px]">
                <span className="flex-1 text-ink">
                  {viewing.row.week} · {STATUS_LABEL[viewing.row.status] ?? viewing.row.status} — the exact report, spaced
                  out to read (Copy gives it byte for byte)
                </span>
                <button
                  type="button"
                  onClick={() =>
                    void navigator.clipboard?.writeText(viewing.raw).then(
                      () => setCopied(true),
                      () => setCopied(false),
                    )
                  }
                  className="rounded border border-hairline px-2 py-0.5 text-ink focus:outline-none focus:ring-2 focus:ring-accent"
                  data-testid="privacy-report-copy"
                >
                  {copied ? "Copied" : "Copy"}
                </button>
                <button
                  type="button"
                  onClick={() => setViewing(null)}
                  className="text-muted underline focus:outline-none focus:ring-2 focus:ring-accent"
                >
                  Close
                </button>
              </div>
              <pre
                className="max-h-64 overflow-auto whitespace-pre rounded bg-canvas p-2 font-mono text-[10.5px] text-ink"
                aria-label="Report JSON (read-only)"
                tabIndex={0}
              >
                {viewing.json}
              </pre>
            </div>
          )}
        </section>
      )}
      <div className="flex flex-wrap gap-2">
        {info.included && (counting || overview.reports.length > 0) && (
          <button
            type="button"
            onClick={() => void deleteLocal()}
            className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-ink focus:outline-none focus:ring-2 focus:ring-accent"
            data-testid="privacy-delete"
          >
            Delete local stats
          </button>
        )}
        <button
          type="button"
          onClick={() => void usageStatsOpenDocs()}
          className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-ink focus:outline-none focus:ring-2 focus:ring-accent"
          data-testid="privacy-open-docs"
        >
          Open metrics documentation
        </button>
      </div>
      <p className="text-[11px] text-muted">
        Reports carry no ID, so one that was already sent can't be found and deleted later. Figurehead keeps raw reports
        for 35 days, then only totals.
      </p>
      <CrashReportsSetting />
    </Card>
  );
}
