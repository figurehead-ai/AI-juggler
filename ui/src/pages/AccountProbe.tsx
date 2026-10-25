/**
 * D-39 (needs the maintainer's ruling; the backend offers it only once
 * ruled): asking Claude Code itself for this profile's limits when the
 * Claude app's meter is old. Off per profile until its owner turns it on,
 * after one sheet that says what runs, what it costs and what leaves the
 * Mac; off again from the same place.
 */
import { useEffect, useState, type JSX } from "react";
import { Button, Card } from "../components/primitives";
import { Modal } from "../components/form";
import { showToast } from "../components/Toast";
import { errorText } from "../lib/bridge";
import { agoWords } from "../lib/quotaWindow";
import { probeRun, probeStatus, probeTurnOff, probeTurnOn, type ProbeStatusInfo } from "../lib/ipc/probe";

const OUTCOME: Record<string, string> = {
  ran: "it answered",
  failed: "Claude Code couldn't check (signed out, or offline)",
  timed_out: "it took too long and was stopped",
  not_started: "Claude Code couldn't be started",
};

export function AccountProbe({ accountId }: { accountId: string }): JSX.Element | null {
  const [status, setStatus] = useState<ProbeStatusInfo | null>(null);
  const [asking, setAsking] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  useEffect(() => void probeStatus(accountId).then(setStatus, () => setStatus(null)), [accountId]);
  if (!status?.offered) return null;

  const act = async (step: () => Promise<ProbeStatusInfo>, done: string) => {
    setBusy(true);
    setProblem(null);
    try {
      setStatus(await step());
      setAsking(false);
      showToast({ message: done });
    } catch (cause) {
      setProblem(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const last =
    status.last_run_ms != null ? `Last asked ${agoWords(status.last_run_ms)}: ${OUTCOME[status.last_outcome ?? ""] ?? "done"}.` : null;

  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="account-probe">
      <p className="text-label uppercase tracking-wider text-muted">Ask Claude Code for its limits</p>
      <p className="text-label text-muted" data-testid="account-probe-words">
        {status.state === "on"
          ? "On: when the Claude app's meter is older than 15 minutes and this profile is in use, AI Juggler runs Claude Code's own usage check here."
          : "When the Claude app isn't open on this profile, its meter goes old. AI Juggler can run Claude Code's own usage check here instead."}
        {last && ` ${last}`}
      </p>
      {problem && (
        <p className="text-meta text-warning" role="alert">
          {problem}
        </p>
      )}
      <div className="flex gap-2">
        {status.state === "on" ? (
          <>
            <Button size="sm" variant="secondary" disabled={busy} onClick={() => void act(() => probeRun(accountId), "Asked Claude Code for its limits.")} data-testid="account-probe-run">
              Measure now
            </Button>
            <Button size="sm" variant="secondary" disabled={busy} onClick={() => void act(() => probeTurnOff(accountId), "Turned off: AI Juggler won't run Claude Code's usage check here.")} data-testid="account-probe-off">
              Turn off
            </Button>
          </>
        ) : (
          <Button size="sm" variant="secondary" onClick={() => setAsking(true)} data-testid="account-probe-open">
            Ask Claude Code…
          </Button>
        )}
      </div>
      {asking && (
        <Modal title="Ask Claude Code for this profile's limits" onClose={() => setAsking(false)} testId="account-probe-sheet">
          <div data-modal-body className="flex flex-col gap-2.5 text-label text-ink">
            <p>AI Juggler runs Claude Code's own usage check here, the same as typing /usage, without opening a window.</p>
            <ul className="flex list-disc flex-col gap-1 pl-4 text-muted">
              <li>It costs no quota: no message goes to Claude.</li>
              <li>Claude Code uses its own sign-in. AI Juggler never sees it, and macOS asks nothing.</li>
              <li>Claude Code asks Anthropic for this account's usage, as /usage does. AI Juggler sends nothing.</li>
              <li>Claude Code saves the answer in its own state file, where AI Juggler reads it. Nothing else changes.</li>
              <li>At most every 15 minutes, only while it's in use and the app's meter is old.</li>
            </ul>
            <p className="text-muted">Turn it off here at any time.</p>
            {problem && (
              <p className="text-meta text-warning" role="alert">
                {problem}
              </p>
            )}
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="secondary" onClick={() => setAsking(false)}>
                Cancel
              </Button>
              <Button size="sm" variant="primary" disabled={busy} onClick={() => void act(() => probeTurnOn(accountId), "Turned on: Claude Code was asked for its limits.")} data-testid="account-probe-on" data-autofocus>
                Turn on
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </Card>
  );
}
