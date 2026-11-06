/**
 * AJ-E2902: the sessions behind "Crashed or probably stuck" — each with
 * why (stuck is inferred, so it says "probably", D-36), the costliest
 * first, a page at a time, each opening its page.
 */
import type { JSX } from "react";
import { Modal } from "../form";
import { PagedList } from "../PagedList";
import { usePagedList } from "../../lib/paged";
import { usageHistoryWasted, type UsageQueryInput } from "../../lib/ipc/usageHistory";
import { SessionRow, SessionsHeader } from "./TopLists";

/** The state's column, beside the sessions' own. */
const STATE = "w-28 shrink-0 px-2.5";

export function WastedSheet({ input, count, projects = new Map(), labels = new Map(), onClose }: { input: UsageQueryInput; count: number; projects?: Map<string, string>; labels?: Map<string, string>; onClose: () => void }): JSX.Element {
  const key = `usage-wasted:${input.edges_ms.join(":")}`;
  const list = usePagedList(key, (request) => usageHistoryWasted(input, request), { pageSize: 50 });
  return (
    <Modal title="Crashed or probably stuck" subtitle={`${count} ${count === 1 ? "session" : "sessions"} in this range, the costliest first`} wide onClose={onClose} testId="usage-wasted">
      {/* Plain rows under their headings, what happened in a column of
          its own (the 0.4 review, 37). */}
      <div className="flex">
        <div className="min-w-0 flex-1">
          <SessionsHeader />
        </div>
        <span aria-hidden="true" className={`${STATE} flex items-end border-b border-hairline pb-2 text-meta font-semibold text-muted`}>
          State
        </span>
      </div>
      <PagedList
        list={list}
        keyOf={(row) => `${row.session.profile}:${row.session.id}`}
        renderRow={(row) => (
          <div className="flex items-center" data-testid="usage-wasted-row" data-why={row.why}>
            <div className="min-w-0 flex-1">
              <SessionRow row={row.session} projects={projects} labels={labels} total={0} />
            </div>
            <span className={`${STATE} text-meta text-warning`} title={row.why === "crashed" ? undefined : "Probably stuck when it was stopped"} data-testid="usage-wasted-state">
              {row.why === "crashed" ? "Crashed" : "Probably stuck"}
            </span>
          </div>
        )}
        rowHeight={56}
        plain
        label="Sessions that crashed or were probably stuck"
        empty={<p className="py-6 text-center text-meta text-ink-2">Nothing went on crashed or stuck sessions.</p>}
        maxHeight={520}
      />
    </Modal>
  );
}
