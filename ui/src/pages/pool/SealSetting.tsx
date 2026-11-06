/**
 * D-62 (AJ-E3005, the P7 walk): seal or unseal a pool after it's made —
 * until now only the New pool wizard could. A sealed pool's sessions are
 * in no other pool, and nothing leaves it through AI Juggler. Each way
 * says first what it changes ("c1 leaves web", "c1 goes back into web"),
 * read from the sessions' own pools, then asks.
 */
import { useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { poolSetSealed, poolsOfSession } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { setPools } from "../../lib/pools";

/** What sealing (or unsealing) `pool` changes, one line per session. */
export async function sealChanges(pool: PoolInfo, members: { id: string; name: string }[]): Promise<string[]> {
  const lines: string[] = [];
  for (const member of members) {
    const info = await poolsOfSession(member.id);
    if (!pool.sealed) {
      const others = info.pools.filter((row) => row.pool_id !== pool.id).map((row) => row.pool_name);
      if (others.length > 0) lines.push(`${member.name} leaves ${others.join(", ")}.`);
    } else {
      const back = info.skipped.filter((row) => row.reason === "sealed_elsewhere" && row.by_pool === pool.id).map((row) => row.pool_name);
      if (back.length > 0) lines.push(`${member.name} goes back into ${back.join(", ")}.`);
    }
  }
  return lines;
}

export function SealSetting({ pool, members }: { pool: PoolInfo; members: { id: string; name: string }[] }): JSX.Element {
  const [changes, setChanges] = useState<string[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const act = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const ask = () => void act(async () => setChanges(await sealChanges(pool, members)));
  const confirm = () =>
    void act(async () => {
      setPools(await poolSetSealed(pool.id, !pool.sealed));
      setChanges(null);
    });
  return (
    <div className="flex flex-col gap-2" data-testid="pool-seal">
      <p className="text-meta text-ink-2">
        {pool.sealed
          ? "Sealed: its sessions are in no other pool, and nothing leaves it through AI Juggler. A switch into or out of it asks first."
          : "Seal it to keep its work apart: its sessions leave every other pool, and nothing leaves it through AI Juggler."}
      </p>
      {changes === null ? (
        <span>
          <Button size="sm" variant="secondary" disabled={busy} onClick={ask} data-testid="pool-seal-open">
            {pool.sealed ? "Unseal…" : "Seal…"}
          </Button>
        </span>
      ) : (
        <div className={"flex flex-col gap-2 rounded-lg border p-2.5 " + (pool.sealed ? "border-hairline bg-surface-2" : "border-warning/40 bg-warning-soft")} role="group" aria-label={pool.sealed ? "Unseal this pool?" : "Seal this pool?"} data-testid="pool-seal-confirm">
          <span className="text-meta font-semibold text-ink">{pool.sealed ? `Unseal «${pool.name}»?` : `Seal «${pool.name}»?`}</span>
          {changes.length > 0 ? (
            <ul className="flex flex-col gap-0.5 text-meta text-ink" data-testid="pool-seal-changes">
              {changes.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          ) : (
            <span className="text-meta text-ink-2">{pool.sealed ? "Nothing comes back: no session was held out of another pool." : "None of its sessions is in another pool."}</span>
          )}
          <span className="flex gap-2">
            <Button size="sm" variant={pool.sealed ? "secondary" : "danger"} disabled={busy} onClick={confirm} data-testid="pool-seal-yes">
              {pool.sealed ? "Unseal" : "Seal it"}
            </Button>
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => setChanges(null)}>
              {pool.sealed ? "Keep it sealed" : "Keep it open"}
            </Button>
          </span>
        </div>
      )}
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
