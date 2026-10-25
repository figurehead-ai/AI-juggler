/**
 * One session's row in a list (a project's, an account's, History): what
 * it was, its state, its app, project, account and when; it opens its
 * page, and right-click, ⋯ or Shift+F10 open its menu (AJ-E1537). A
 * session nothing named reads as on Home (the review of 0.1.5): the first
 * words of the last thing asked, else its branch or place and when it
 * started, never a column of "Untitled session".
 */
import type { JSX, ReactNode } from "react";
import { AccountAvatar } from "../components/Avatar";
import { AppIcon } from "../components/AppIcon";
import { ProjectMark } from "../components/ProjectMark";
import { useItemMenu } from "../components/ItemMenu";
import { providerDisplayName } from "../lib/providers";
import { STATE_TONE } from "../lib/rail-items";
import { askedKey, untitledName, useAskedWords } from "../lib/session-titles";
import { appOfConversation, appOfSession, type AppRef } from "../lib/app-of";
import type { Row } from "./SessionsList";

/** D-38: the app a row shows: where it is open, else its tool's app. */
export function appOfRow(row: Pick<Row, "item">): AppRef {
  return row.item.kind === "conversation" ? appOfConversation(row.item.conversation) : appOfSession(row.item.session);
}

/** The row's title on screen: its own, else what it was last asked, else
 * its branch or place (left out beside a project column) and when it
 * started. */
function useShownTitle(row: Row, projectShown: boolean): string {
  const item = row.item;
  const session = item.kind === "conversation" ? null : item.session;
  const conversation = item.kind === "conversation" ? item.conversation : null;
  const untitled = session ? !session.label && !session.title : conversation ? !conversation.title : false;
  const profileId = session?.account_id ?? conversation?.account_id ?? "";
  const conversationId = session?.conversation_id ?? conversation?.id ?? null;
  const asked = useAskedWords(untitled && conversationId ? [{ profileId, conversationId }] : []);
  if (!untitled) return row.title;
  const words = conversationId ? asked.get(askedKey(profileId, conversationId)) : null;
  if (words) return words;
  const started = session?.started_at_ms ?? conversation?.started_ms ?? null;
  const cwd = session?.cwd ?? conversation?.cwd ?? "";
  const place = projectShown ? null : cwd.replace(/\/+$/, "").split("/").pop() || null;
  const branch = session?.git_branch ?? conversation?.git_branch ?? null;
  return started !== null ? untitledName(branch, place, started, Date.now()) : row.title;
}

/** One row: it opens its page; right-click, ⋯ or Shift+F10 opens its menu
 * (AJ-E1537), the same menu as everywhere else for what it is. In a
 * short list it's the list's `li`; in a paged one, the content of
 * `PagedList`'s. */
export function SessionRowBody({
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
  const title = useShownTitle(row, project !== null);
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
          <span className="truncate text-body font-medium text-ink" data-testid="page-session-title">{title}</span>
          <span className="truncate text-meta text-muted">
            <span className={row.tone} data-testid="page-session-state">{row.state}</span>
            {row.detail && row.detail !== row.title && !title.startsWith(`${row.detail} · `) && <> · {row.detail}</>}
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
