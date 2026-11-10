/**
 * AJ-E2108 — a session's conversation, on its page only (never in rows,
 * the tray or notifications): its turns as its own app shows them
 * (`ConversationTurn`). 0.3.5 (ADR-0024): followed while the page shows
 * it (`useConversation`): its latest turns once, then only what changes,
 * appended or grown in place under stable ids; the view stays where the
 * reader is (`useStickToBottom`); a problem after turns were shown is a
 * line over them, never an empty card. Earlier turns on request; keys
 * and tokens taken out; held in memory, never written (ADR-0024). When it asks you, the question
 * waits at the bottom with Go to it. A chat its app keeps on its servers
 * has no transcript on this Mac: its title is all there is.
 */
import { useEffect, useLayoutEffect, useMemo, useRef, type JSX, type ReactNode } from "react";
import { Button, Card } from "../../components/primitives";
import { IconGoTo } from "../../components/icons-actions";
import type { TurnInfo } from "../../lib/ipc/conversations";
import type { FeedState } from "../../lib/conversation-feed-state";
import { plainWords } from "./Markdown";
import { foldOf } from "./TurnWork";
import { Turn } from "./ConversationTurn";
import { sessionSource, useConversation, type FeedSource } from "./useConversation";
import { useStickToBottom } from "./useStickToBottom";
import { NewMessagesPill } from "./NewMessagesPill";
import { appWords, picksInSidebar, speakerName, type SessionModel } from "./model";

/** Why there's nothing to show, in words. */
const NONE: Record<string, string> = {
  none: "There's no transcript of it on this computer to read, so its title is all AI Juggler shows.",
  compressed: "Its transcript is stored compressed now that it has ended, so it isn't read here.",
  unreadable: "Its transcript couldn't be read just now.",
};

/** What a line over the turns says (they stay shown). */
const BANNER: Record<string, string> = {
  unreadable: "Couldn't read the latest changes. Trying again.",
  retrying: "Couldn't reach the latest changes. Trying again.",
  gone: "Its transcript is no longer on this computer. This is what was read.",
  none: "Its transcript is no longer on this computer. This is what was read.",
  ended: "It ended and its transcript was stored compressed. This is what was read.",
  compressed: "It ended and its transcript was stored compressed. This is what was read.",
  earlier: "Couldn't load earlier messages.",
};

/** Meta text: 7:1 or better on every surface, both themes. */
const META = "text-meta text-muted";

/** "1 key", "3 keys". */
function keysWords(count: number): string {
  return count === 1 ? "1 key" : `${count} keys`;
}

function openSettings(): void {
  window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "last-messages" } }));
}

export function LastMessages({
  model,
  visible,
  remoteMac,
  onGoTo,
  source,
  title = "Conversation",
  head,
  asker,
}: {
  model: SessionModel;
  visible: boolean;
  remoteMac?: string | null;
  /** Go to it, from the question card (a live session). */
  onGoTo?: () => void;
  /** AJ-E1520: another conversation than the session's own (a
   * subagent's), followed the same way; it asks no one, so no question
   * card. */
  source?: FeedSource;
  /** The card's heading. */
  title?: string;
  /** Before the heading (a way back). */
  head?: ReactNode;
  /** Who the "you" side is, said over its turns ("The session"). */
  asker?: string;
}): JSX.Element {
  const own = useMemo(() => (model.conversationId === null ? null : sessionSource(model.profileId, model.conversationId)), [model.profileId, model.conversationId]);
  const { state: chat, showEarlier } = useConversation({ source: remoteMac ? null : (source ?? own), visible: visible && !remoteMac });
  const speaker = speakerName(model.tool);
  const following = chat.status === "shown" && chat.subscription !== null && model.live && visible;
  return (
    // Sized to what it holds, at most the column's height (then its
    // messages scroll): a short conversation has its question card right
    // under its last message, never a void between them.
    <Card className="reading flex min-h-[22rem] flex-initial flex-col overflow-hidden @4xl:min-h-0" data-testid="last-messages">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline px-5 py-3.5">
        {head}
        <h3 className="min-w-0 max-w-full truncate text-section font-semibold text-ink">{title}</h3>
        {following && (
          <span className="inline-flex items-center gap-1.5 text-label font-medium text-live" data-testid="last-messages-live" title="Following along while it runs">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-current motion-safe:animate-pulse" />
            Live
          </span>
        )}
        {chat.status === "shown" && (
          <span className={"ml-auto " + META} data-testid="last-messages-note">
            From its transcript · not saved
            {redacted(chat.turns) > 0 && ` · ${keysWords(redacted(chat.turns))} taken out`}
          </span>
        )}
      </div>
      {remoteMac ? (
        <Quiet testId="last-messages-remote">Open on {remoteMac} to read: its messages stay on that Mac.</Quiet>
      ) : model.conversationId === null ? (
        <Quiet>It hasn't written anything yet.</Quiet>
      ) : (
        <Body chat={chat} model={model} speaker={speaker} onShowEarlier={showEarlier} asker={asker} />
      )}
      {!remoteMac && !source && <Bottom chat={chat} model={model} speaker={speaker} onGoTo={onGoTo} />}
    </Card>
  );
}

/** Keys or tokens taken out of what is shown. */
function redacted(turns: TurnInfo[]): number {
  return turns.reduce((sum, turn) => sum + turn.redacted, 0);
}

function Quiet({ children, testId, state }: { children: ReactNode; testId?: string; state?: string }): JSX.Element {
  return (
    <div className="flex flex-1 items-center justify-center px-6 py-10">
      <p className={"max-w-md text-center text-body leading-[1.5] text-muted"} data-testid={testId} data-state={state} role="status">
        {children}
      </p>
    </div>
  );
}

function Body({ chat, model, speaker, onShowEarlier, asker }: { chat: FeedState; model: SessionModel; speaker: string; onShowEarlier: () => void; asker?: string }): JSX.Element {
  const scroller = useRef<HTMLDivElement | null>(null);
  const turns = chat.turns;
  const last = turns[turns.length - 1];
  const { onScroll, pill, toBottom, keep } = useStickToBottom(scroller, {
    firstId: turns[0]?.id ?? null,
    lastKey: last ? `${last.id}:${last.revision}:${turns.length}` : null,
  });
  // A line over the turns takes room from them: the bottom stays in view.
  useLayoutEffect(keep, [chat.banner, keep]);
  // A first read that passed a long stretch with no turn: read on.
  const further = chat.status === "shown" && turns.length === 0 && chat.earlier !== null;
  useEffect(() => {
    if (further && !chat.loadingEarlier) onShowEarlier();
  }, [further, chat.loadingEarlier, onShowEarlier]);

  if (turns.length === 0) return <Empty chat={chat} further={further} />;
  const asksAt = model.state.asks && last?.speaker === "assistant" ? turns.length - 1 : -1;
  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      {chat.banner && BANNER[chat.banner] && (
        <p className={"border-b border-hairline bg-surface-2 px-5 py-2 " + META} role="status" data-testid="last-messages-banner" data-banner={chat.banner}>
          {BANNER[chat.banner]}
        </p>
      )}
      <div ref={scroller} onScroll={onScroll} className="min-h-0 flex-1 overflow-y-auto outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60" tabIndex={0} role="region" aria-label="Messages" data-testid="last-messages-scroll">
        <div className="mx-auto flex w-full max-w-[760px] flex-col gap-6 px-5 py-5">
          {chat.earlier !== null ? (
            <div className="flex justify-center">
              <Button size="md" variant="secondary" onClick={onShowEarlier} disabled={chat.loadingEarlier || chat.subscription === null} data-testid="last-messages-earlier">
                {chat.lookingFurther ? "Looking further back…" : chat.loadingEarlier ? "Loading earlier messages…" : "Show earlier messages"}
              </Button>
            </div>
          ) : (
            <p className={"text-center " + META} data-testid="last-messages-start">
              The start of the conversation
            </p>
          )}
          <ol className="flex flex-col gap-6" data-testid="last-messages-turns">
            {turns.map((turn, index) => (
              <Turn
                key={turn.id}
                turn={turn}
                tool={model.tool}
                speaker={speaker}
                layout={chat.layout}
                asks={index === asksAt}
                asker={asker}
                startedMs={turns[index - 1]?.at_ms ?? null}
                working={model.live && model.state.working && index === turns.length - 1}
              />
            ))}
          </ol>
        </div>
      </div>
      {pill && <NewMessagesPill onClick={toBottom} testId="last-messages-new" />}
    </div>
  );
}

/** Before any turn was shown: loading, or why there is none. */
function Empty({ chat, further }: { chat: FeedState; further: boolean }): JSX.Element {
  if (chat.status === "failed") return <Quiet>Couldn't read them: {chat.why}</Quiet>;
  if (chat.status === "loading" || further) {
    return (
      <div className="flex flex-1 flex-col gap-5 px-5 py-6" aria-label={further ? "Looking further back" : "Loading the conversation"} role="status">
        <span className="ml-auto h-10 w-1/2 rounded-2xl bg-skeleton motion-safe:animate-pulse" />
        <span className="h-16 w-3/4 rounded-2xl bg-skeleton motion-safe:animate-pulse" />
        <span className="ml-auto h-8 w-1/3 rounded-2xl bg-skeleton motion-safe:animate-pulse" />
      </div>
    );
  }
  if (chat.status === "off") {
    return (
      <Quiet testId="last-messages-off">
        Last messages are off.{" "}
        <button type="button" onClick={openSettings} className="font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          Turn them on in Settings › Desktop apps
        </button>
        .
      </Quiet>
    );
  }
  return (
    <Quiet testId="last-messages-none" state={chat.status}>
      {NONE[chat.status] ?? "Its transcript holds no messages yet."}
    </Quiet>
  );
}

/** The question an asking turn ends on: its reply's last paragraph with a
 * "?", else its last paragraph, in plain words. Only the reply (0.3.4): a
 * "?" in the notes written while it worked isn't what it asks. */
export function questionOf(turn: TurnInfo | undefined): string | null {
  if (!turn || turn.speaker !== "assistant") return null;
  const { reply } = foldOf(turn);
  const text = reply ? reply.text : turn.blocks.filter((block) => block.kind === "text").map((block) => block.text).join("\n\n");
  const paragraphs = text
    .replace(/```[\s\S]*?```/g, "")
    .split(/\n\s*\n/)
    .map((paragraph) => plainWords(paragraph))
    .filter(Boolean);
  const asked = [...paragraphs].reverse().find((paragraph) => paragraph.includes("?"));
  return asked ?? paragraphs[paragraphs.length - 1] ?? null;
}

function Bottom({ chat, model, speaker, onGoTo }: { chat: FeedState; model: SessionModel; speaker: string; onGoTo?: () => void }): JSX.Element | null {
  if (!model.live) return null;
  const place = model.places[0];
  const app = appWords(place, model.tool);
  // "Answer it" only while it asks (the backlog: a working session's
  // page said "Answer it in ChatGPT" with nothing to answer).
  const answer = app ? (
    <p className={META} data-testid="last-messages-answer">
      {model.state.asks ? `Answer it in ${app}. ` : ""}Go to it brings {place?.label || app} to the front
      {picksInSidebar(place, model.tool) ? "; if another session is showing there, pick this one in its sidebar." : "."}
    </p>
  ) : null;
  const last = chat.turns[chat.turns.length - 1];
  const question = model.state.asks ? questionOf(last) : null;
  if (model.state.asks) {
    return (
      <div className="border-t border-warning/30 bg-warning-soft px-5 py-4" data-testid="last-messages-asks">
        <div className="mx-auto flex w-full max-w-[760px] flex-wrap items-start gap-x-4 gap-y-3">
          <div className="flex min-w-0 flex-1 basis-72 flex-col gap-1">
            <p className="text-meta font-semibold text-warning">{speaker} asks you</p>
            {question && (
              <p className="line-clamp-3 text-body leading-[1.5] text-ink" data-testid="last-messages-question">
                {question}
              </p>
            )}
            {answer}
          </div>
          {onGoTo && (
            <Button size="md" variant="primary" onClick={onGoTo} data-testid="last-messages-go-to" className="shrink-0">
              Go to it
              <IconGoTo width={14} height={14} aria-hidden="true" />
            </Button>
          )}
        </div>
      </div>
    );
  }
  return (
    <div className="border-t border-hairline px-5 py-3" data-testid={model.state.working ? "last-messages-working" : "last-messages-open"}>
      <div className="mx-auto flex w-full max-w-[760px] flex-col gap-1">
        {model.state.working && (
          <p className="inline-flex items-center gap-2 text-body font-medium text-live" role="status">
            <span aria-hidden="true" className="inline-flex gap-1">
              {[0, 1, 2].map((dot) => (
                <span key={dot} className="size-1.5 rounded-full bg-current motion-safe:animate-pulse" style={{ animationDelay: `${dot * 200}ms` }} />
              ))}
            </span>
            {speaker} is working
          </p>
        )}
        {answer}
      </div>
    </div>
  );
}
