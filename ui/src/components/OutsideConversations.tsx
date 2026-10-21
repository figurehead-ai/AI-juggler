/**
 * AJ-E1529 — conversations AI Juggler didn't start, in the rail under
 * their project: built like a session row (RailRow), with a quiet
 * "outside AI Juggler" mark, and read-only (D-08). The live ones always
 * show; then the most recent few, the rest one click away on the
 * project's page, so the rail stays within its budget at 50+.
 */
import type { JSX } from "react";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { AccountDisc, hueOf, stateChip } from "./indicator";
import { conversationTitle, liveOutside, outside, useConversations } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { Card } from "./primitives";
import type { ChipFilters, RowGroup } from "../lib/railview";
import { ago } from "../lib/time-words";

/** Recent outside conversations shown per project, besides the live ones. */
export const RECENT_SHOWN = 3;


/** Which outside conversations the rail's filters let through: all of
 * them unfiltered, the live ones under "Running", the ended ones under
 * "Ended", none under any other filter (they have no attention state,
 * and the tool/account/project/search filters are the sessions'). */
export type OutsideScope = "all" | "live" | "ended" | "none";

export function outsideScope(filters: ChipFilters, query: string): OutsideScope {
  if (query.trim() !== "" || filters.provider !== null || filters.account !== null || filters.project !== null) return "none";
  if (filters.chips.length === 0) return "all";
  if (filters.chips.length === 1 && filters.chips[0] === "running") return "live";
  if (filters.chips.length === 1 && filters.chips[0] === "ended") return "ended";
  return "none";
}

/** What the rail shows of one project's outside conversations: every
 * live one, then the newest few; and how many more there are. */
export function railSlice(all: ConversationInfo[], projectId: string, scope: OutsideScope = "all"): { shown: ConversationInfo[]; more: number } {
  if (scope === "none") return { shown: [], more: 0 };
  const mine = outside(all).filter((conversation) => conversation.project_id === projectId);
  const live = scope === "ended" ? [] : mine.filter((conversation) => conversation.live);
  const recent = scope === "live" ? [] : mine.filter((conversation) => !conversation.live).slice(0, RECENT_SHOWN);
  const more = scope === "all" ? mine.length - live.length - recent.length : 0;
  return { shown: [...live, ...recent], more };
}

/** The rail's project groups, plus one for each project where a session
 * started outside AI Juggler is running now (and none of AI Juggler's):
 * a live session always shows under its project. */
export function withOutsideGroups(groups: RowGroup[], all: ConversationInfo[], projects: ProjectInfo[], scope: OutsideScope = "all"): RowGroup[] {
  if (scope === "none" || scope === "ended") return groups;
  const have = new Set(groups.map((group) => group.id));
  const live = new Set(outside(all).filter((conversation) => conversation.live).map((conversation) => conversation.project_id));
  const extra = projects
    .filter((project) => !have.has(project.id) && live.has(project.id))
    .map((project) => ({ id: project.id, label: project.name, rollup: "", rows: [] }));
  return [...groups, ...extra];
}

export function ConversationRow({
  conversation,
  selected,
  density,
  now,
  onOpen,
}: {
  conversation: ConversationInfo;
  selected: boolean;
  density: "comfortable" | "compact";
  now: number;
  onOpen: (conversation: ConversationInfo) => void;
}): JSX.Element {
  const title = conversationTitle(conversation);
  const chip = stateChip(conversation.live ? "running" : "finished");
  const Icon = chip.Icon;
  const when = conversation.last_activity_ms === null ? "" : conversation.live ? ago(conversation.last_activity_ms, now) : `${ago(conversation.last_activity_ms, now)} ago`;
  const state = conversation.live ? "Running" : "Ended";
  const menu = useItemMenu("conversation", conversation, title);
  return (
    <div className="group relative" {...menu.triggerProps}>
      <button
        type="button"
        data-conversation-id={conversation.id}
        data-testid="rail-outside"
        aria-current={selected ? "true" : undefined}
        onClick={() => onOpen(conversation)}
        className={
          "relative flex w-full min-w-0 items-center gap-2.5 rounded-lg pl-2.5 pr-2 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (density === "compact" ? "py-1.5 " : "py-2 ") +
          (selected ? "bg-surface" : "hover:bg-surface/60")
        }
      >
        {selected && <span aria-hidden="true" className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />}
        <span aria-hidden="true" className={"flex w-4 shrink-0 justify-center " + (conversation.live ? "text-accent-hi" : "text-faint")}>
          <Icon width={13} height={13} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-baseline gap-1.5">
            <span className={"min-w-0 truncate text-[12.5px] font-medium " + (conversation.live ? "text-ink" : "text-muted")}>{title}</span>
            {density === "compact" && when && <span className="ml-auto shrink-0 font-mono text-[10px] text-muted">{when}</span>}
          </span>
          {density === "comfortable" && (
            <span className="flex min-w-0 items-center gap-1 overflow-hidden text-[11px] text-muted">
              <span className="shrink-0 whitespace-nowrap">{state}</span>
              <span className="shrink-0 whitespace-nowrap" data-testid="outside-mark">
                · outside AI Juggler
              </span>
              {when && <span className="min-w-0 truncate">· {when}</span>}
            </span>
          )}
          <span className="sr-only">
            , {state}, outside AI Juggler, on {conversation.account_label}
          </span>
        </span>
        <AccountDisc accountId={conversation.account_id} label={conversation.account_label} size={16} />
      </button>
      {menu.button({ label: `Conversation actions for ${title}`, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}

/** One project's outside conversations, as the rail shows them. */
export function OutsideRows({
  projectId,
  selectedId,
  density,
  now,
  scope,
}: {
  projectId: string;
  selectedId: string | null;
  density: "comfortable" | "compact";
  now: number;
  scope: OutsideScope;
}): JSX.Element | null {
  const { all } = useConversations();
  const host = useActionHost();
  const onOpen = (conversation: ConversationInfo) => host.openConversation?.(conversation);
  const onMore = host.openProjectById ? () => host.openProjectById!(projectId) : undefined;
  const { shown, more } = railSlice(all, projectId, scope);
  if (shown.length === 0) return null;
  return (
    <>
      {shown.map((conversation) => (
        <ConversationRow
          key={`${conversation.account_id}/${conversation.id}`}
          conversation={conversation}
          selected={selectedId === conversation.id}
          density={density}
          now={now}
          onOpen={onOpen}
        />
      ))}
      {more > 0 && onMore && (
        <button
          type="button"
          onClick={onMore}
          data-testid="rail-outside-more"
          className="self-start rounded-md py-1 pl-7 pr-2 text-[11px] text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          {more} more outside AI Juggler
        </button>
      )}
    </>
  );
}

/** An account's running sessions: AI Juggler's and, AJ-E1529, those
 * started outside it (from its own conversations). */
export function AccountRunningCard({
  accountId,
  running,
  now,
  onOpenSession,
}: {
  accountId: string;
  running: InstanceInfo[];
  now: number;
  onOpenSession: (id: string) => void;
}): JSX.Element {
  const { all } = useConversations();
  const host = useActionHost();
  const elsewhere = liveOutside(all, accountId);
  const count = running.length + elsewhere.length;
  return (
    <Card className="flex flex-col gap-1.5 p-4" data-testid="account-running">
      <p className="text-[11px] uppercase tracking-wider text-muted">Running sessions ({count})</p>
      {count === 0 && <p className="text-[11.5px] text-muted">None right now.</p>}
      {running.map((row) => (
        <button key={row.id} type="button" onClick={() => onOpenSession(row.id)} className="flex items-center gap-2 py-1 text-left text-xs">
          <span className="min-w-0 flex-1 truncate font-medium">{row.label ?? row.git_branch ?? row.account_label}</span>
          <span className="shrink-0 text-[11px] text-muted">{stateChip(row.status).label}</span>
        </button>
      ))}
      {elsewhere.map((conversation) => (
        <ConversationRow
          key={conversation.id}
          conversation={conversation}
          selected={false}
          density="compact"
          now={now}
          onOpen={(c) => host.openConversation?.(c)}
        />
      ))}
    </Card>
  );
}

/** A project's conversations started outside AI Juggler, live first:
 * every one (the rail shows only the newest few). */
export function ProjectConversationsCard({ projectId, now }: { projectId: string; now: number }): JSX.Element | null {
  const { all } = useConversations();
  const host = useActionHost();
  const mine = outside(all).filter((conversation) => conversation.project_id === projectId);
  if (mine.length === 0) return null;
  const ordered = [...mine.filter((c) => c.live), ...mine.filter((c) => !c.live)];
  return (
    <Card className="flex flex-col gap-1 p-4" data-testid="project-outside">
      <p className="text-[11px] uppercase tracking-wider text-muted">
        Conversations outside AI Juggler ({mine.length})
      </p>
      {ordered.map((conversation) => (
        <ConversationRow
          key={`${conversation.account_id}/${conversation.id}`}
          conversation={conversation}
          selected={false}
          density="comfortable"
          now={now}
          onOpen={(c) => host.openConversation?.(c)}
        />
      ))}
    </Card>
  );
}

/** Home's Running list, the rows of sessions started outside AI Juggler:
 * built like Home's own (the live dot, what and where, the account, how
 * long), opening their page. */
export function HomeOutsideRows({ rows, now }: { rows: ConversationInfo[]; now: number }): JSX.Element {
  const host = useActionHost();
  return (
    <>
      {rows.map((conversation) => (
        <li key={`${conversation.account_id}/${conversation.id}`} data-home-item className="border-t border-hairline/60 first:border-t-0">
          <button
            type="button"
            data-home-row
            data-testid="home-outside"
            className="flex w-full items-center gap-3.5 px-4 py-2.5 text-left hover:bg-surface-2"
            onClick={() => host.openConversation?.(conversation)}
          >
            <span aria-hidden="true" className="size-2 shrink-0 rounded-full bg-live shadow-[0_0_0_4px_var(--color-live-soft)]" />
            <span className="min-w-0 flex-1 truncate text-sm text-ink">
              {conversationTitle(conversation)}
              <span className="text-faint"> · outside AI Juggler</span>
            </span>
            <span className="flex items-center gap-1.5 text-xs text-muted">
              <span aria-hidden="true" className="size-2 shrink-0 rounded-full" style={{ background: `hsl(${hueOf(conversation.account_id)} 70% 66%)` }} />
              {conversation.account_label}
            </span>
            {/* Its last activity: a resumed one can be months old. */}
            <span className="w-14 text-right font-mono text-xs text-faint" title="Last activity">
              {conversation.last_activity_ms === null ? "" : ago(conversation.last_activity_ms, now)}
            </span>
          </button>
        </li>
      ))}
    </>
  );
}
