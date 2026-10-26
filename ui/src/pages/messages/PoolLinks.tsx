/**
 * E18 (AJ-E1805) — which other pools this pool's agents may write to. A
 * message to another pool's member needs a link you made here; taking it
 * back stops it at once. Agents can't link pools.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { messagesLink, messagesLinks } from "../../lib/ipc/messages";
import { usePools } from "../../lib/pools";

export function PoolLinks({ poolId }: { poolId: string }): JSX.Element {
  const pools = usePools();
  const [links, setLinks] = useState<string[]>([]);
  const [adding, setAdding] = useState("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void messagesLinks(poolId).then((next) => alive && setLinks(next), (cause) => alive && setError(errorText(cause)));
    return () => {
      alive = false;
    };
  }, [poolId]);
  const name = (id: string) => pools?.pools.find((pool) => pool.id === id)?.name ?? id;
  const others = (pools?.pools ?? []).filter((pool) => pool.id !== poolId && !links.includes(pool.id));
  const change = (other: string, on: boolean) =>
    void messagesLink(poolId, other, on).then(
      (next) => (setLinks(next), setAdding(""), setError(null)),
      (cause) => setError(errorText(cause)),
    );
  return (
    <div className="flex flex-col gap-2" data-testid="pool-links">
      <div className="flex flex-col gap-0.5">
        <span className="text-[14px] font-medium text-ink">Messages to other pools</span>
        <span className="text-[13px] text-muted">Agents here can write only to this pool, unless you link another one. Agents can't link pools.</span>
      </div>
      {links.length > 0 && (
        <ul className="flex flex-col gap-1">
          {links.map((id) => (
            <li key={id} className="flex items-center gap-2 text-[13px] text-ink">
              <span className="min-w-0 flex-1 truncate">Linked with «{name(id)}»</span>
              <Button size="sm" variant="ghost" onClick={() => change(id, false)}>
                Unlink
                <span className="sr-only"> {name(id)}</span>
              </Button>
            </li>
          ))}
        </ul>
      )}
      {others.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <select
            aria-label="A pool to link"
            value={adding}
            onChange={(event) => setAdding(event.target.value)}
            className="h-8 rounded-lg border border-hairline bg-surface-2 px-2 text-[13px] text-ink outline-none focus:border-accent/60"
          >
            <option value="">Link with another pool…</option>
            {others.map((pool) => (
              <option key={pool.id} value={pool.id}>
                {pool.name}
              </option>
            ))}
          </select>
          <Button size="sm" variant="secondary" disabled={!adding} onClick={() => change(adding, true)} data-testid="pool-link-add">
            Link
          </Button>
        </div>
      )}
      {error && (
        <p role="alert" className="text-[13px] text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
