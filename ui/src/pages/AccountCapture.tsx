/**
 * AJ-E1530 Req.1 (ADR-0018 amendment, the maintainer's "Offer it,
 * opt-in"): measuring limits for an account's terminal sessions (`claude`
 * run in a terminal) through its status line. The Claude app never runs a
 * status line (it runs Claude Code headless), so its sessions are measured
 * from the app's own meter instead, with no setup. Off unless the user
 * turns it on, after seeing the exact change; removable from the same
 * place. Shown only for an account used where it is, of a tool that needs
 * it. Below it, D-39's "ask Claude Code", offered only once ruled.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "../components/primitives";
import { Modal } from "../components/form";
import { showToast } from "../components/Toast";
import { errorText } from "../lib/bridge";
import { AccountProbe } from "./AccountProbe";
import {
  captureStatus,
  capturePreview,
  captureTurnOff,
  captureTurnOn,
  type CapturePreviewInfo,
  type CaptureStatusInfo,
} from "../lib/ipc/capture";

/** What turning it off did, in words. */
const OFF_WORDS: Record<string, string> = {
  restored: "Turned off: your settings are back exactly as they were.",
  spliced: "Turned off: AI Juggler's status line is out, and your other changes are kept.",
  theirs: "Turned off: your status line had changed since, so AI Juggler left it as you set it.",
};

/** The profile's opt-in ways to measure its limits. */
export function AccountCapture({ accountId }: { accountId: string }): JSX.Element {
  return (
    <>
      <TerminalCapture accountId={accountId} />
      <AccountProbe accountId={accountId} />
    </>
  );
}

function TerminalCapture({ accountId }: { accountId: string }): JSX.Element | null {
  const [status, setStatus] = useState<CaptureStatusInfo | null>(null);
  const [preview, setPreview] = useState<CapturePreviewInfo | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const load = () => void captureStatus(accountId).then(setStatus, () => setStatus(null));
  useEffect(load, [accountId]);
  if (!status?.offered) return null;

  const open = async () => {
    setProblem(null);
    try {
      setPreview(await capturePreview(accountId));
    } catch (cause) {
      setProblem(errorText(cause));
    }
  };
  const turnOn = async () => {
    if (!preview) return;
    setBusy(true);
    try {
      await captureTurnOn(accountId, preview.fingerprint);
      setPreview(null);
      showToast({ message: "Turned on: sessions you start with claude in a terminal report their limits too." });
      load();
    } catch (cause) {
      setProblem(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const turnOff = async () => {
    setBusy(true);
    try {
      const done = await captureTurnOff(accountId);
      showToast({ message: OFF_WORDS[done.outcome] ?? "Turned off." });
      load();
    } catch (cause) {
      setProblem(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="account-capture">
      <p className="text-section font-semibold text-ink">Limits for terminal sessions</p>
      {status.state === "on" ? (
        <p className="text-label text-ink" data-testid="account-capture-words">
          On: sessions you start with claude in a terminal report their limits to AI Juggler too. Your own status line
          still shows as before. The Claude app's sessions are measured from the app's own meter.
        </p>
      ) : status.state === "changed" ? (
        <p className="text-label text-ink" data-testid="account-capture-words">
          Your status line changed since you turned this on, so terminal sessions aren't measured any more. AI Juggler
          left your change as it is.
        </p>
      ) : (
        <p className="text-label text-muted" data-testid="account-capture-words">
          For sessions you start with claude in a terminal: Claude Code reports their limits only to its status line.
          AI Juggler can add its capture to this profile's status line, with your consent; your own still shows. The
          Claude app's sessions don't need this: they're measured from the app's own meter.
        </p>
      )}
      {problem && (
        <p className="text-meta text-warning" role="alert" data-testid="account-capture-problem">
          {problem}
        </p>
      )}
      <div>
        {status.state === "off" ? (
          <Button size="sm" variant="secondary" onClick={() => void open()} data-testid="account-capture-open">
            Also measure terminal sessions…
          </Button>
        ) : (
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => void turnOff()} data-testid="account-capture-off">
            {status.state === "changed" ? "Forget it" : "Turn off"}
          </Button>
        )}
      </div>
      {preview && (
        <Modal
          title="Measure limits for terminal sessions"
          onClose={() => setPreview(null)}
          wide
          testId="account-capture-preview"
        >
          <div data-modal-body className="flex flex-col gap-3 text-label">
            <p className="text-ink">
              AI Juggler changes one setting in <span className="font-mono">{preview.file}</span>, the status line, so
              Claude also sends its limits to AI Juggler. Your own status line keeps running and looks the same; turn this
              off here to put the file back.
            </p>
            <div className="flex flex-col gap-1" data-testid="account-capture-diff">
              <p className="text-label text-muted">Now</p>
              <pre className="overflow-x-auto whitespace-pre-wrap break-all rounded-lg bg-danger/10 px-3 py-2 font-mono text-label text-ink">
                {preview.before ? `- ${preview.before}` : "(no status line)"}
              </pre>
              <p className="text-label text-muted">After</p>
              <pre className="overflow-x-auto whitespace-pre-wrap break-all rounded-lg bg-success/10 px-3 py-2 font-mono text-label text-ink">
                {`+ ${preview.after}`}
              </pre>
              <p className="text-label text-muted">Nothing else in the file changes.</p>
            </div>
            {problem && (
              <p className="text-meta text-warning" role="alert">
                {problem}
              </p>
            )}
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="secondary" onClick={() => setPreview(null)}>
                Cancel
              </Button>
              <Button size="sm" variant="primary" disabled={busy} onClick={() => void turnOn()} data-testid="account-capture-on" data-autofocus>
                Turn on
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </Card>
  );
}
