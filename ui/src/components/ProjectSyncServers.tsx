/**
 * AJ-E1507: the sync sheet's MCP servers: each account's, and what one
 * could get from another, with the target tool's verdict. Secrets never
 * move.
 */
import { useCallback, useEffect, useState } from "react";
import { errorText, type ProjectInfo } from "../lib/bridge";
import { projectSyncServers, projectSyncServersApply, type ServerCopyInfo, type ServersSyncInfo } from "../lib/ipc/projectSync";
import { DoneLine, SYNC_HEADING, type Done } from "./SyncDoneLine";

export function ServersSection({ project }: { project: ProjectInfo }) {
  const [servers, setServers] = useState<ServersSyncInfo | null>(null);
  const [done, setDone] = useState<Done | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => setServers(await projectSyncServers(project.id)), [project.id]);
  useEffect(() => {
    void load().catch((cause) => setError(errorText(cause)));
  }, [load]);
  const copy = async (offer: ServerCopyInfo) => {
    setBusy(`${offer.offer}|${offer.to}`);
    setError(null);
    setDone(null);
    try {
      const applied = await projectSyncServersApply(project.id, offer);
      // Nothing set up (it's there now, or it went from the source): say
      // why, as a problem, not as a done line.
      if (applied.record === null) setError(applied.message);
      else setDone({ message: applied.message, record: applied.record });
      await load();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(null);
    }
  };
  if (!servers) {
    return error ? (
      <p className="text-xs text-danger" role="alert">
        {error}
      </p>
    ) : null;
  }
  return (
    <section className="flex flex-col gap-2" aria-labelledby="sync-servers-heading" data-testid="project-sync-servers">
      <h3 id="sync-servers-heading" className={SYNC_HEADING}>
        MCP servers
      </h3>
      {servers.accounts.length === 0 ? (
        <p className="text-xs text-muted">No account to compare yet.</p>
      ) : (
        <dl className="grid grid-cols-[minmax(0,auto)_1fr] gap-x-4 gap-y-1 text-xs">
          {servers.accounts.map((account) => (
            <div key={account.account} className="contents">
              <dt className="text-ink">
                {account.label} <span className="text-muted">· {account.tool}</span>
              </dt>
              <dd className="font-mono text-label text-muted">{account.servers.length > 0 ? account.servers.join(", ") : "None"}</dd>
            </div>
          ))}
        </dl>
      )}
      {servers.copies.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {servers.copies.map((offer) => (
            <li key={`${offer.offer}|${offer.to}`} className="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-hairline px-3 py-2 text-xs">
              <span className="text-ink">
                <span className="font-mono">{offer.server}</span> for {offer.to_label}
                <span className="text-muted"> · from {offer.from_label}</span>
              </span>
              {offer.note && <span className="basis-full text-label text-muted">{offer.note}</span>}
              {offer.verdict !== "impossible" && (
                <button
                  type="button"
                  disabled={busy !== null}
                  onClick={() => void copy(offer)}
                  aria-label={`Copy ${offer.server} to ${offer.to_label}`}
                  className="ml-auto rounded border border-hairline px-1.5 py-0.5 text-label hover:bg-surface-2 disabled:opacity-50"
                >
                  {busy === `${offer.offer}|${offer.to}` ? "Copying…" : "Copy"}
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {servers.accounts.length > 1 && servers.copies.length === 0 && (
        <p className="text-xs text-muted">Every account has the same servers.</p>
      )}
      {done && <DoneLine done={done} onUndone={() => void load()} />}
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
