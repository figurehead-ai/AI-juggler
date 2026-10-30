/**
 * The canvas as a list (View as list): the same members and interactions
 * for the keyboard and screen readers, and for a pool too big to draw.
 * Its members are the context cards (a page at a time, AJ-E1703); then
 * who talks to whom, each with what waits on it; then the asks no member
 * on the canvas made.
 */
import type { JSX } from "react";

import { PoolCardRows } from "../../components/PoolCards";
import type { InstanceInfo } from "../../lib/bridge";
import type { PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import { agoWords } from "../../lib/time-words";
import { PendingChip, edgeWords } from "./CanvasEdges";
import type { Selection } from "./Canvas";
import type { CanvasData } from "./useCanvas";
import { edgeKey } from "./useCanvas";

export function MemberList({ data, canvas, sessions, names, now, onSelect }: { data: CanvasData; canvas: PoolCanvasInfo | null; sessions: InstanceInfo[]; names: ReadonlyMap<string, string>; now: number; onSelect: (selection: Selection) => void }): JSX.Element {
  const edges = canvas?.edges ?? [];
  return (
    <div className="h-full min-h-0 overflow-y-auto p-5" data-canvas-scroll data-testid="pool-list">
      <div className="mx-auto flex max-w-[960px] flex-col gap-6">
        <section aria-labelledby="pool-list-members" className="flex flex-col gap-2">
          <h3 id="pool-list-members" className="text-section text-ink">
            Members
          </h3>
          <div className="rounded-card border border-hairline bg-surface">
            <PoolCardRows list={data.list} sessions={sessions} now={now} />
          </div>
        </section>
        <section aria-labelledby="pool-list-edges" className="flex flex-col gap-2">
          <h3 id="pool-list-edges" className="text-section text-ink">
            Who talks to whom <span className="text-meta font-normal text-muted">this week</span>
          </h3>
          {edges.length === 0 ? (
            <p className="text-meta text-muted">No messages or task hand-offs between the members yet.</p>
          ) : (
            <ul className="flex flex-col divide-y divide-hairline/60 rounded-card border border-hairline bg-surface" data-testid="pool-list-edges">
              {edges.map((edge) => {
                const key = edgeKey(edge.from_session_id, edge.to_session_id);
                const from = names.get(edge.from_session_id) ?? "A member";
                const to = names.get(edge.to_session_id) ?? "a member";
                return (
                  <li key={key} className="flex flex-wrap items-center gap-3 px-4 py-2.5">
                    <button type="button" onClick={() => onSelect({ kind: "edge", key })} className="flex min-w-0 flex-1 flex-col text-left outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
                      <span className="truncate text-body text-ink">
                        {from} → {to}
                      </span>
                      <span className="text-meta text-muted">
                        {edgeWords(edge, from, to).count} · last {agoWords(edge.last_at_ms, now)}
                      </span>
                    </button>
                    {edge.pending.length > 0 && <PendingChip edge={edge} />}
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
