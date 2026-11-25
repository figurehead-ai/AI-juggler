/**
 * AJ-E1551 — the conversation switcher's lists, kept by session so a page
 * opened again draws them (and reserves the tree's column) from its first
 * frame, and the column's loading state (AJ-E1552's LoadingState) while
 * they're read: a skeleton at the list's final width.
 */
import { useCallback, useEffect, useState, type JSX, type ReactNode } from "react";
import { LoadingState } from "../../components/LoadingState";
import { conversationsStarted, type StartedSessionInfo } from "../../lib/ipc/conversations";
import { subagentsList, type SubagentInfo } from "../../lib/ipc/subagents";
import { useLoadDeadline } from "../../lib/loading";
import type { SessionModel } from "./model";
import { useHeardSubagentCounts } from "./useSubagents";

/** How many ended subagents the folded list shows. */
const DONE_SHOWN = 8;
/** AJ-E3106 R5: the running subagents listed here; more open the tab. */
const RUNNING_SHOWN = 30;
/** How often the lists are read again while the session runs. */
const EVERY_MS = 15_000;

type Lists = { running: SubagentInfo[]; done: SubagentInfo[]; started: StartedSessionInfo[] };
const NO_LISTS: Lists = { running: [], done: [], started: [] };

/** AJ-E1551: the lists last read, by session, so a page opened again
 * draws them (and reserves their column) from its first frame. */
const listsSeen = new Map<string, Lists>();

/** The session's subagents running now and the latest ended, and the
 * sessions it started; `known` once they were read (now or before). */
export function useSwitcherLists(model: SessionModel, visible: boolean, round: number): Lists & { total: number; runningTotal: number; known: boolean } {
  const counts = useHeardSubagentCounts(model.profileId, model.conversationId);
  const conversationId = model.conversationId;
  const key = conversationId ? `${model.profileId ?? ""}\u0000${conversationId}` : null;
  const [lastRead, setLastRead] = useState<{ key: string | null; lists: Lists | null }>(() => ({ key, lists: key ? (listsSeen.get(key) ?? null) : null }));
  const seen = lastRead.key === key ? lastRead.lists : key ? (listsSeen.get(key) ?? null) : null;
  const lists = seen ?? NO_LISTS;
  const setLists = useCallback(
    (found: Lists) => {
      if (key) listsSeen.set(key, found);
      setLastRead({ key, lists: found });
    },
    [key],
  );
  const total = counts?.total ?? 0;
  useEffect(() => {
    if (!visible || !conversationId) return undefined;
    // A Retry (the loading state's, past its deadline) reads afresh.
    if (round > 0 && key) listsSeen.delete(key);
    let alive = true;
    const agents = (rows: { agent: SubagentInfo | null; context: boolean }[]) => rows.flatMap((row) => (row.agent && !row.context ? [row.agent] : []));
    const read = () => {
      const subagents =
        total > 0
          ? Promise.all([
              subagentsList(model.profileId, conversationId, { cursor: null, limit: RUNNING_SHOWN }, { view: "running", search: null, kind: null, sort: "newest", open: [], closed: [] }),
              subagentsList(model.profileId, conversationId, { cursor: null, limit: DONE_SHOWN }, { view: "done", search: null, kind: null, sort: "newest", open: [], closed: [] }),
            ]).then(([running, done]) => ({ running: agents(running.page.items), done: agents(done.page.items) }))
          : Promise.resolve({ running: [], done: [] });
      void Promise.all([subagents, conversationsStarted(model.tool, conversationId).catch(() => [])]).then(
        ([found, started]) => alive && setLists({ ...found, started }),
        () => undefined,
      );
    };
    read();
    const timer = model.live ? window.setInterval(read, EVERY_MS) : null;
    return () => {
      alive = false;
      if (timer !== null) window.clearInterval(timer);
    };
  }, [visible, conversationId, model.profileId, model.tool, model.live, total, setLists, round, key]);
  return { ...lists, total, runningTotal: counts?.running ?? lists.running.length, known: seen !== null };
}

/** AJ-E1551: the tree's column while its lists are read (AJ-E1552's one
 * loading state): the root and a few branch lines at the list's own width
 * and place, "Still loading…" when slow, Retry past its deadline. */
export function SwitcherLoading({ listed, round, onRetry, children }: { listed: boolean; round: number; onRetry: () => void; children: ReactNode }): JSX.Element {
  const what = "this session's subagents";
  const { slow, timedOut } = useLoadDeadline(!listed, what, { round });
  const skeleton = (
    <output aria-label={`Reading ${what}`} className="flex max-h-40 min-h-0 flex-col gap-2 p-1.5 @xl:max-h-none @xl:p-0 @xl:pr-1" data-testid="switcher-skeleton">
      <span className="h-9 rounded-lg bg-skeleton motion-safe:animate-pulse" />
      {[0, 1, 2].map((n) => (
        <span key={n} className="ml-[25px] h-7 rounded-md bg-skeleton motion-safe:animate-pulse" />
      ))}
    </output>
  );
  return (
    <LoadingState state={listed ? "ready" : timedOut ? "error" : "loading"} slow={slow} what={what} skeleton={skeleton} empty={null} onRetry={onRetry}>
      {children}
    </LoadingState>
  );
}

