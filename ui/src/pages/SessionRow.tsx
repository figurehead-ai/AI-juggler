/**
 * One session's row in a page's list (a project's, an App's, an
 * Account's, History): the one session row (E31, AJ-E3103:
 * `components/row/SessionRow`, its `list` density), so it looks and
 * behaves as it does in the rail and on Home. It opens its page;
 * right-click, ⋯ or Shift+F10 open its menu (AJ-E1537). A session nothing
 * named reads as on Home (the review of 0.1.5): the first words of the
 * last thing asked, else its facts (folder · App · when it started, D-96),
 * never a column of "Untitled session".
 */
import type { JSX, ReactNode } from "react";
import { SessionRow } from "../components/row/SessionRow";
import { askedKey, useAskedWords } from "../lib/session-titles";
import { appOfConversation, appOfSession, type AppRef } from "../lib/app-of";
import type { Row } from "./SessionsList";

/** D-38: the app a row shows: the App the caller says it belongs to,
 * else where it is open, else its tool's app. */
export function appOfRow(row: Pick<Row, "item" | "app">): AppRef {
  if (row.app) return row.app;
  return row.item.kind === "conversation" ? appOfConversation(row.item.conversation) : appOfSession(row.item.session);
}

/** The row's title on screen: its own, else what it was last asked, else
 * its facts (`row.title`: the one short-name rule, D-96 — "billing · Work
 * · started 1:14 AM", the same words as in the rail). */
function useShownTitle(row: Row): string {
  const item = row.item;
  const session = item.kind === "conversation" ? null : item.session;
  const conversation = item.kind === "conversation" ? item.conversation : null;
  const untitled = session ? !session.label && !session.title : conversation ? !conversation.title : false;
  const profileId = session?.account_id ?? conversation?.account_id ?? "";
  const conversationId = session?.conversation_id ?? conversation?.id ?? null;
  const asked = useAskedWords(untitled && conversationId ? [{ profileId, conversationId }] : []);
  if (!untitled) return row.title;
  return (conversationId ? asked.get(askedKey(profileId, conversationId)) : null) || row.title;
}

/** A list's row: `project` is the one it names (`null` on the project's
 * own page); `inApp` on an App's page, whose App it doesn't repeat. */
export function ListRow({
  row,
  project,
  below,
  badge,
  inApp = false,
  stateSaid = false,
  now = Date.now(),
}: {
  row: Row;
  project: { id: string; name: string | null } | null;
  below?: ReactNode;
  badge?: ReactNode;
  inApp?: boolean;
  /** Its section's heading says its state already: no state word. */
  stateSaid?: boolean;
  now?: number;
}): JSX.Element {
  const title = useShownTitle(row);
  const item = { ...row.rail, title: row.title, projectId: project?.id ?? row.rail.projectId, project: project ? (project.name ?? "a moved project") : null };
  return (
    <SessionRow
      item={item}
      density="list"
      now={now}
      onSelect={() => row.open()}
      title={title}
      app={appOfRow(row)}
      menuKind={row.item.kind}
      within={{ app: inApp, project: project === null }}
      trailing={badge}
      stateSlot={stateSaid ? null : undefined}
      below={below}
      data={{ "data-key": row.key }}
    />
  );
}
