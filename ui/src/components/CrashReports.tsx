/**
 * AJ-E1107 — crash reports, behind their own setting (separate from
 * usage statistics): Never (the default), Ask me each time, or Always
 * send. DO_NOT_TRACK forces Never. A report is scrubbed on this Mac
 * before it is kept; with "Ask me each time" the next launch shows it
 * in full and asks. This build has no reporting service yet, so a
 * report the user chooses to send stays queued here — and says so.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "./primitives";
import { crashAnswer, crashInfo, crashSet, errorText, type CrashInfo } from "../lib/bridge";
import { Picker } from "./controls";

const OPTIONS: { value: "never" | "ask" | "always"; label: string }[] = [
  { value: "never", label: "Never" },
  { value: "ask", label: "Ask me each time" },
  { value: "always", label: "Always send" },
];

export function CrashReportsSetting(): JSX.Element | null {
  const [info, setInfo] = useState<CrashInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void crashInfo().then(setInfo);
  }, []);
  if (!info) return null;
  const locked = info.reason !== null;
  return (
    <div className="flex flex-col gap-1.5 border-t border-hairline pt-3" data-testid="privacy-crash-reports">
      <div className="flex flex-wrap items-center gap-2 text-label text-ink">
        <span className="font-medium">Crash reports</span>
        <Picker
          size="sm"
          value={locked ? "never" : info.setting}
          disabled={locked}
          reason={info.reason ?? undefined}
          onChange={(value) => void crashSet(value as "never" | "ask" | "always").then(setInfo, (cause) => setError(errorText(cause)))}
          ariaLabel="Crash reports"
          testId="crash-setting"
          options={OPTIONS}
        />
      </div>
      <p className="text-label text-muted">
        {info.reason
          ? `${info.reason}.`
          : "Separate from usage statistics. A report is scrubbed on this computer first: your home folder becomes ~, and your user name, links and anything that looks like a key are removed."}
      </p>
      {!info.can_send && info.effective !== "never" && (
        <p className="text-label text-muted" data-testid="crash-cannot-send">
          This build can't send crash reports yet{info.queued > 0 ? ` — ${info.queued} kept on this computer until one can` : ""}.
        </p>
      )}
      {error && (
        <p role="alert" className="text-label text-ink">
          {error}
        </p>
      )}
    </div>
  );
}

/** The next-launch question for a report kept under "Ask me each time". */
export function CrashReportPrompt(): JSX.Element | null {
  const [info, setInfo] = useState<CrashInfo | null>(null);
  const sendRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    void crashInfo().then(setInfo);
  }, []);
  const report = info?.effective === "ask" ? info.asking[0] : undefined;
  useEffect(() => {
    if (report) sendRef.current?.focus();
  }, [report?.id]); // eslint-disable-line react-hooks/exhaustive-deps
  if (!info || !report) return null;
  const answer = (choice: "send" | "dont_send" | "always_send") => void crashAnswer(report.id, choice).then(setInfo);
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="crash-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      data-testid="crash-prompt"
      onKeyDown={(event) => {
        if (event.key === "Escape") answer("dont_send");
      }}
    >
      <div className="flex w-full max-w-lg flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-lg">
        <h2 id="crash-title" className="text-sm font-semibold">
          AI Juggler quit unexpectedly. Send a crash report?
        </h2>
        <p className="text-label leading-relaxed text-muted">
          This is the whole report, exactly as it would be sent. It was scrubbed on this computer: no home folder, user name,
          links or keys.
          {!info.can_send && " This build can't send it yet, so Send keeps it on this computer until one can."}
        </p>
        <pre
          className="max-h-64 overflow-auto whitespace-pre rounded bg-canvas p-2 font-mono text-label text-ink"
          aria-label="Crash report (read-only)"
          tabIndex={0}
          data-testid="crash-preview"
        >
          {JSON.stringify(report, null, 2)}
        </pre>
        <div className="flex flex-wrap justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={() => answer("dont_send")} data-testid="crash-dont-send">
            Don't send
          </Button>
          <Button size="sm" variant="secondary" onClick={() => answer("always_send")} data-testid="crash-always-send">
            Always send
          </Button>
          <Button ref={sendRef} size="sm" variant="primary" onClick={() => answer("send")} data-testid="crash-send">
            Send
          </Button>
        </div>
      </div>
    </div>
  );
}
