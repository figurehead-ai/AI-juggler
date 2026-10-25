/**
 * AJ-E1508: an account's "Copy settings from…": the usual settings of
 * another account of the same tool (its MCP servers, permissions, status
 * line, plugins), for an account brought in as a copy or any account
 * later. Each is ticked when it can be set up; what the account has of
 * its own is never replaced. Credentials, history and caches never come
 * with it. Every set-up can be undone here or in Settings.
 */
import { useEffect, useState, type JSX } from "react";
import { Modal } from "./form";
import { Button } from "./primitives";
import { SwitchSetupList, SwitchSetupResults } from "./SwitchSetupList";
import { accountsList, errorText, type AccountInfo } from "../lib/bridge";
import { accountSettingsApply, accountSettingsCheck } from "../lib/ipc/accountSettings";
import type { AppliedInfo, SetupCheckInfo } from "../lib/ipc/switchSetup";

export function AccountSettingsSheet({
  account,
  onClose,
}: {
  account: AccountInfo;
  onClose: (copied: number) => void;
}): JSX.Element {
  const [sources, setSources] = useState<AccountInfo[] | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const [check, setCheck] = useState<SetupCheckInfo | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [results, setResults] = useState<AppliedInfo[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const source = chosen ?? sources?.[0]?.id ?? null;

  useEffect(() => {
    accountsList().then(
      (rows) => setSources(rows.filter((row) => row.provider === account.provider && row.id !== account.id)),
      (cause) => setError(errorText(cause)),
    );
  }, [account.id, account.provider]);

  useEffect(() => {
    if (!source) return;
    let alive = true;
    setCheck(null);
    setPicked(new Set());
    setError(null);
    accountSettingsCheck(account.id, source).then(
      (next) => {
        if (!alive) return;
        setCheck(next);
        setPicked(new Set(next.offers.filter((offer) => offer.suggested).map((offer) => offer.id)));
      },
      (cause) => alive && setError(errorText(cause)),
    );
    return () => {
      alive = false;
    };
  }, [account.id, source]);

  const toggle = (id: string) =>
    setPicked((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const apply = async () => {
    if (!source) return;
    setBusy(true);
    setError(null);
    try {
      setResults(await accountSettingsApply(account.id, source, [...picked]));
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const copied = results?.filter((result) => result.record !== null).length ?? 0;
  const name = sources?.find((row) => row.id === source)?.label ?? "";

  return (
    <Modal title={`Copy settings into ${account.label}`} onClose={() => onClose(copied)} wide testId="account-settings">
      <div className="flex max-h-[70vh] flex-col gap-4 overflow-y-auto pr-1 text-sm">
        {sources !== null && sources.length === 0 && (
          <p className="text-muted">There's no other account of this tool to copy from.</p>
        )}
        {sources !== null && sources.length > 0 && !results && (
          <>
            <label className="flex flex-wrap items-center gap-2">
              <span>From</span>
              {sources.length > 1 ? (
                <select
                  value={source ?? ""}
                  onChange={(event) => setChosen(event.target.value)}
                  aria-label="Account to copy the settings from"
                  className="rounded-lg border border-hairline bg-surface px-2 py-1 text-sm text-ink"
                >
                  {sources.map((row) => (
                    <option key={row.id} value={row.id}>
                      {row.label}
                    </option>
                  ))}
                </select>
              ) : (
                <span className="font-medium text-ink">{name}</span>
              )}
            </label>
            {check === null && !error && <p className="text-xs text-muted">Reading what {name} has…</p>}
            {check && check.offers.length === 0 && (
              <p className="text-xs text-muted">
                {account.label} already has everything {name} has to copy: its MCP servers, permissions and status line.
              </p>
            )}
            {/* Named by this account, whatever the check calls it. */}
            {check && check.offers.length > 0 && (
              <SwitchSetupList check={{ ...check, to: account.label }} picked={picked} onToggle={toggle} />
            )}
            <p className="text-label text-muted">Never its sign-in, history or caches, and nothing {account.label} has of its own is replaced.</p>
          </>
        )}
        {results && <SwitchSetupResults target={account.label} results={results} />}
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          {results ? (
            <Button size="sm" variant="primary" onClick={() => onClose(copied)} data-autofocus>
              Done
            </Button>
          ) : sources !== null && sources.length === 0 ? (
            <Button size="sm" variant="primary" onClick={() => onClose(0)}>
              Close
            </Button>
          ) : (
            <>
              <Button size="sm" variant="ghost" onClick={() => onClose(0)}>
                Cancel
              </Button>
              <Button
                size="sm"
                variant="primary"
                disabled={busy || picked.size === 0 || check === null}
                onClick={() => void apply()}
                data-testid="account-settings-apply"
              >
                {busy ? "Copying…" : picked.size === 1 ? "Copy 1 setting" : `Copy ${picked.size} settings`}
              </Button>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
