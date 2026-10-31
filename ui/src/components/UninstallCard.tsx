/**
 * Settings › Data: "Before you uninstall". One button puts back what AI
 * Juggler changed outside its own folder (the status lines of accounts
 * measured outside it, AJ-E1530) and lists what's left for the user to
 * remove. Nothing is deleted: the user decides, with the list in hand.
 */
import { useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { errorText } from "../lib/bridge";
import { appPrepareUninstall, type UninstallPrepInfo } from "../lib/ipc/app";
import { CardTitle } from "./CardTitle";

const OUTCOME: Record<string, string> = {
  restored: "its settings are back as they were",
  spliced: "AI Juggler's part is out; your later edits stay",
  theirs: "already your own line; nothing to change",
};

export function UninstallCard(): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<UninstallPrepInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const prepare = async () => {
    setBusy(true);
    setError(null);
    try {
      setDone(await appPrepareUninstall());
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-uninstall">
      <CardTitle>Before you uninstall</CardTitle>
      <p className="text-label leading-relaxed text-muted">
        Puts back what AI Juggler changed outside its own folder, and lists what's left for you to remove. Nothing is deleted.
      </p>
      <div>
        <Button size="sm" variant="secondary" onClick={() => void prepare()} disabled={busy} data-testid="uninstall-prepare">
          {busy ? "Preparing…" : "Prepare to uninstall"}
        </Button>
      </div>
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      {done && (
        <div className="flex flex-col gap-3 text-label leading-relaxed" role="status" data-testid="uninstall-result">
          <section className="flex flex-col gap-1">
            <h4 className="font-medium text-ink">Status lines</h4>
            {done.status_lines.length === 0 ? (
              <p className="text-ink-2">None to put back: no account's status line points at AI Juggler.</p>
            ) : (
              <ul className="flex flex-col gap-0.5">
                {done.status_lines.map((line) => (
                  <li key={line.account} className={line.outcome === "failed" ? "text-danger" : "text-muted"} data-testid="uninstall-status-line">
                    <span className="text-ink">{line.account}:</span>{" "}
                    {line.outcome === "failed" ? `not put back (${line.problem ?? "unknown"}). Turn off measuring on its page.` : OUTCOME[line.outcome]}
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section className="flex flex-col gap-1">
            <h4 className="font-medium text-ink">Left for you to remove</h4>
            <ul className="flex list-disc flex-col gap-1 pl-4 text-muted">
              <li>
                AI Juggler's data folder: <span className="break-all font-mono text-ink">{done.data_folder}</span>. It holds its database, backups, logs and the
                account copies it made. Your own account folders were never moved.
              </li>
              <li>
                In Keychain Access, the items named{" "}
                {done.keychain_services.map((service, at) => (
                  <span key={service}>
                    {at > 0 && " and "}
                    <span className="font-mono text-ink">{service}</span>
                  </span>
                ))}
                , when you used them.
              </li>
              {done.worktrees.filter((tree) => tree.exists).length > 0 && (
                <li>
                  The worktrees it made (with git, from their repository):
                  <ul className="mt-0.5 flex flex-col gap-0.5">
                    {done.worktrees
                      .filter((tree) => tree.exists)
                      .map((tree) => (
                        <li key={tree.path} className="break-all font-mono text-ink">
                          {tree.path} <span className="text-muted">({tree.branch})</span>
                        </li>
                      ))}
                  </ul>
                </li>
              )}
              <li>Then move AI Juggler to the Trash.</li>
            </ul>
          </section>
        </div>
      )}
    </Card>
  );
}
