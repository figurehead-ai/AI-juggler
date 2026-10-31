/**
 * AJ-E1517 — Settings › General, under About & updates: Report a bug (the
 * GitHub form) and Export diagnostics. The diagnostics are shown first,
 * file by file, exactly as they'll be saved — scrubbed on this Mac, with
 * the settings left out named — and only then saved where the user picks.
 */
import { useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { Status } from "./SettingsSections";
import { errorText } from "../lib/bridge";
import {
  appDiagnosticsExport,
  appDiagnosticsPreview,
  appReportBugOpen,
  pickDiagnosticsDestination,
  type DiagnosticsPreviewInfo,
} from "../lib/ipc/app";
import { CardTitle } from "./CardTitle";
import { byteSize } from "../lib/compact";
import { Disclosure } from "./controls";
import { commandTimings, slowCalls, SLOW_MS, type CommandTiming, type SlowCall } from "../lib/ipc/timing";

const bytesOf = (text: string) => new TextEncoder().encode(text).length;

export function DiagnosticsCard(): JSX.Element {
  const [preview, setPreview] = useState<DiagnosticsPreviewInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const run = async (step: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await step();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const show = () =>
    run(async () => {
      setStatus(null);
      setPreview(await appDiagnosticsPreview());
    });
  const save = (shown: DiagnosticsPreviewInfo) =>
    run(async () => {
      const dest = await pickDiagnosticsDestination(shown.file_name);
      if (!dest) return;
      const saved = await appDiagnosticsExport(shown.id, dest);
      setPreview(null);
      setStatus(`Saved to ${saved.path} (${byteSize(saved.bytes)}). Attach it to your report.`);
    });
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-diagnostics">
      <div>
        <CardTitle>Report a problem</CardTitle>
        <p className="mt-0.5 text-xs leading-relaxed text-ink-2">
          Tell us what went wrong on GitHub. Diagnostics help: versions, settings and AI Juggler's own logs, scrubbed on
          this computer. No transcripts, terminal output, credentials, home folder or user name. You see every file before
          it's saved, and attach the zip yourself.
        </p>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="secondary" onClick={() => void run(appReportBugOpen)} data-testid="report-bug">
          Report a bug
        </Button>
        <Button size="sm" variant="secondary" onClick={() => void show()} disabled={busy} data-testid="diagnostics-show">
          Export diagnostics…
        </Button>
      </div>
      {preview && (
        <section
          className="flex flex-col gap-2 rounded-lg border border-hairline p-3"
          aria-labelledby="diagnostics-title"
          data-testid="diagnostics-preview"
          onKeyDown={(event) => {
            if (event.key === "Escape") setPreview(null);
          }}
        >
          <h4 id="diagnostics-title" className="text-xs font-semibold">
            What the zip holds
          </h4>
          <ul className="flex flex-col gap-1">
            {preview.files.map((file) => (
              <li key={file.name} data-testid="diagnostics-file">
                <details>
                  <summary className="cursor-pointer text-xs">
                    <span className="font-mono text-ink">{file.name}</span>
                    <span className="text-muted"> · {file.about} · {byteSize(bytesOf(file.text))}</span>
                  </summary>
                  <pre
                    className="mt-1 max-h-56 overflow-auto whitespace-pre-wrap break-all rounded bg-canvas p-2 font-mono text-label text-ink"
                    aria-label={`${file.name} (read-only)`}
                    tabIndex={0}
                  >
                    {file.text}
                  </pre>
                </details>
              </li>
            ))}
          </ul>
          {preview.left_out.length > 0 && (
            <p className="text-label leading-relaxed text-muted" data-testid="diagnostics-left-out">
              Left out: {preview.left_out.length} {preview.left_out.length === 1 ? "setting" : "settings"} that hold
              tokens, addresses or paths ({preview.left_out.join(", ")}).
            </p>
          )}
          <div className="flex flex-wrap justify-end gap-2">
            <Button size="sm" variant="secondary" onClick={() => setPreview(null)} data-testid="diagnostics-cancel">
              Cancel
            </Button>
            <Button size="sm" variant="primary" onClick={() => void save(preview)} disabled={busy} data-testid="diagnostics-save">
              Save…
            </Button>
          </div>
        </section>
      )}
      <Status status={status} error={error} />
      <ResponseTimes />
    </Card>
  );
}

/** 0.3.2: how long the backend took, this run (`lib/ipc/timing`). Read
 * when opened, so it costs nothing while closed. */
function ResponseTimes(): JSX.Element {
  const [rows, setRows] = useState<CommandTiming[]>([]);
  const [slow, setSlow] = useState<SlowCall[]>([]);
  const read = () => (setRows(commandTimings().slice(0, 12)), setSlow(slowCalls().slice(0, 8)));
  return (
    <Disclosure summary="Backend response times" testId="diagnostics-timings" onToggle={(open) => open && read()}>
      <div className="mt-2 flex flex-col gap-2 text-label">
        <p className="text-ink-2">
          Since AI Juggler started: each backend call&apos;s slowest and latest time. Calls over {SLOW_MS} ms are listed below. Names and times only.
        </p>
        {rows.length === 0 ? (
          <p className="text-ink-2">No calls yet.</p>
        ) : (
          <table className="w-full text-left tabular-nums" data-testid="diagnostics-timings-table">
            <thead className="text-muted">
              <tr>
                <th className="py-1 font-medium">Call</th>
                <th className="py-1 text-right font-medium">Calls</th>
                <th className="py-1 text-right font-medium">Slowest</th>
                <th className="py-1 text-right font-medium">Latest</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.command} className="border-t border-hairline">
                  <td className="py-1 font-mono text-ink">{row.command}</td>
                  <td className="py-1 text-right text-ink">{row.calls}</td>
                  <td className={"py-1 text-right " + (row.slowestMs >= SLOW_MS ? "text-warning" : "text-ink")}>{ms(row.slowestMs)}</td>
                  <td className="py-1 text-right text-ink">{ms(row.lastMs)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {slow.length > 0 && (
          <p className="text-ink-2" data-testid="diagnostics-slow">
            Slow: {slow.map((call) => `${call.command} ${ms(call.ms)}`).join(" · ")}
          </p>
        )}
        <Button size="sm" variant="ghost" className="w-fit" onClick={read}>
          Refresh
        </Button>
      </div>
    </Disclosure>
  );
}

function ms(value: number): string {
  return value >= 1000 ? `${(value / 1000).toFixed(1)} s` : `${Math.round(value)} ms`;
}
