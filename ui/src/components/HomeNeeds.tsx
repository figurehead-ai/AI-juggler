/**
 * AJ-E2105 — Home's Needs you card: the first thing that waits on the
 * user, large (its title, where it is, and its primary: "Open its page ↵"
 * for a session, with "Continue in another app…" beside it), then the
 * rest, one line each with its own primary. A session another app runs
 * has its ↗ too, the way to its app (⌘↩ on it; the maintainer's review of
 * 0.1.4), and its menu on a right-click or ⇧F10. Every kind of item sits here
 * (lib/home-needs.ts). Message text is shown only through `bubble`, a
 * session's last message when its page's reader provides one (AJ-E2108).
 */
import { useKeys } from "../lib/use-keys";
import { useState, type JSX, type ReactNode } from "react";
import { Button } from "./primitives";
import { SectionTitle } from "./SectionTitle";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { PlaceMark } from "./PlaceMarks";
import { chipOf, homeChips, primaryWords, type HomeChip, type HomeNeed } from "../lib/home-needs";
import { Segmented } from "./controls";
import { needWord } from "./PlaceRows";
import { markSeen } from "../lib/seen";
import { focusHandoverSoon } from "../lib/handover-focus";
import { AskBatch } from "./AskCard";
import { MessageNeeds } from "./MessageNeeds";
import { TaskNeeds } from "./TaskNeeds";
import type { MessageNeedInfo } from "../lib/ipc/messages";
import type { TaskNeedInfo } from "../lib/ipc/tasks";
import type { ProjectInfo } from "../lib/bridge";
import { goToWhere, type RailItem } from "../lib/rail-items";

export interface NeedHandlers {
  onOpenSession: (id: string) => void;
  onLimit?: (id: string) => void;
  onWait?: (id: string) => void;
  onSignIn?: (need: HomeNeed) => void;
  onOpenThisMac?: () => void;
  onOpenCleanup?: () => void;
}

function Mark({ need, size }: { need: HomeNeed; size: number }): JSX.Element {
  if (need.item) return <PlaceMark place={need.item.place} profile={need.item.accountLabel} size={size} ring="var(--color-surface)" />;
  const glyph = need.kind === "signin" ? "→" : need.severe ? "■" : "▲";
  return (
    <span aria-hidden="true" style={{ width: size, height: size }} className={"flex shrink-0 items-center justify-center rounded-lg text-xs font-bold " + (need.kind === "signin" ? "bg-accent-soft text-accent-hi" : need.severe ? "bg-danger-soft text-danger" : "bg-warning-soft text-warning")}>
      {glyph}
    </span>
  );
}

/** Runs an item's primary (or its second choice). */
function useRun(handlers: NeedHandlers): (need: HomeNeed, second?: boolean) => void {
  const host = useActionHost();
  // E3104: a need whose session the list hasn't loaded opens by its ids.
  const openUnloaded = (need: HomeNeed) => {
    if (need.sessionId) handlers.onOpenSession(need.sessionId);
    else if (need.conversationIds) host.openConversationById?.(need.conversationIds.accountId, need.conversationIds.id, null);
  };
  return (need, second = false) => {
    const item = need.item;
    const session = item?.session;
    const conversation = item?.conversation;
    switch (need.kind) {
      case "session":
      case "failed":
        if (conversation) {
          markSeen(conversation.account_id, conversation.id);
          // Its page here ("Continue in another app…" lives there,
          // AJ-E2108); its ↗ goes to its app.
          host.openConversation?.(conversation);
        } else if (session) {
          handlers.onOpenSession(session.id);
          if (second) focusHandoverSoon();
        } else openUnloaded(need);
        return;
      case "limit":
        if (session) (second ? handlers.onWait : handlers.onLimit)?.(session.id);
        else openUnloaded(need);
        return;
      case "signin":
        handlers.onSignIn?.(need);
        return;
      case "mac":
        handlers.onOpenThisMac?.();
        return;
      case "heavy":
        if (need.sessionId) handlers.onOpenSession(need.sessionId);
        return;
      case "quota":
        // F16: its open session's switch list (AI Juggler's or another
        // app's), else its account.
        if (need.sessionId) {
          handlers.onOpenSession(need.sessionId);
          focusHandoverSoon();
        } else if (need.conversation) {
          host.openConversation?.(need.conversation);
          focusHandoverSoon();
        } else if (need.account) host.openAccount?.(need.account);
        return;
      case "leftovers":
        handlers.onOpenCleanup?.();
    }
  };
}

/** AJ-E2910: what Home's row keys read: its session, and whether it has a
 * conversation to carry over. */
function rowData(need: HomeNeed): Record<string, string | undefined> {
  const session = need.item?.session ?? null;
  return {
    "data-session-id": session?.id ?? need.sessionId,
    "data-has-conversation": session?.conversation_id ? "yes" : undefined,
  };
}

type BoxProps = { tag: "div" | "li"; className: string; data: Record<string, string | undefined>; children: (goTo: ReactNode) => ReactNode };

/** A need's box. One that stands for a session carries its menu, ⌘↩ and
 * its ↗ (`children` places it). */
function NeedBox({ need, ...props }: BoxProps & { need: HomeNeed }): JSX.Element {
  if (need.item) return <SessionBox item={need.item} {...props} />;
  const Tag = props.tag;
  return (
    <Tag className={props.className} {...props.data}>
      {props.children(null)}
    </Tag>
  );
}

function SessionBox({ item, tag: Tag, className, data, children }: BoxProps & { item: RailItem }): JSX.Element {
  const menu = useItemMenu(item.session ? "session" : "conversation", (item.session ?? item.conversation) as never, item.title);
  const where = goToWhere(item);
  return (
    <Tag className={className} {...data} {...menu.triggerProps}>
      {children(where === null ? null : menu.goTo({ where, shown: true }))}
      {menu.element}
    </Tag>
  );
}

/** Its second choice, when it has one. */
function secondWords(need: HomeNeed): string | null {
  if (need.kind === "session" && need.item?.need === "asks") return "Continue in another app…";
  if (need.kind === "limit") return "Wait";
  return null;
}

export function HomeNeeds({
  needs,
  messages = [],
  tasks = { needs: [], projects: [] },
  handlers,
  bubble,
  children,
}: {
  needs: HomeNeed[];
  messages?: MessageNeedInfo[];
  /** E19: tasks that wait on you, and the projects (for a launch's place). */
  tasks?: { needs: TaskNeedInfo[]; projects: ProjectInfo[] };
  handlers: NeedHandlers;
  bubble?: ReactNode;
  children?: ReactNode;
}): JSX.Element | null {
  const actionKeys = useKeys("home-action");
  const run = useRun(handlers);
  // AJ-E2910: a chip narrows the list (never remembered: a stale filter
  // would hide a new need); the counts elsewhere never change with it.
  const [chip, setChip] = useState<HomeChip>("all");
  const chips = homeChips(needs, { asks: messages.length, review: tasks.needs.length });
  const shownChip = chips.some((entry) => entry.value === chip) ? chip : "all";
  const shownNeeds = shownChip === "all" ? needs : needs.filter((need) => chipOf(need) === shownChip);
  const showMessages = shownChip === "all" || shownChip === "asks";
  const showTasks = shownChip === "all" || shownChip === "review";
  const hidden = needs.length + messages.length + tasks.needs.length - shownNeeds.length - (showMessages ? messages.length : 0) - (showTasks ? tasks.needs.length : 0);
  const [first, ...rest] = shownNeeds;
  // Nothing waits: the heading says so, and the card stays away.
  const waiting = (showMessages ? messages.length : 0) + (showTasks ? tasks.needs.length : 0);
  if (needs.length === 0 && !children && messages.length + tasks.needs.length === 0) return null;
  return (
    <section aria-labelledby="home-needs" className="flex min-w-0 flex-col gap-2.5" data-testid="home-needs">
      <SectionTitle id="home-needs">Needs you</SectionTitle>
      <div className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface p-4">
        {chips.length > 0 && (
          <div className="flex flex-wrap items-center gap-2" data-testid="home-chips">
            <Segmented label="Show what needs you" value={shownChip} segments={chips.map(({ value, label }) => ({ value, label }))} onChange={setChip} testId="home-chip" />
            {hidden > 0 && (
              <button type="button" className="text-meta text-muted hover:text-ink hover:underline" onClick={() => setChip("all")} data-testid="home-chips-hidden">
                {hidden} more under All
              </button>
            )}
          </div>
        )}
        {(shownChip === "all" || shownChip === "asks") && <AskBatch titleOf={(id) => needs.find((need) => need.item?.session?.id === id)?.title} />}
        {/* E18: agents' messages that wait on you. */}
        {showMessages && <MessageNeeds needs={messages} />}
        {showTasks && <TaskNeeds needs={tasks.needs} projects={tasks.projects} />}
        {first ? (
          <NeedBox need={first} tag="div" className="flex flex-col gap-3" data={{ "data-home-item": "", "data-testid": `home-need-${first.kind}`, "data-first": "true", "data-kind": first.kind, ...rowData(first) }}>
            {(goTo) => (
            <>
            <div className="flex min-w-0 items-start gap-3">
              <Mark need={first} size={32} />
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate text-section font-semibold text-ink">{first.title}</span>
                {/* 0.3.2: the first need's why is the point of the card; two lines, not one cut. */}
                <span className="line-clamp-2 text-xs text-muted" title={first.detail}>{first.detail}</span>
              </div>
              {first.item && <span className={"shrink-0 text-xs font-semibold " + (first.severe ? "text-danger" : "text-warning")}>{needWord(first.item)}</span>}
            </div>
            {bubble}
            <div className="flex flex-wrap items-center gap-2">
              <Button size="sm" variant="primary" data-home-row aria-keyshortcuts="Enter" onClick={() => run(first)}>
                {primaryWords(first)}
                <span aria-hidden="true" className="opacity-80">↵</span>
                <span className="sr-only"> — {first.title}</span>
              </Button>
              {goTo}
              {secondWords(first) && (
                <Button size="sm" variant="secondary" data-home-action onClick={() => run(first, true)}>
                  {secondWords(first)}
                  <span className="sr-only"> — {first.title}</span>
                </Button>
              )}
            </div>
            </>
            )}
          </NeedBox>
        ) : waiting > 0 ? null : (
          <p className="text-sm text-ink-2" data-testid="home-needs-none">
            Nothing waits on you. What's open is in the list and under Your apps.
          </p>
        )}
        {rest.length > 0 && (
          <ul className="flex flex-col border-t border-hairline/60 pt-1">
            {rest.map((need) => (
              <NeedBox key={need.key} need={need} tag="li" data={{ "data-home-item": "", "data-kind": need.kind, "data-testid": `home-need-${need.kind}`, ...rowData(need) }} className="flex min-w-0 items-center gap-2.5 border-b border-hairline/40 py-2 last:border-b-0">
                {(goTo) => (
                <>
                <Mark need={need} size={24} />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-meta font-medium text-ink">{need.title}</span>
                  <span className="truncate text-label text-muted" title={need.detail}>{need.item ? `${needWord(need.item)} · ${need.detail}` : need.detail}</span>
                </span>
                <Button size="sm" variant="secondary" data-home-row aria-keyshortcuts={actionKeys.aria} onClick={() => run(need)}>
                  {primaryWords(need)}
                  <span className="sr-only"> — {need.title}</span>
                </Button>
                {goTo}
                </>
                )}
              </NeedBox>
            ))}
          </ul>
        )}
        {children}
      </div>
    </section>
  );
}
