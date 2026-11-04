/**
 * D-62 (AJ-E3009 req. 4): what a pool's members used this week, as a
 * view only. It reads the usage history's sessions for the last 7 days
 * and keeps this pool's; nothing here ranks, picks or wakes anyone by
 * room (D-31): your own launch and switch sheets suggest by room.
 */
import { useEffect, useState, type JSX } from "react";

import { usageHistorySessions } from "../../lib/ipc/usageHistory";
import { formatTokens } from "../../components/indicator";

const WEEK = 7 * 24 * 60 * 60 * 1000;

export function PoolUsage({ members, project }: { members: { id: string; name: string }[]; project: string | null }): JSX.Element {
  const [used, setUsed] = useState<{ total: number; rows: { name: string; tokens: number }[] } | null>(null);
  const ids = members.map((member) => member.id).join("|");
  useEffect(() => {
    if (!ids) {
      setUsed({ total: 0, rows: [] });
      return;
    }
    let alive = true;
    const now = Date.now();
    const names = new Map(members.map((member) => [member.id, member.name]));
    void usageHistorySessions({ edges_ms: [now - WEEK, now], group_by: "profile", profile: null, project, top: 0 }, { cursor: null, limit: 200 }).then(
      (page) => {
        if (!alive) return;
        const rows = page.items
          .filter((row) => row.session_id !== null && names.has(row.session_id))
          .map((row) => ({ name: names.get(row.session_id!) ?? "A member", tokens: row.io }));
        rows.sort((a, b) => b.tokens - a.tokens);
        setUsed({ total: rows.reduce((sum, row) => sum + row.tokens, 0), rows: rows.slice(0, 5) });
      },
      () => alive && setUsed(null),
    );
    return () => {
      alive = false;
    };
    // `members` is read through `ids` (its names come with them).
  }, [ids, project]);
  if (used === null) return <p className="text-label text-muted">Usage isn't readable now.</p>;
  return (
    <div className="flex flex-col gap-1.5" data-testid="pool-usage">
      <p className="text-meta text-ink">
        <span className="font-semibold">{formatTokens(used.total)}</span> <span className="text-ink-2">in and out this week, by its members</span>
      </p>
      {used.rows.length > 0 && (
        <ul className="flex flex-col gap-0.5">
          {used.rows.map((row) => (
            <li key={row.name} className="flex items-center justify-between gap-2 text-meta">
              <span className="truncate text-ink-2">{row.name}</span>
              <span className="shrink-0 tabular-nums text-ink">{formatTokens(row.tokens)}</span>
            </li>
          ))}
        </ul>
      )}
      <p className="text-label text-muted">A view only: a pool never picks or wakes anyone by how much room they have.</p>
    </div>
  );
}
