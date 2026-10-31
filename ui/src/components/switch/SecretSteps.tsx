/**
 * AJ-E1543 (never a bare status): something that can be set up but needs
 * a value the user holds — an MCP server's token. AI Juggler never copies
 * a secret, so the row says the switch doesn't wait for it, that ticking it
 * sets it up without the value, and how to give the account its own copy:
 * the target tool's steps and command (from its adapter, a placeholder in
 * place of the value), a Copy button and Check again.
 */
import { useState, type JSX } from "react";
import type { GetItInfo } from "../../lib/ipc/switchSetup";
import { errorText } from "../../lib/bridge";

const BUTTON = "min-h-7 rounded-lg border border-hairline px-3 text-meta text-ink hover:bg-surface-2";

export function SecretSteps({
  getIt,
  name,
  to,
  onCheckAgain,
}: {
  getIt: GetItInfo;
  /** What it's for ("github"), for the words and the controls' names. */
  name: string;
  /** The account it's for. */
  to: string;
  /** Reads the account again; resolves to what it found, in words. */
  onCheckAgain?: () => Promise<string>;
}): JSX.Element {
  const [copied, setCopied] = useState<"yes" | "no" | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const command = getIt.command;
  const copy = async () => {
    if (!command) return;
    try {
      await navigator.clipboard.writeText(command);
      setCopied("yes");
    } catch {
      setCopied("no");
    }
  };
  const checkAgain = async () => {
    if (!onCheckAgain) return;
    setChecking(true);
    try {
      setStatus(await onCheckAgain());
    } catch (cause) {
      setStatus(errorText(cause));
    } finally {
      setChecking(false);
    }
  };
  return (
    <div className="flex flex-col gap-1.5" data-testid="switch-setup-secret-steps">
      <p className="text-body text-ink">
        Tick it to set {name} up on {to} without the value, and give {to} its own copy:
      </p>
      <ol className="flex list-decimal flex-col gap-0.5 pl-5 text-meta text-ink">
        {getIt.steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
      {command && (
        <pre className="whitespace-pre-wrap break-words rounded-lg border border-hairline bg-surface-2 px-3 py-2 font-mono text-meta text-ink" data-testid="switch-setup-command">
          {command}
        </pre>
      )}
      <div className="flex flex-wrap items-center gap-2">
        {command && (
          <button type="button" onClick={() => void copy()} className={BUTTON} aria-label={`Copy the command for ${name}`} data-testid="switch-setup-copy">
            Copy
          </button>
        )}
        {onCheckAgain && (
          <button type="button" onClick={() => void checkAgain()} disabled={checking} className={BUTTON} aria-label={`Check again for ${name}`} data-testid="switch-setup-recheck">
            {checking ? "Checking…" : "Check again"}
          </button>
        )}
        {copied && (
          <span role="status" className="text-meta text-ink">
            {copied === "yes" ? "Copied. Put your own value in place of the <your …> part before you press Return." : "Couldn't copy it; select the command and copy it yourself."}
          </span>
        )}
      </div>
      {status && (
        <p className="text-meta text-ink" role="status">
          {status}
        </p>
      )}
    </div>
  );
}
