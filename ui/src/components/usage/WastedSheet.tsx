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

export function WastedSheet({ input, count, projects = new Map(), labels = new Map(), onClose }: { input: UsageQueryInput; count: number; projects?: Map<string, string>; labels?: Map<string, string>; onClose: () => void }): JSX.Element {
  const key = `usage-wasted:${input.edges_ms.join(":")}`;
  const list = usePagedList(key, (request) => usageHistoryWasted(input, request), { pageSize: 50 });
  return (
    <Modal title="Crashed or probably stuck" subtitle={`${count} ${count === 1 ? "session" : "sessions"} in this range, the costliest first`} wide onClose={onClose} testId="usage-wasted">
      <SessionsHeader />
      <PagedList
        list={list}
        keyOf={(row) => `${row.session.profile}:${row.session.id}`}
        renderRow={(row) => (
          <div data-testid="usage-wasted-row" data-why={row.why}>
            <SessionRow row={row.session} projects={projects} labels={labels} total={0} />
            <p className="-mt-1 px-2.5 pb-2 pl-[2.6rem] text-meta text-warning">{row.why === "crashed" ? "Crashed" : "Probably stuck when it was stopped"}</p>
          </div>
        )}
        rowHeight={72}
        label="Sessions that crashed or were probably stuck"
        empty={<p className="py-6 text-center text-meta text-ink-2">Nothing went on crashed or stuck sessions.</p>}
        maxHeight={520}
      />
    </Modal>
  );
}
