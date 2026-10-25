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
import { providerDisplayName } from "../lib/providers";
import { AccountAvatar } from "../components/Avatar";
import { useEffect, useMemo, useState, type JSX, type ReactNode } from "react";
import { isActiveStatus } from "../components/indicator";
import { STATE_TONE, conversationItem, sessionState, stateWord, type RailState } from "../lib/rail-items";
import { AppIcon } from "../components/AppIcon";
import { appOfConversation, appOfSession, type AppRef } from "../lib/app-of";
import { ProjectMark } from "../components/ProjectMark";
import { sessionTitle } from "../components/ManageWork";
import { conversationTitle, outside, useConversations } from "../lib/conversations";
import { useActionHost, useItemMenu } from "../components/ItemMenu";
import { ago, agoWords, sessionWhen } from "../lib/time-words";
export { dayWords } from "../lib/time-words";
import { hasLiveBackend, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { sessionsCounts, sessionsPage, type SessionRowInfo, type SessionsCountsInfo, type SessionsQuery } from "../lib/ipc/sessions";
import { pageOf, usePagedList, type PageRequest } from "../lib/paged";
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
  open: () => void;
};

/** D-38: the app a row shows: where it is open, else its tool's app. */
export function appOfRow(row: Pick<Row, "item">): AppRef {
  return row.item.kind === "conversation" ? appOfConversation(row.item.conversation) : appOfSession(row.item.session);
}

/** One of AI Juggler's sessions as a row. */
export function sessionRow(session: InstanceInfo, now: number, onOpenSession: (id: string) => void): Row {
  const said = sessionState(session);
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
    tone: said.tone,
    tool: session.provider,
    accountId: session.account_id,
    accountLabel: session.account_label,
    outside: false,
    open: () => onOpenSession(session.id),
  };
}

/** A conversation started outside AI Juggler as a row. */
export function conversationRow(conversation: ConversationInfo, now: number, host: ActionHost): Row {
  const said = conversationItem(conversation, () => null, null);
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
    state: stateWord(said.state, said.probably),
    railState: said.state,
    tone: STATE_TONE[said.state].text,
    tool: conversation.tool,
    accountId: conversation.account_id,
    accountLabel: conversation.account_label,
    outside: true,
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
    <p className="text-meta text-muted" data-testid="page-sessions-count">
      {total === 1 ? "1 session" : `${total} sessions`}
      {open > 0 && ` · ${open} open`}
    </p>
  );
}

function EmptyList({ empty }: { empty: string }): JSX.Element {
  return (
    <p className="rounded-card border border-hairline bg-surface px-4 py-8 text-center text-body text-muted" data-testid="page-sessions-empty">
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
          <SessionRowBody
            key={row.key}
            as="li"
            row={row}
            project={names && row.projectId ? { id: row.projectId, name: names.get(row.projectId) ?? null } : null}
            below={below?.(row)}
            badge={badge?.(row)}
          />
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
}: {
  /** Names the list and everything that shapes it (its memory's key). */
  listKey: string;
  /** A heading for each run of rows that share it (History's days). */
  groupOf?: (row: Row) => string;
  query: SessionsQuery;
  /** The design preview's rows, already narrowed (no backend). */
  local: () => Row[];
  empty: string;
  projects?: ProjectInfo[];
  below?: (row: Row) => ReactNode;
  badge?: (row: Row) => ReactNode;
  onOpenSession: (id: string) => void;
  /** The list's counts, when read (History's Archived tab). */
  onCounts?: (counts: SessionsCountsInfo) => void;
  /** How many rows the list holds, once its first page is read. */
  onTotal?: (total: number) => void;
}): JSX.Element {
  const host = useActionHost();
  const live = hasLiveBackend();
  // Pages hold the backend's rows as they come (what's remembered, or
  // prefetched after first paint); they're drawn as rows here.
  const fetchPage = useMemo(
    () => (live ? (page: PageRequest) => sessionsPage(query, page) : async (page: PageRequest) => pageOf(local().map(infoOfRow), page)),
    // The key names the query: a new key is a new fetcher.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [live, listKey],
  );
  const read = usePagedList<SessionRowInfo>(listKey, fetchPage, {
    keyOf: (info) => info.key,
    seed: live ? undefined : (size) => pageOf(local().map(infoOfRow), { cursor: null, limit: size }),
  });
  const rows = useMemo(() => read.items.flatMap((info) => rowOfInfo(info, Date.now(), onOpenSession, host) ?? []), [read.items, onOpenSession, host]);
  const list = { ...read, items: rows };
  // The heading's counts: the backend's for the whole list.
  const [counts, setCounts] = useState<SessionsCountsInfo | null>(null);
  const scopeKey = `${query.project_id ?? ""}/${query.account_id ?? ""}/${query.text ?? ""}`;
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
  }, [live, scopeKey, list.items.length === 0]);
  const names = projects ? new Map(projects.map((project) => [project.id, project.name])) : null;
  const total = list.total ?? list.items.length;
  const told = list.loading ? null : total;
  useEffect(() => {
    if (told !== null) onTotal?.(told);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [told]);
  const open = !live ? list.items.filter((row) => row.live).length : query.which === "ended" || query.which === "archived" ? 0 : (counts?.open ?? 0);
  return (
    <div className="flex flex-col gap-2" data-testid="page-sessions">
      {!list.loading && list.items.length > 0 && <ListCount total={total} open={open} />}
      <PagedList
        list={list}
        keyOf={(row) => row.key}
        label="Sessions"
        rowHeight={groupOf ? 64 : 58}
        empty={<EmptyList empty={empty} />}
        testId="page-sessions-list"
        renderRow={(row, index) => {
          const group = groupOf?.(row) ?? null;
          const before = index > 0 ? list.items[index - 1] : undefined;
          const heads = group !== null && (before === undefined || groupOf!(before) !== group);
          return (
            <>
              {heads && (
                <h3 className="border-b border-hairline/60 bg-surface-2/50 px-4 pb-1.5 pt-2.5 text-meta font-semibold text-ink" data-testid="page-sessions-group">
                  {group}
                </h3>
              )}
              <SessionRowBody
                as="div"
                row={row}
                project={names && row.projectId ? { id: row.projectId, name: names.get(row.projectId) ?? null } : null}
                below={below?.(row)}
                badge={badge?.(row)}
              />
            </>
          );
        }}
      />
    </div>
  );
}

/** One row: it opens its page; right-click, ⋯ or Shift+F10 opens its menu
 * (AJ-E1537), the same menu as everywhere else for what it is. In a
 * short list it's the list's `li`; in a paged one, the content of
 * `PagedList`'s. */
function SessionRowBody({
  row,
  project,
  below,
  badge,
  as,
}: {
  row: Row;
  project: { id: string; name: string | null } | null;
  below?: ReactNode;
  badge?: ReactNode;
  as: "li" | "div";
}): JSX.Element {
  const menu = useRowMenu(row);
  const Box = as;
  return (
    <Box
      className={"group relative " + (as === "li" ? "border-t border-hairline/60 first:border-t-0" : "")}
      data-testid="page-session-item"
      data-key={row.key}
      {...menu.triggerProps}
    >
      <button
        type="button"
        data-testid="page-session-row"
        data-outside={row.outside ? "true" : undefined}
        data-live={row.live ? "true" : undefined}
        className="flex w-full items-center gap-3.5 px-4 py-2.5 pr-11 text-left hover:bg-surface-2"
        onClick={row.open}
      >
        <span className="relative flex shrink-0">
          <AppIcon {...appOfRow(row)} size={24} />
          <span
            aria-hidden="true"
            data-state-dot={row.railState}
            className={"absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-surface " + STATE_TONE[row.railState].dot}
          />
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="truncate text-body font-medium text-ink">{row.title}</span>
          <span className="truncate text-meta text-muted">
            <span className={row.tone} data-testid="page-session-state">{row.state}</span>
            {row.detail && row.detail !== row.title && <> · {row.detail}</>}
            {row.outside && ` · in ${appOfRow(row).name}`}
          </span>
        </span>
        {project && (
          <span className="hidden w-44 min-w-0 shrink-0 items-center gap-1.5 text-meta text-muted md:flex" data-testid="page-session-project">
            <ProjectMark projectId={project.id} size={16} />
            <span className="truncate">{project.name ?? "a moved project"}</span>
          </span>
        )}
        <span className="flex w-36 min-w-0 shrink-0 items-center gap-1.5 text-meta text-muted">
          <AccountAvatar accountId={row.accountId} label={row.accountLabel} size={16} />
          <span aria-hidden="true" className="truncate">{row.accountLabel}</span>
          <span className="sr-only">{providerDisplayName(row.tool)} on {row.accountLabel}</span>
        </span>
        {badge}
        <span className="w-24 shrink-0 whitespace-nowrap text-right text-meta tabular-nums text-muted">{row.when}</span>
      </button>
      {below}
      {menu.button({ label: `Actions for ${row.title}`, className: "absolute right-2 top-1/2 -translate-y-1/2" })}
      {menu.element}
    </Box>
  );
}

/** A row's menu, for what it is (its kind decides its items). */
function useRowMenu(row: Row) {
  const item = row.item;
  return useItemMenu(item.kind, (item.kind === "conversation" ? item.conversation : item.session) as never, row.title);
}
