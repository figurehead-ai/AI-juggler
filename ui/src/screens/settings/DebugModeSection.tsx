/**
 * AJ-E2912 (D-75): Settings › Advanced › Debug mode. Off by default; on,
 * AI Juggler writes down every step of each switch, launch, hand-over and
 * pool change (sizes, timings, the tools' own errors, never what's in a
 * conversation) to a
 * bounded log on this computer, for a bug report. It turns itself off
 * after seven days; clearing the log is one click.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Disclosure } from "../../components/controls";
import { errorText } from "../../lib/bridge";
import { byteSize } from "../../lib/compact";
import { debugLogClear, debugLogShow, debugModeGet, debugModeSet, type DebugModeInfo } from "../../lib/ipc/report";
import { PaneSection, SettingRow, SwitchRow } from "./parts";

const until = (ms: number) => new Date(ms).toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });

/** What the switch row says. */
export function debugWords(info: DebugModeInfo): string {
  if (info.on && info.until_ms) return `On until ${until(info.until_ms)}, then off by itself.`;
  return "Writes down each step of every switch, launch, hand-over and pool change, and each report an agent sends: sizes, timings and the tools' own errors. Never what's in a conversation.";
}

export function DebugModeSection(): JSX.Element {
  const [info, setInfo] = useState<DebugModeInfo | null>(null);
  const [log, setLog] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    debugModeGet().then(setInfo, (cause: unknown) => setError(errorText(cause)));
  }, []);
  // `done`: a note when the row's own words don't already say it (on,
  // they say until when: rc26 X2b read "…then off by itself. On for
  // seven days.").
  const act = (step: () => Promise<DebugModeInfo>, done: string | null) => {
    setError(null);
    step().then(
      (next) => {
        setInfo(next);
        setNote(done);
        setLog(null);
      },
      (cause: unknown) => setError(errorText(cause)),
    );
  };
  const show = () => debugLogShow().then((text) => setLog(text || "The log is empty."), (cause: unknown) => setError(errorText(cause)));
  return (
    <PaneSection
      id="debug-mode"
      title="Debug mode"
      footer="On this computer only, readable by you only. It goes with a bug report only when you choose to send it."
    >
      {info === null ? (
        <SettingRow title="Keep a detailed log" description={error ?? "Reading…"} testId="debug-loading" />
      ) : (
        <>
          <SwitchRow
            title="Keep a detailed log"
            description={debugWords(info)}
            checked={info.on}
            onChange={(on) => act(() => debugModeSet(on), on ? null : "Off. The log stays until you clear it.")}
            testId="debug-mode"
            note={error ?? note}
            noteTone={error ? "warning" : "info"}
          />
          <SettingRow
            title="The debug log"
            description={
              info.lines > 0
                ? `${info.lines.toLocaleString()} ${info.lines === 1 ? "step" : "steps"}, ${byteSize(info.bytes)}. At most 16 MB: the oldest steps go first.`
                : "Empty."
            }
            testId="debug-log-row"
            control={
              <>
                <Button size="sm" variant="secondary" disabled={info.lines === 0} onClick={() => void show()} data-testid="debug-log-show">
                  Show
                </Button>
                <Button size="sm" variant="secondary" disabled={info.lines === 0} onClick={() => act(debugLogClear, "Cleared.")} data-testid="debug-log-clear">
                  Clear the log
                </Button>
              </>
            }
          />
          {log !== null && (
            <li className="px-4 py-3" data-testid="debug-log-text">
              <Disclosure summary="The log's newest steps" open>
                <pre
                  className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap break-all rounded bg-canvas p-2 font-mono text-label text-ink"
                  tabIndex={0}
                  aria-label="The debug log (read-only)"
                >
                  {log}
                </pre>
              </Disclosure>
            </li>
          )}
        </>
      )}
    </PaneSection>
  );
}
