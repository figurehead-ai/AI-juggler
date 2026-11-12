/**
 * AJ-E1529 — conversations AI Juggler didn't start, in the rail under
 * their project: built like a session row (RailRow), with a quiet
 * "outside AI Juggler" mark, and read-only (D-08). AJ-E2103: only the
 * open ones show under their project (Open now); one that asks you leads
 * the rail (Needs you), and ended, inactive and background ones wait
 * behind the rail's "Also:" line. The Ended filter still lists the newest
 * few per project, the rest on the project's page.
 */
import { providerDisplayName } from "../lib/providers";
import { ProjectMark } from "./ProjectMark";
import { AppIcon } from "./AppIcon";
import { appOfConversation } from "../lib/app-of";
import type { JSX } from "react";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { stateChip } from "./indicator";
import { conversationTitle, outside } from "../lib/conversations";
import { matches, parseQuery } from "../lib/rail-view";
import type { AccountOf } from "../lib/rail-places";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { ChipFilters, RowGroup } from "../lib/railview";
import { ago, agoWords } from "../lib/time-words";
import { STATE_TONE, conversationAppKey, conversationItem, goToWhere, stateWord as railStateWord } from "../lib/rail-items";
import { ProfileBadge } from "./ProfileBadge";

/** Ended outside conversations shown per project under the Ended filter. */
export const RECENT_SHOWN = 3;

/** AJ-E2103: an open one whose app says it waits on you, or finished
 * something to look at (the app's own word, D-37). */
export function asksYou(conversation: ConversationInfo): boolean {
  return conversation.live && (conversation.state === "asks_you" || conversation.state === "done_review");
}

/** Open now under its project: open, interactive, not asking. */
export function openNow(conversation: ConversationInfo): boolean {
  return conversation.live && !conversation.background && !asksYou(conversation);
}

/** E21's words for its state: the rail's (AJ-E2104 Req.3), never a word
 * of its own. */
export function stateWord(conversation: ConversationInfo): string {
  const item = conversationItem(conversation, () => null, null);
  return railStateWord(item.state, item.probably);
}

/** Its state's colour (the glyph and the word), as the place rows show it. */
function stateTone(conversation: ConversationInfo): string {
  return STATE_TONE[conversationItem(conversation, () => null, null).state].text;
}


/** Which outside conversations the rail's filters let through: the open
 * ones unfiltered, the live ones under "Running", the ended ones under
 * "Ended". 0.4.4: searched or narrowed (`found`), every open one that
 * matches, one that asks you too (no Needs you leads a narrowed board),
 * as AI Juggler's own show (`narrowOutside` says which match). */
export type OutsideScope = "all" | "found" | "live" | "ended" | "none";

export function outsideScope(filters: ChipFilters, query: string): OutsideScope {
  const narrowed = query.trim() !== "" || filters.provider !== null || filters.account !== null || filters.project !== null;
  if (filters.chips.length === 0) return narrowed ? "found" : "all";
  if (filters.chips.length === 1 && filters.chips[0] === "running") return "live";
  if (filters.chips.length === 1 && filters.chips[0] === "ended") return "ended";
  return "none";
}

/** The live ones each scope shows under a project (`ended` shows none live). */
const LIVE_SHOWN: Record<OutsideScope, (conversation: ConversationInfo) => boolean> = {
  all: openNow,
  found: (conversation) => conversation.live && !conversation.background,
  live: (conversation) => conversation.live,
  ended: () => false,
  none: () => false,
};

/** 0.4.4: other apps' sessions narrowed as the board narrows AI Juggler's
 * own: Only › Project, Only › Account (`account`, its name) and the
 * search's words, by the rail's one rule (`matches`, the App lens's). */
export function narrowOutside(all: ConversationInfo[], filters: ChipFilters, text: string, projects: ProjectInfo[], account: string | undefined, accountOf: AccountOf): ConversationInfo[] {
  const query = parseQuery(text);
  const nameOf = (id: string | null) => projects.find((project) => project.id === id)?.name ?? null;
  return all.filter((conversation) => {
    if (filters.project !== null && conversation.project_id !== filters.project) return false;
    if (filters.account !== null && conversation.account_id !== filters.account) return false;
    if (filters.provider !== null && conversation.tool !== filters.provider) return false;
    return matches(conversationItem(conversation, nameOf, null), account === undefined ? {} : { account }, query, accountOf);
  });
}

/** What the rail shows of one project's outside conversations: the open
 * ones (AJ-E2103: no ended ones under every project); under the Running
 * filter every live one, under the Ended filter the newest few ended and
 * how many more there are. */
export function railSlice(all: ConversationInfo[], projectId: string, scope: OutsideScope = "all"): { shown: ConversationInfo[]; more: number } {
  const mine = outside(all).filter((conversation) => conversation.project_id === projectId);
  if (scope !== "ended") return { shown: mine.filter(LIVE_SHOWN[scope]), more: 0 };
  const ended = mine.filter((conversation) => !conversation.live);
  return { shown: ended.slice(0, RECENT_SHOWN), more: ended.length - Math.min(ended.length, RECENT_SHOWN) };
}

/** The rail's project groups, plus one for each project where a session
 * started outside AI Juggler is running now (and none of AI Juggler's):
 * a live session always shows under its project. */
export function withOutsideGroups(groups: RowGroup[], all: ConversationInfo[], projects: ProjectInfo[], scope: OutsideScope = "all"): RowGroup[] {
  if (scope === "none" || scope === "ended") return groups;
  const have = new Set(groups.map((group) => group.id));
  const live = new Set(outside(all).filter(LIVE_SHOWN[scope]).map((conversation) => conversation.project_id));
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
  const when = conversation.last_activity_ms === null ? "" : conversation.live ? ago(conversation.last_activity_ms, now) : agoWords(conversation.last_activity_ms, now);
  const state = stateWord(conversation);
  const menu = useItemMenu("conversation", conversation, title);
  // Its click and ↩ open its page here; ↗ and ⌘↩ go to it in its app
  // (the maintainer's review of 0.1.4), always shown while it asks you.
  const primary = () => onOpen(conversation);
  const asking = asksYou(conversation);
  const goTo = menu.goTo({ where: goToWhere(conversationItem(conversation, () => null, null)) ?? conversation.account_label, shown: asking, className: "absolute right-1 top-1/2 -translate-y-1/2" });
  // AJ-E2103: open in two places is one row that says so.
  // (A fixture or an older backend may send none.)
  const places = conversation.places ?? [];
  const where = places.map((place) => place.label || "an app AI Juggler can't name").join(" and ");
  return (
    <div className="group relative" data-rail-row {...menu.triggerProps}>
      <button
        type="button"
        data-conversation-id={conversation.id}
        data-testid="rail-outside"
        aria-current={selected ? "true" : undefined}
        data-rail-selected={selected ? "true" : undefined}
        onClick={primary}
        className={
          "relative flex w-full min-w-0 items-center gap-2.5 rounded-lg pl-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (goTo && asking ? "pr-9 " : "pr-2 ") +
          (density === "compact" ? "py-1.5 " : "py-2 ") +
          (selected ? "bg-surface" : "hover:bg-surface/60")
        }
      >
        {selected && <span aria-hidden="true" className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />}
        <span aria-hidden="true" className={"flex w-4 shrink-0 justify-center " + stateTone(conversation)}>
          <Icon width={13} height={13} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-baseline gap-1.5">
            <span className={"min-w-0 truncate text-meta font-medium " + (conversation.live ? "text-ink" : "text-muted")}>{title}</span>
            {density === "compact" && when && <span className="ml-auto shrink-0 text-label tabular-nums text-muted">{when}</span>}
          </span>
          {density === "comfortable" && (
            <span className="flex min-w-0 items-center gap-1 overflow-hidden text-label text-muted">
              <span className={"shrink-0 whitespace-nowrap " + stateTone(conversation)} data-testid="rail-state-word">{state}</span>
              <span className="shrink-0 whitespace-nowrap" data-testid="outside-mark">
                · outside AI Juggler
              </span>
              {places.length > 1 && (
                <span className="shrink-0 whitespace-nowrap" data-testid="outside-places" title={`Open in ${where}`}>
                  · open in {places.length} places
                </span>
              )}
              {when && <span className="min-w-0 truncate">· {when}</span>}
            </span>
          )}
          <span className="sr-only">
            , {state}, outside AI Juggler, {providerDisplayName(conversation.tool)} on {conversation.account_label}
            {where && `, open in ${where}`}
          </span>
        </span>
        <AppIcon {...appOfConversation(conversation)} size={16} />
        <ProfileBadge app={conversationAppKey(conversation)} name={conversation.account_label} />
      </button>
      {goTo}
      {menu.button({ label: `Conversation actions for ${title}`, className: "absolute bottom-0 left-0" })}
      {menu.element}
    </div>
  );
}

/** One project's outside conversations, as the rail shows them (`all`:
 * the board's, narrowed as it is). */
export function OutsideRows({
  all,
  projectId,
  selectedId,
  density,
  now,
  scope,
}: {
  all: ConversationInfo[];
  projectId: string;
  selectedId: string | null;
  density: "comfortable" | "compact";
  now: number;
  scope: OutsideScope;
}): JSX.Element | null {
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
          className="self-start rounded-md py-1 pl-7 pr-2 text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          {more} more outside AI Juggler
        </button>
      )}
    </>
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
            <ProjectMark projectId={conversation.project_id} size={16} />
            <span className="min-w-0 flex-1 truncate text-sm text-ink">
              {conversationTitle(conversation)}
              <span className="text-muted"> · outside AI Juggler</span>
            </span>
            <span className="flex items-center gap-1.5 text-xs text-muted">
              <AppIcon {...appOfConversation(conversation)} size={16} />
              <ProfileBadge app={conversationAppKey(conversation)} name={conversation.account_label} />
              <span><span className="sr-only">{providerDisplayName(conversation.tool)} on </span>{conversation.account_label}</span>
            </span>
            {/* Its last activity: a resumed one can be months old. */}
            <span className="w-14 text-right text-label tabular-nums text-muted" title="Last activity">
              {conversation.last_activity_ms === null ? "" : ago(conversation.last_activity_ms, now)}
            </span>
          </button>
        </li>
      ))}
    </>
  );
}
