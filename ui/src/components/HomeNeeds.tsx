/**
 * AJ-E2105 with D-118's mock (Home round 3) — Home's Needs you now: one
 * card per need, in Needs you's one order (the louder level, then the one
 * waiting longest: `needs-order`, the rail's too), whatever its kind: a
 * session's (its title, what it needs in its level's colour and what it
 * asks when its tool said; "Open its page ↵", "Continue in another app…",
 * ↗), an agent's message (MessageNeedCard: "Asks you: …", Reply · Open
 * the thread), a task's (TaskNeedCard), a sign-in, This Mac, processes
 * left behind. The first card's primary is the one filled button (Enter).
 * Approvals (a tool's, a pool's, a task's) keep their batch of approval
 * cards above (keys 1–9). A session another app runs has its ↗ too (⌘↩
 * on it; the maintainer's review of 0.1.4), and its menu on a right-click
 * or ⇧F10. Message text is shown only through `bubble`, a session's last
 * message when its page's reader provides one (AJ-E2108).
 */
import { useKeys } from "../lib/use-keys";
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { Button } from "./primitives";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { PlaceMark } from "./PlaceMarks";
import { homeNeedLevel, needName, primaryName, primaryWords, type HomeNeed } from "../lib/home-needs";
import { needWord } from "./PlaceRows";
import { markSeen } from "../lib/seen";
import { focusHandoverSoon } from "../lib/handover-focus";
import { switchSourceOf } from "../lib/ipc/conversations";
import { AskBatch } from "./AskCard";
import { isHeldAsk, MessageNeedCard, MessageNeeds } from "./MessageNeeds";
import { isTaskAsk, TaskNeedCard, TaskNeeds } from "./TaskNeeds";
import { NeedActions, NeedByLine, needCardClass } from "./NeedCard";
import { mergeByWait } from "../lib/needs-order";
import { levelTone } from "../lib/attention";
import type { MessageNeedInfo } from "../lib/ipc/messages";
import type { TaskNeedInfo } from "../lib/ipc/tasks";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { goToWhere, placeWords, type RailItem } from "../lib/rail-items";
import { askExcerpt, needWait, waitingSince } from "../lib/waiting";

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
          if (second) focusHandoverSoon(session.id);
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
          focusHandoverSoon(need.sessionId);
        } else if (need.conversation) {
          host.openConversation?.(need.conversation);
          focusHandoverSoon(switchSourceOf(need.conversation));
        } else if (need.account) host.openAccount?.(need.account);
        return;
      case "leftovers":
        handlers.onOpenCleanup?.();
    }
  };
}

/** Pinned needs shown before "+n more need you" (D-118 R2). */
export const PINNED_SHOWN = 3;

/** AJ-E2910: what Home's row keys read: its session, and whether it has a
 * conversation to carry over. */
function rowData(need: HomeNeed): Record<string, string | undefined> {
  const session = need.item?.session ?? null;
  return {
    "data-need-key": need.key,
    "data-session-id": session?.id ?? need.sessionId,
    "data-has-conversation": session?.conversation_id ? "yes" : undefined,
    "data-level": String(homeNeedLevel(need)), // D-117: its attention level
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

/** One entry of Needs you now: a need of Home's, an agent's message, a
 * task's; where it stands in the one order (`needs-order`). */
type Entry =
  | { kind: "need"; key: string; need: HomeNeed; level: number; since: number }
  | { kind: "message"; key: string; message: MessageNeedInfo; level: number; since: number }
  | { kind: "task"; key: string; task: TaskNeedInfo; level: number; since: number };

/** A need's wait start: its session's (`lib/waiting`), else its notice's;
 * one with neither after the rest of its level. */
function sinceOf(need: HomeNeed): number {
  return need.item ? waitingSince(need.item) : (need.since ?? Number.MAX_SAFE_INTEGER);
}

/** Every entry, in the one order: Home's needs (their own order kept),
 * agents' messages and tasks, each oldest first, merged by level, then
 * the longest wait (an agent's message or task waits on you: L2). */
export function needsOrder(needs: HomeNeed[], messages: MessageNeedInfo[], tasks: TaskNeedInfo[]): Entry[] {
  const own = needs.map((need): Entry => ({ kind: "need", key: need.key, need, level: homeNeedLevel(need), since: sinceOf(need) }));
  const said = messages.filter((m) => !isHeldAsk(m)).map((message): Entry => ({ kind: "message", key: `message:${message.key}`, message, level: 2, since: message.at_ms })).sort((a, b) => a.since - b.since);
  const asked = tasks.filter((t) => !isTaskAsk(t)).map((task): Entry => ({ kind: "task", key: `task:${task.id}`, task, level: 2, since: task.at_ms })).sort((a, b) => a.since - b.since);
  return mergeByWait([own, said, asked], (entry) => entry);
}

/** What it needs, inline after its name ("asks you", "limit reached"). */
function inlineWord(word: string): string {
  return word.charAt(0).toLowerCase() + word.slice(1);
}

/** A session's need as its card (D-118's mock): "Claude app · Work Max
 * payments-api  waiting 12 min", then "Migrate the billing webhooks asks
 * you: “…”" (what it asks when its tool said; else where to see it,
 * D-96 A), then its actions. Another need (a sign-in, This Mac…): its
 * title, why, and its primary. */
function NeedCardOf({ need, first, now, bubble, keys, run }: { need: HomeNeed; first: boolean; now?: number; bubble?: ReactNode; keys?: string; run: (need: HomeNeed, second?: boolean) => void }): JSX.Element {
  const level = homeNeedLevel(need);
  const item = need.item;
  const wait = item && now !== undefined ? needWait(item, now) : null;
  const asks = item?.need === "asks" ? askExcerpt(item.session) : null;
  const second = secondWords(need);
  return (
    <NeedBox need={need} tag="li" className={needCardClass(level)} data={{ "aria-label": needName(need), "data-home-item": "", "data-testid": `home-need-${need.kind}`, "data-first": first ? "true" : undefined, "data-kind": need.kind, ...rowData(need) }}>
      {(goTo) => (
        <>
          <Mark need={need} size={22} />
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            {item ? (
              <>
                <NeedByLine author={placeWords(item.place).join(" · ")} place={item.project} time={wait} timeTestId="need-wait" />
                <span className="text-body text-ink">
                  <span className="font-semibold">{need.title}</span>{" "}
                  <span className={"font-semibold " + levelTone(level).text} data-testid="need-state">
                    {level === 3 && <span aria-hidden="true">✕ </span>}
                    {inlineWord(needWord(item))}
                  </span>
                  {item.need === "asks" &&
                    (asks ? (
                      <span data-testid="need-asks">: “{asks}”</span>
                    ) : (
                      // D-96 A: its app said it asks, never what.
                      <span className="text-muted" data-testid="need-asks"> — its app doesn't say what; open it to see</span>
                    ))}
                </span>
              </>
            ) : (
              <>
                <span className="text-body font-semibold text-ink">{need.title}</span>
                {need.detail && (
                  <span className="line-clamp-2 text-meta text-muted" title={need.detail} data-testid="need-line">
                    {need.detail}
                  </span>
                )}
              </>
            )}
            {first && bubble}
            <NeedActions>
              <Button size="sm" variant={first ? "primary" : "ghost"} data-home-row aria-keyshortcuts={first ? "Enter" : keys} aria-label={primaryName(need)} onClick={() => run(need)}>
                {primaryWords(need)}
                {first && <span aria-hidden="true" className="opacity-80">↵</span>}
              </Button>
              {second && (
                <Button size="sm" variant="ghost" data-home-action onClick={() => run(need, true)}>
                  {second}
                  <span className="sr-only"> — {need.title}</span>
                </Button>
              )}
              {goTo}
            </NeedActions>
          </span>
        </>
      )}
    </NeedBox>
  );
}

export function HomeNeeds({
  needs,
  messages = [],
  tasks = { needs: [], projects: [] },
  handlers,
  bubble,
  children,
  now,
}: {
  needs: HomeNeed[];
  /** The clock its waits are read against ("waiting 7 h"); none, no wait shown. */
  now?: number;
  messages?: MessageNeedInfo[];
  /** E19: tasks that wait on you, and the projects (for a launch's place). */
  tasks?: { needs: TaskNeedInfo[]; projects: ProjectInfo[] };
  handlers: NeedHandlers;
  bubble?: ReactNode;
  children?: ReactNode;
}): JSX.Element | null {
  const actionKeys = useKeys("home-action");
  const run = useRun(handlers);
  // D-118 (AJ-E3702 R2): past 3, the rest wait behind "+n more need you"
  // (the pinned zone stays short; the count above says them all).
  const [more, setMore] = useState(false);
  // Opened, focus goes to the first need it held (its button goes away).
  const reveal = useRef<string | null>(null);
  useEffect(() => {
    const key = reveal.current;
    if (!more || key === null) return;
    reveal.current = null;
    document.querySelector<HTMLElement>(`[data-testid="home-needs"] [data-need-key="${CSS.escape(key)}"] button`)?.focus();
  }, [more]);
  const entries = needsOrder(needs, messages, tasks.needs);
  const shown = more ? entries : entries.slice(0, PINNED_SHOWN);
  const held = entries.length - shown.length;
  const asks = messages.some(isHeldAsk) || tasks.needs.some(isTaskAsk);
  // Nothing waits: the zone says so in one muted line (Home's own).
  if (entries.length === 0 && !children && !asks) return null;
  return (
    // No area title: the page's heading already says what needs you (the
    // 0.4 design review, 42); the name stays for screen readers.
    <section aria-label="Needs you" className="flex min-w-0 flex-col gap-2" data-testid="home-needs">
      <AskBatch titleOf={(id) => needs.find((need) => need.item?.session?.id === id)?.title} />
      {/* E18, E19: approvals from agents (their cards; keys 1–9). */}
      <MessageNeeds needs={messages} />
      <TaskNeeds needs={tasks.needs.filter(isTaskAsk)} projects={tasks.projects} />
      {shown.length > 0 ? (
        <ul className="flex flex-col gap-2">
          {shown.map((entry, index) =>
            entry.kind === "need" ? (
              <NeedCardOf key={entry.key} need={entry.need} first={index === 0} now={now} bubble={bubble} keys={actionKeys.aria} run={run} />
            ) : entry.kind === "message" ? (
              <MessageNeedCard key={entry.key} need={entry.message} first={index === 0} now={now} keys={actionKeys.aria} />
            ) : (
              <TaskNeedCard key={entry.key} need={entry.task} projects={tasks.projects} now={now} />
            ),
          )}
        </ul>
      ) : asks || children ? null : (
        <p className="rounded-xl bg-surface p-4 text-sm text-ink-2" data-testid="home-needs-none">
          Nothing waits on you. What's open is in the list and under Your apps.
        </p>
      )}
      {held > 0 && (
        <button type="button" aria-expanded={false} onClick={() => { reveal.current = entries[shown.length]?.key ?? null; setMore(true); }} className="self-start rounded text-meta text-ink-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="home-needs-more">
          +{held} more {held === 1 ? "needs" : "need"} you
        </button>
      )}
      {children}
    </section>
  );
}
