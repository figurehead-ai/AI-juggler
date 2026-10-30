/**
 * AJ-E1520 (0.3) — the rows of a Subagents view, a page at a time: a
 * session's (its tab) or a project's (its page), for one query. The
 * rows already shown stay while a changed query (a group opened, a
 * search typed) is read, so nothing blinks; followed every few seconds
 * while something runs or a long session is still being read.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { usePagedList, type PagedList } from "../../lib/paged";
import { subagentsList, subagentsProject, type SubagentRowInfo, type SubagentsInfo, type SubagentsQuery } from "../../lib/ipc/subagents";
import type { PageRequest } from "../../types/generated/PageRequest";

/** How often the rows are read again while something runs. */
export const FOLLOW_MS = 4_000;

/** Whose subagents. */
export type SubagentsSource = { kind: "session"; profileId: string; conversationId: string } | { kind: "project"; projectId: string };

export type SubagentsMeta = Omit<SubagentsInfo, "page">;

export function sourceKey(source: SubagentsSource): string {
  return source.kind === "session" ? `s:${source.profileId}:${source.conversationId}` : `p:${source.projectId}`;
}

export function useSubagentRows(
  source: SubagentsSource | null,
  query: SubagentsQuery,
  visible: boolean,
): { list: PagedList<SubagentRowInfo>; rows: SubagentRowInfo[]; meta: SubagentsMeta | null } {
  const [meta, setMeta] = useState<SubagentsMeta | null>(null);
  const fetchPage = useCallback(
    (page: PageRequest) => {
      if (source === null) return Promise.resolve({ items: [], next_cursor: null, total: 0 });
      const read = source.kind === "session" ? subagentsList(source.profileId, source.conversationId, page, query) : subagentsProject(source.projectId, page, query);
      return read.then(({ page: rows, ...rest }) => {
        setMeta(rest);
        return rows;
      });
    },
    [source, query],
  );
  const key = source ? `subagents:${sourceKey(source)}:${JSON.stringify(query)}` : null;
  const list = usePagedList<SubagentRowInfo>(key, fetchPage, { keyOf: (row) => row.key, pageSize: 100 });
  // The rows last shown stand in while another query is read.
  const shown = useRef<SubagentRowInfo[]>([]);
  if (!list.loading) shown.current = list.items;
  const following = visible && !!meta && (meta.summary.running > 0 || meta.reading);
  const refresh = list.refresh;
  useEffect(() => {
    if (!following) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState !== "hidden") refresh();
    }, FOLLOW_MS);
    return () => window.clearInterval(timer);
  }, [following, refresh]);
  return { list, rows: list.loading ? shown.current : list.items, meta };
}
