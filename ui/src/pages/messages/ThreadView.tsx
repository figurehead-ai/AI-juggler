/**
 * E18 (AJ-E1802, AJ-E1803) — one thread, as a conversation: the session
 * page's chat bubbles (yours on the right), each message's kind, its
 * time, and where it is for each recipient. Deliver, where a recipient's
 * session runs in AI Juggler and hasn't got it yet, puts it in as that
 * session's next prompt, between turns (Claude Code in AI Juggler's
 * terminal too, typed at its idle prompt, D-29). Any other terminal is
 * never typed into: it reads its messages on its next turn.
 *
 * A message held for your OK has its answers on it (Allow once, Always
 * allow here from the second time, Deny), the same command the canvas
 * card and Needs you answer with (the rc19 walk: it only said "Waits on
 * your OK").
 *
 * The header pauses, stops or resumes the thread (resuming resets its
 * count of agent messages in a row). Earlier messages load a page at a
 * time; a reply goes at the bottom.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Pill } from "../../components/Pill";
import { errorText } from "../../lib/bridge";
import {
  messagesAnswer,
  messagesDeliver,
  messagesSetThread,
  messagesThread,
  onMessagesChanged,
  useMessageNeeds,
  type MessageInfo,
  type MessageNeedInfo,
  type ThreadPage,
} from "../../lib/ipc/messages";
import { Compose, type Member } from "./Compose";
import { KIND_WORDS, THREAD_WORDS, hopWords, recipientWords, when } from "./words";

type Answer = "once" | "always" | "deny";

/** A held message's answers, one group per ask that holds it. */
function HeldAnswers({ asks, busy, onAnswer }: { asks: MessageNeedInfo[]; busy: boolean; onAnswer: (askId: string, answer: Answer) => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5">
      {asks.map((ask) => (
        <fieldset key={ask.key} className="m-0 flex flex-wrap items-center gap-1.5 border-0 p-0" aria-label={`Answer: ${ask.title}`} data-testid="message-held-answers">
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => ask.ask_id && onAnswer(ask.ask_id, "once")} data-testid="message-allow-once">
            Allow once
          </Button>
          {ask.offer_always && (
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => ask.ask_id && onAnswer(ask.ask_id, "always")} data-testid="message-allow-always">
              Always allow here
            </Button>
          )}
          <Button size="sm" variant="ghost" disabled={busy} onClick={() => ask.ask_id && onAnswer(ask.ask_id, "deny")} data-testid="message-deny">
            Deny
          </Button>
        </fieldset>
      ))}
    </div>
  );
}

function Bubble({ message, now, onDeliver, busy, held, onAnswer }: { message: MessageInfo; now: number; onDeliver: (recipient: string) => void; busy: boolean; held: MessageNeedInfo[]; onAnswer: (askId: string, answer: Answer) => void }): JSX.Element {
  const you = message.from_kind === "user";
  const asks = message.kind === "question" && message.recipients.some((r) => r.name === "You");
  return (
    <li className={"flex max-w-[min(88%,720px)] flex-col gap-1.5 " + (you ? "ml-auto items-end" : "items-start")} data-testid="message-bubble" data-from={message.from_kind}>
      <div className={"flex flex-wrap items-baseline gap-x-2 gap-y-0.5 " + (you ? "justify-end" : "")}>
        <span className={"text-meta font-semibold " + (asks ? "text-warning" : "text-ink")}>{message.from_name}</span>
        {message.kind !== "note" && <span className="text-label text-muted">{KIND_WORDS[message.kind] ?? message.kind}</span>}
        <span className="text-label text-muted">{when(message.at_ms, now)}</span>
        {!you && <span className="text-label text-muted">· an agent</span>}
      </div>
      <p
        className={
          "whitespace-pre-wrap break-words rounded-2xl border px-3.5 py-2.5 text-body leading-relaxed text-ink " +
          (you ? "border-accent/25 bg-accent-soft" : asks ? "border-warning/40 bg-warning-soft" : "border-hairline bg-surface-2")
        }
      >
        {message.body}
      </p>
      {message.redacted > 0 && (
        <span className="text-label text-muted" data-testid="message-redacted">
          {message.redacted === 1 ? "1 secret withheld" : `${message.redacted} secrets withheld`}, as you chose for handovers
        </span>
      )}
      <ul className={"flex flex-col gap-1 " + (you ? "items-end" : "items-start")} aria-label="Where it is for each recipient">
        {message.recipients
          .filter((r) => !(you && r.name === "You"))
          .map((recipient) => (
            <li key={recipient.id} className="flex flex-wrap items-center gap-2 text-label text-muted" data-testid="message-recipient" data-state={recipient.state}>
              {recipient.name === "You" ? (
                <span className={recipient.state === "unread" ? "font-medium text-warning" : ""}>{recipientWords(recipient.state, recipient.name)}</span>
              ) : (
                <span>
                  To <span className="text-ink">{recipient.name}</span> · {recipientWords(recipient.state, recipient.name)}
                </span>
              )}
              {recipient.deliverable && (
                <Button size="sm" variant="secondary" disabled={busy} title={recipient.how} onClick={() => onDeliver(recipient.id)} data-testid="message-deliver">
                  Deliver
                  <span className="sr-only"> to {recipient.name}</span>
                </Button>
              )}
            </li>
          ))}
      </ul>
      {held.length > 0 && <HeldAnswers asks={held} busy={busy} onAnswer={onAnswer} />}
    </li>
  );
}

export function ThreadView({ threadId, poolId, members, now }: { threadId: string; poolId: string; members: Member[]; now: number }): JSX.Element {
  const [page, setPage] = useState<ThreadPage | null>(null);
  const [earlier, setEarlier] = useState<MessageInfo[]>([]);
  const [more, setMore] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The asks that hold a message here (Needs you's list, kept current).
  const needs = useMessageNeeds();

  useEffect(() => {
    let alive = true;
    setPage(null);
    setEarlier([]);
    const read = () =>
      void messagesThread(threadId).then(
        (next) => {
          if (!alive) return;
          setPage(next);
          setMore((was) => was ?? next.earlier);
        },
        (cause) => alive && setError(errorText(cause)),
      );
    setMore(null);
    read();
    const off = onMessagesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [threadId]);

  const act = (run: () => Promise<ThreadPage>) => {
    setBusy(true);
    setError(null);
    void run().then(
      (next) => {
        setBusy(false);
        setPage(next);
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  const answer = (askId: string, choice: Answer) => {
    setBusy(true);
    setError(null);
    void messagesAnswer(askId, choice).then(
      () => setBusy(false),
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  const asksOf = (messageId: string) => needs.filter((need) => need.kind === "ask" && need.ask_id !== null && need.message_id === messageId);
  const loadEarlier = () => {
    if (!more) return;
    void messagesThread(threadId, more).then((older) => {
      setEarlier((was) => [...older.messages, ...was]);
      setMore(older.earlier);
    });
  };

  if (!page) {
    return (
      <div className="flex flex-col gap-3 p-4" role="status" aria-label="Loading the thread">
        <span className="h-12 w-2/3 rounded-2xl bg-skeleton motion-safe:animate-pulse" />
        <span className="ml-auto h-10 w-1/2 rounded-2xl bg-skeleton motion-safe:animate-pulse" />
        {error && <p className="text-meta text-danger">{error}</p>}
      </div>
    );
  }
  const { thread } = page;
  const hops = hopWords(thread.hop, thread.hop_cap);
  const messages = [...earlier, ...page.messages];
  return (
    <section className="flex min-w-0 flex-col gap-4" aria-labelledby={`thread-${thread.id}`} data-testid="thread-view">
      <header className="flex flex-wrap items-start justify-between gap-3 border-b border-hairline/60 pb-3">
        <div className="flex min-w-0 flex-col gap-1">
          <h3 id={`thread-${thread.id}`} className="text-[16px] font-semibold text-ink">
            {thread.subject}
          </h3>
          <p className="text-meta text-ink-2">
            {thread.people.join(", ")}
            {thread.pool_name ? ` · in «${thread.pool_name}»` : ""}
          </p>
          <div className="flex flex-wrap items-center gap-2 pt-0.5">
            {thread.state !== "open" && <Pill tone={thread.state === "stopped" ? "danger" : "warning"}>{THREAD_WORDS[thread.state] ?? thread.state}</Pill>}
            {hops && (
              <Pill tone="neutral" wrap testId="thread-hops">
                {hops}
              </Pill>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          {thread.state === "open" ? (
            <>
              <Button size="sm" variant="secondary" disabled={busy} onClick={() => act(() => messagesSetThread(thread.id, "paused"))} data-testid="thread-pause">
                Pause
              </Button>
              <Button size="sm" variant="ghost" disabled={busy} onClick={() => act(() => messagesSetThread(thread.id, "stopped"))} data-testid="thread-stop">
                Stop
              </Button>
            </>
          ) : (
            <Button size="sm" variant="primary" disabled={busy} onClick={() => act(() => messagesSetThread(thread.id, "open"))} data-testid="thread-resume">
              Resume
            </Button>
          )}
        </div>
      </header>
      {thread.state !== "open" && thread.state_reason && (
        <p className="rounded-lg border border-warning/30 bg-warning-soft/50 px-3 py-2 text-meta text-ink" data-testid="thread-why">
          {THREAD_WORDS[thread.state]}: {thread.state_reason}. {thread.state === "paused" ? "No agent can write here or be woken by it until you resume it." : "Agents can't write here any more."}
        </p>
      )}
      {more && (
        <Button size="sm" variant="ghost" className="self-center" onClick={loadEarlier} data-testid="thread-earlier">
          Show earlier messages
        </Button>
      )}
      <ol className="flex flex-col gap-4" aria-label={`Messages in ${thread.subject}`}>
        {messages.map((message) => (
          <Bubble key={message.id} message={message} now={now} busy={busy} held={asksOf(message.id)} onAnswer={answer} onDeliver={(recipient) => act(() => messagesDeliver(message.id, recipient))} />
        ))}
      </ol>
      <p className="text-label text-muted">An agent's message is its own words: it can't approve anything or change settings.</p>
      {error && (
        <p role="alert" className="text-meta text-danger">
          {error}
        </p>
      )}
      {thread.state !== "stopped" && <Compose poolId={poolId} members={members} threadId={thread.id} onSent={setPage} />}
    </section>
  );
}
