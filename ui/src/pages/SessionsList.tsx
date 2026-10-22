/**
 * AJ-E1533 — a project's or an account's sessions on its page: the ones
 * AI Juggler started and the ones it didn't (AJ-E1529), across accounts
 * and tools, live first, then by last activity. Built like Home's rows
 * (what and where, the account, when) and opening their page.
 */
import { AccountAvatar } from "../components/Avatar";
import { useState, type JSX, type ReactNode } from "react";
import { STATE_WORDS, isActiveStatus } from "../components/indicator";
import { ToolMark } from "../components/ToolMark";
import { ProjectMark } from "../components/ProjectMark";
import { sessionTitle } from "../components/ManageWork";
import { conversationTitle, outside, useConversations } from "../lib/conversations";
import { useActionHost, useItemMenu } from "../components/ItemMenu";
import { ago, agoWords, sessionWhen } from "../lib/time-words";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import type { ConversationInfo } from "../lib/ipc/conversations";

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
  state: string;
  tool: string;
  accountId: string | null;
  accountLabel: string;
  outside: boolean;
  open: () => void;
};

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
  const mine: Row[] = sessions
    .filter((session) => inScope(session.project_id, session.account_id))
    .map((session) => ({
      key: `s/${session.id}`,
      item: { kind: isActiveStatus(session.status) ? ("session" as const) : ("history" as const), session },
      projectId: session.project_id,
      live: isActiveStatus(session.status),
      last: session.last_activity_ms ?? session.started_at_ms,
      when: sessionWhen(session, now),
      title: sessionTitle(session),
      detail: null,
      state: STATE_WORDS[session.status] ?? session.status,
      tool: session.provider,
      accountId: session.account_id,
      accountLabel: session.account_label,
      outside: false,
      open: () => onOpenSession(session.id),
    }));
  const theirs: Row[] = outside(all)
    .filter((conversation) => inScope(conversation.project_id, conversation.account_id))
    .map((conversation) => ({
      key: `c/${conversation.account_id}/${conversation.id}`,
      item: { kind: "conversation" as const, conversation },
      projectId: conversation.project_id,
      live: conversation.live,
      last: conversation.last_activity_ms ?? conversation.started_ms ?? 0,
      // As the rail says it for an outside one (OutsideConversations).
      when: conversation.last_activity_ms === null ? "" : conversation.live ? ago(conversation.last_activity_ms, now) : agoWords(conversation.last_activity_ms, now),
      title: conversationTitle(conversation),
      detail: conversation.git_branch,
      state: conversation.live ? "Running" : "Ended",
      tool: conversation.tool,
      accountId: conversation.account_id,
      accountLabel: conversation.account_label,
      outside: true,
      open: () => host.openConversation?.(conversation),
    }));
  return [...mine, ...theirs].sort((a, b) => Number(b.live) - Number(a.live) || b.last - a.last);
}

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
  if (rows.length === 0) {
    return (
      <p className="rounded-xl border border-hairline bg-surface px-4 py-6 text-center text-xs text-muted" data-testid="page-sessions-empty">
        {empty}
      </p>
    );
  }
  const shown = all ? rows : rows.slice(0, LIST_SHOWN);
  const live = rows.filter((row) => row.live).length;
  const names = projects ? new Map(projects.map((project) => [project.id, project.name])) : null;
  return (
    <div className="flex flex-col gap-2" data-testid="page-sessions">
      <p className="text-[11px] text-muted" data-testid="page-sessions-count">
        {rows.length === 1 ? "1 session" : `${rows.length} sessions`}
        {live > 0 && ` · ${live} running`}
      </p>
      <ul className="overflow-hidden rounded-xl border border-hairline bg-surface">
        {shown.map((row) => (
          <SessionListRow
            key={row.key}
            row={row}
            project={names && row.projectId ? { id: row.projectId, name: names.get(row.projectId) ?? null } : null}
            below={below?.(row)}
            badge={badge?.(row)}
          />
        ))}
      </ul>
      {rows.length > shown.length && (
        <button type="button" onClick={() => setAll(true)} className="self-start text-xs text-accent-hi hover:underline" data-testid="page-sessions-all">
          Show all {rows.length}
        </button>
      )}
    </div>
  );
}

/** One row: it opens its page; right-click, ⋯ or Shift+F10 opens its menu
 * (AJ-E1537), the same menu as everywhere else for what it is. */
function SessionListRow({
  row,
  project,
  below,
  badge,
}: {
  row: Row;
  project: { id: string; name: string | null } | null;
  below?: ReactNode;
  badge?: ReactNode;
}): JSX.Element {
  const menu = useRowMenu(row);
  return (
    <li className="group relative border-t border-hairline/60 first:border-t-0" data-testid="page-session-item" data-key={row.key} {...menu.triggerProps}>
      <button
        type="button"
        data-testid="page-session-row"
        data-outside={row.outside ? "true" : undefined}
        data-live={row.live ? "true" : undefined}
        className="flex w-full items-center gap-3.5 px-4 py-2.5 pr-10 text-left hover:bg-surface-2"
        onClick={row.open}
      >
        <span
          aria-hidden="true"
          className={"size-2 shrink-0 rounded-full " + (row.live ? "bg-live shadow-[0_0_0_4px_var(--color-live-soft)]" : "bg-hairline")}
        />
        <ToolMark tool={row.tool} size={14} className="shrink-0 text-muted" />
        <span className="min-w-0 flex-1 truncate text-sm text-ink">
          {row.title}
          {row.detail && <span className="text-faint"> · {row.detail}</span>}
          <span className="text-faint">
            {" "}
            · {row.state}
            {row.outside && " · outside AI Juggler"}
          </span>
        </span>
        {project && (
          <span className="flex min-w-0 max-w-[30%] shrink items-center gap-1.5 text-xs text-muted" data-testid="page-session-project">
            <ProjectMark projectId={project.id} size={16} />
            <span className="truncate">{project.name ?? "a moved project"}</span>
          </span>
        )}
        <span className="flex shrink-0 items-center gap-1.5 text-xs text-muted">
          <AccountAvatar accountId={row.accountId} label={row.accountLabel} size={16} />
          <span aria-hidden="true" className="hidden sm:inline">{row.accountLabel}</span>
          <span className="sr-only">on {row.accountLabel}</span>
        </span>
        {badge}
        <span className="w-20 shrink-0 whitespace-nowrap text-right font-mono text-xs text-faint">{row.when}</span>
      </button>
      {below}
      {menu.button({ label: `Actions for ${row.title}`, className: "absolute right-2 top-2" })}
      {menu.element}
    </li>
  );
}

/** A row's menu, for what it is (its kind decides its items). */
function useRowMenu(row: Row) {
  const item = row.item;
  return useItemMenu(item.kind, (item.kind === "conversation" ? item.conversation : item.session) as never, row.title);
}
