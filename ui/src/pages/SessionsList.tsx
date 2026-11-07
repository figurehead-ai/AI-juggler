/**
 * AJ-E1533 — a project's or an account's sessions on its page: the ones
 * AI Juggler started and the ones it didn't (AJ-E1529), across accounts
 * and tools, live first, then by last activity. Built like Home's rows
 * (what and where, the account, when) and opening their page.
 *
 * 0.1.5 review: a list that can grow reads a page at a time
 * (`PagedSessions`: the backend's `sessions_page`, drawn by `PagedList`);
 * `SessionsList` stays for short lists the UI already holds.
 */
import type { AppRef } from "../lib/app-of";
import { useCallback, useEffect, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { isActiveStatus } from "../components/indicator";
import { conversationItem, sessionItem, type RailItem, type RailState } from "../lib/rail-items";
import { rowState } from "../components/row/SessionRow";
import { sessionTitle } from "../components/ManageWork";
import { conversationTitle, outside, useConversations } from "../lib/conversations";
import { onConversationsChanged } from "../lib/ipc/conversations";
import { BAND_WORDS, bandOf } from "../components/related/order";
import { useHeld } from "../components/related/useHeld";
import { NewRowsBar } from "../components/related/NewRowsBar";
import { useActionHost } from "../components/ItemMenu";
import { ListRow } from "./SessionRow";
import { ago, agoWords, sessionWhen } from "../lib/time-words";
export { dayWords } from "../lib/time-words";
import { hasLiveBackend } from "../lib/bridge";
import { onSessionsUpdated } from "../lib/ipc/instances";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { sessionsCounts, sessionsPage, type SessionRowInfo, type SessionsCountsInfo, type SessionsQuery } from "../lib/ipc/sessions";
import { pageOf, refreshPaged, usePagedList, type PageRequest } from "../lib/paged";
import { PagedList } from "../components/PagedList";
import type { ActionHost } from "../lib/actions";

/** How many rows show before "Show all". */
export const LIST_SHOWN = 30;

/** What a row is, for its ⋯ menu: an AI Juggler session (running, or
 * ended: its history entry) or a conversation AI Juggler didn't start. */
export type RowItem =
  | { kind: "session" | "history"; session: InstanceInfo }
  | { kind: "conversation"; conversation: ConversationInfo };

export type Row = {
  key: string;
  item: RowItem;
  projectId: string | null;
  live: boolean;
  last: number;
  /** When, in the rail's words. */
  when: string;
  title: string;
  detail: string | null;
  /** Its state in the rail's words (AJ-E2104 Req.3), and their colour. */
  state: string;
  railState: RailState;
  tone: string;
  tool: string;
  accountId: string | null;
  accountLabel: string;
  outside: boolean;
  /** A search's words in context, when its transcript matched. */
  snippet?: string | null;
  /** D-62: the App it belongs to, when the caller knows it (a session
   * that ended in the Claude app is the Claude app's, not its tool's). */
  app?: AppRef | null;
  /** E31 (AJ-E3103): what the one session row draws (the rail's item:
   * the rail's own, when the row came from it). */
  rail: RailItem;
  open: () => void;
};

export { appOfRow } from "./SessionRow";

/** One of AI Juggler's sessions as a row. */
export function sessionRow(session: InstanceInfo, now: number, onOpenSession: (id: string) => void): Row {
  const rail = sessionItem(session, () => null, false);
  const said = { ...rowState(rail), state: rail.state };
  return {
    key: `s/${session.id}`,
    item: { kind: isActiveStatus(session.status) ? "session" : "history", session },
    projectId: session.project_id,
    live: isActiveStatus(session.status),
    last: session.last_activity_ms ?? session.started_at_ms,
    when: sessionWhen(session, now),
    title: sessionTitle(session),
    detail: null,
    state: said.word,
    railState: said.state,
    tone: said.text,
    tool: session.provider,
    accountId: session.account_id,
    accountLabel: session.account_label,
    outside: false,
    rail,
    open: () => onOpenSession(session.id),
  };
}

/** A conversation started outside AI Juggler as a row. */
export function conversationRow(conversation: ConversationInfo, now: number, host: ActionHost): Row {
  const rail = conversationItem(conversation, () => null, null);
  const said = rowState(rail);
  return {
    key: `c/${conversation.account_id}/${conversation.id}`,
    item: { kind: "conversation", conversation },
    projectId: conversation.project_id,
    live: conversation.live,
    last: conversation.last_activity_ms ?? conversation.started_ms ?? 0,
    // As the rail says it for an outside one (OutsideConversations).
    when: conversation.last_activity_ms === null ? "" : conversation.live ? ago(conversation.last_activity_ms, now) : agoWords(conversation.last_activity_ms, now),
    title: conversationTitle(conversation),
    detail: conversation.git_branch,
    state: said.word,
    railState: rail.state,
    tone: said.text,
    tool: conversation.tool,
    accountId: conversation.account_id,
    accountLabel: conversation.account_label,
    outside: true,
    rail,
    open: () => host.openConversation?.(conversation),
  };
}

/** A row as the backend's page has it. */
export function infoOfRow(row: Row): SessionRowInfo {
  const item = row.item;
  return { key: row.key, session: item.kind === "conversation" ? null : item.session, conversation: item.kind === "conversation" ? item.conversation : null, snippet: row.snippet ?? null };
}

/** A row of the backend's page. */
export function rowOfInfo(info: SessionRowInfo, now: number, onOpenSession: (id: string) => void, host: ActionHost): Row | null {
  const row = info.session ? sessionRow(info.session, now, onOpenSession) : info.conversation ? conversationRow(info.conversation, now, host) : null;
  return row && { ...row, snippet: info.snippet };
}

/** Every session of a project (or an account), live first. */
export function useSessionRows(
  sessions: InstanceInfo[],
  scope: { projectId?: string; accountId?: string },
  onOpenSession: (id: string) => void,
  now: number,
): Row[] {
  const { all } = useConversations();
  const host = useActionHost();
  const inScope = (projectId: string | null, accountId: string | null) =>
    (scope.projectId === undefined || projectId === scope.projectId) &&
    (scope.accountId === undefined || accountId === scope.accountId);
  const mine: Row[] = sessions.filter((session) => inScope(session.project_id, session.account_id)).map((session) => sessionRow(session, now, onOpenSession));
  const theirs: Row[] = outside(all)
    .filter((conversation) => inScope(conversation.project_id, conversation.account_id))
    .map((conversation) => conversationRow(conversation, now, host));
  return [...mine, ...theirs].sort((a, b) => Number(b.live) - Number(a.live) || b.last - a.last);
}

/** The heading over a list: how many, how many open. */
function ListCount({ total, open }: { total: number; open: number }): JSX.Element {
  return (
    <p className="text-meta text-ink-2" data-testid="page-sessions-count">
      {total === 1 ? "1 session" : `${total} sessions`}
      {open > 0 && ` · ${open} open`}
    </p>
  );
}

function EmptyList({ empty }: { empty: string }): JSX.Element {
  return (
    <p className="rounded-card border border-hairline bg-surface px-4 py-8 text-center text-body text-ink-2" data-testid="page-sessions-empty">
      {empty}
    </p>
  );
}

/** A short list the UI holds: the first 30, then Show all. */
export function SessionsList({
  rows,
  empty,
  projects,
  below,
  badge,
}: {
  rows: Row[];
  empty: string;
  /** Show each row's project (a list across projects: History). */
  projects?: ProjectInfo[];
  /** What a row shows under itself (History's matching words). */
  below?: (row: Row) => ReactNode;
  /** A chip beside its time (History's "Archived"). */
  badge?: (row: Row) => ReactNode;
}): JSX.Element {
  const [all, setAll] = useState(false);
  if (rows.length === 0) return <EmptyList empty={empty} />;
  const shown = all ? rows : rows.slice(0, LIST_SHOWN);
  const live = rows.filter((row) => row.live).length;
  const names = projects ? new Map(projects.map((project) => [project.id, project.name])) : null;
  return (
    <div className="flex flex-col gap-2" data-testid="page-sessions">
      <ListCount total={rows.length} open={live} />
      <ul className="overflow-hidden rounded-xl border border-hairline bg-surface">
        {shown.map((row) => (
          <li key={row.key} className="border-t border-hairline/60 first:border-t-0">
            <ListRow row={row} project={names && row.projectId ? { id: row.projectId, name: names.get(row.projectId) ?? null } : null} below={below?.(row)} badge={badge?.(row)} />
          </li>
        ))}
      </ul>
      {rows.length > shown.length && (
        <button type="button" onClick={() => setAll(true)} className="self-start text-meta text-accent-hi hover:underline" data-testid="page-sessions-all">
          Show all {rows.length}
        </button>
      )}
    </div>
  );
}

/**
 * A list that can grow, a page at a time: History, a project's or an
 * account's Sessions. The backend narrows, searches and orders it
 * (`sessions_page`); in the design preview (no backend), `local` rows
 * are paged the same way.
 *
 * E31 (AJ-E3103): an `ordered` list (every related list) heads its
 * sections (Needs you, Working, Open, Last 7 days, Earlier) once it has
 * more than one, follows the app's changes, and never jumps while read:
 * new rows wait behind "N new" (`components/related/useHeld`).
 */
export function PagedSessions({
  listKey,
  query,
  local,
  empty,
  projects,
  below,
  badge,
  onOpenSession,
  onCounts,
  onTotal,
  groupOf,
  counted = false,
  inApp = false,
  ordered = false,
}: {
  /** D-62: an App's list: its rows don't repeat the App. */
  inApp?: boolean;
  /** Kept for callers; an Account's rows name their App (an Account
   * isn't an App). */
  inAccount?: boolean;
  /** Names the list and everything that shapes it (its memory's key). */
  listKey: string;
  /** A heading for each run of rows that share it (History's days). */
  groupOf?: (row: Row) => string;
  query: SessionsQuery;
  /** The design preview's rows, already narrowed (no backend). */
  local: () => Row[];
  empty: ReactNode;
  projects?: ProjectInfo[];
  below?: (row: Row) => ReactNode;
  badge?: (row: Row) => ReactNode;
  onOpenSession: (id: string) => void;
  /** The list's counts, when read (History's Archived tab). */
  onCounts?: (counts: SessionsCountsInfo) => void;
  /** How many rows the list holds, once its first page is read. */
  onTotal?: (total: number) => void;
  /** Its section's title says how many already: no count line of its own. */
  counted?: boolean;
  /** E31: the related lists' order, sections and hold. */
  ordered?: boolean;
}): JSX.Element {
  const host = useActionHost();
  const live = hasLiveBackend();
  // Pages hold the backend's rows as they come (what's remembered, or
  // prefetched after first paint); they're drawn as rows here.
  // The preview's rows as they are now (a change re-reads them, as the
  // backend's would be).
  const latest = useRef(local);
  latest.current = local;
  const fetchPage = useMemo(
    () => (live ? (page: PageRequest) => sessionsPage(query, page) : async (page: PageRequest) => pageOf(latest.current().map(infoOfRow), page)),
    // The key names the query: a new key is a new fetcher.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [live, listKey],
  );
  const read = usePagedList<SessionRowInfo>(listKey, fetchPage, {
    keyOf: (info) => info.key,
    seed: live ? undefined : (size) => pageOf(local().map(infoOfRow), { cursor: null, limit: size }),
  });
  const rows = useMemo(() => read.items.flatMap((info) => rowOfInfo(info, Date.now(), onOpenSession, host) ?? []), [read.items, onOpenSession, host]);
  useFollow(ordered, listKey);
  // The sections' "now": when the list was opened (a row never changes
  // section just because time passed while it was read).
  const [opened] = useState(() => Date.now());
  const bandOfRow = useCallback((row: Row) => bandOf(row.rail, opened), [opened]);
  const held = useHeld({ ...read, items: rows }, rowKey, bandOfRow, listKey);
  const list = ordered ? held.list : { ...read, items: rows };
  // The heading's counts: the backend's for the whole list.
  const [counts, setCounts] = useState<SessionsCountsInfo | null>(null);
  const scopeKey = listKey;
  useEffect(() => {
    if (!live) return;
    let alive = true;
    void sessionsCounts({ ...query, which: null }).then(
      (next) => {
        if (!alive) return;
        setCounts(next);
        onCounts?.(next);
      },
      () => null,
    );
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [live, scopeKey, read.items.length === 0, read.total]);
  const names = projects ? new Map(projects.map((project) => [project.id, project.name])) : null;
  const total = read.total ?? list.items.length;
  const told = read.loading ? null : total;
  useEffect(() => {
    if (told !== null) onTotal?.(told);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [told]);
  const open = !live ? list.items.filter((row) => row.live).length : query.which === "ended" || query.which === "archived" ? 0 : (counts?.open ?? 0);
  // Sections head the rows once there's more than one.
  const sections = ordered && new Set(list.items.map(held.bandOf)).size > 1 ? (row: Row) => BAND_WORDS[held.bandOf(row)] : undefined;
  const heading = groupOf ?? sections;
  return (
    <div className="flex flex-col gap-2" data-testid="page-sessions">
      {!counted && !read.loading && list.items.length > 0 && <ListCount total={total} open={open} />}
      {ordered && <NewRowsBar waiting={held.waiting} onShow={held.showAll} />}
      <PagedList
        list={list}
        keyOf={rowKey}
        label="Sessions"
        rowHeight={heading ? 66 : 60}
        empty={typeof empty === "string" ? <EmptyList empty={empty} /> : empty}
        testId="page-sessions-list"
        renderRow={(row, index) => {
          const group = heading?.(row) ?? null;
          const before = index > 0 ? list.items[index - 1] : undefined;
          const heads = group !== null && (before === undefined || heading!(before) !== group);
          return (
            <>
              {heads && (
                <h3 className="border-b border-hairline/60 bg-surface-2/50 px-4 pb-1.5 pt-2.5 text-meta font-semibold text-ink" data-testid="page-sessions-group">
                  {group}
                </h3>
              )}
              <ListRow
                row={row}
                project={names && row.projectId ? { id: row.projectId, name: names.get(row.projectId) ?? null } : null}
                below={below?.(row)}
                badge={badge?.(row)}
                inApp={inApp}
                // Under a heading that says its state ("Working"), the row
                // doesn't say it again (the 0.4 review, 45).
                stateSaid={group !== null && group.toLowerCase() === row.state.toLowerCase()}
              />
            </>
          );
        }}
      />
    </div>
  );
}

const rowKey = (row: Row) => row.key;

/** A shown related list reads itself again when sessions or the tools'
 * conversations change (once a burst settles); what it shows holds still
 * (`useHeld`). */
function useFollow(on: boolean, listKey: string): void {
  useEffect(() => {
    if (!on) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const again = () => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => refreshPaged(listKey), FOLLOW_SETTLE_MS);
    };
    const stops = [onSessionsUpdated(again), onConversationsChanged(again)];
    return () => {
      if (timer) clearTimeout(timer);
      stops.forEach((stop) => stop());
    };
  }, [on, listKey]);
}

/** As the relation counts settle a burst of changes (lib/relation-counts). */
const FOLLOW_SETTLE_MS = 250;
