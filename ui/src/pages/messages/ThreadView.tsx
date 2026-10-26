/**
 * E18 (AJ-E1802, AJ-E1803) — one thread, as a conversation: the session
 * page's chat bubbles (yours on the right), each message's kind, its
 * time, and where it is for each recipient. Deliver, where a recipient's
 * session runs in AI Juggler and hasn't got it yet, puts it in as that
 * session's next prompt, between turns. A session in a terminal is never
 * typed into: it reads its messages itself.
 *
 * The header pauses, stops or resumes the thread (resuming resets its
 * count of agent messages in a row). Earlier messages load a page at a
 * time; a reply goes at the bottom.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import {
  messagesDeliver,
  messagesSetThread,
  messagesThread,
  onMessagesChanged,
  type MessageInfo,
  type ThreadPage,
} from "../../lib/ipc/messages";
import { Compose, type Member } from "./Compose";
import { KIND_WORDS, THREAD_WORDS, hopWords, recipientWords, when } from "./words";

function Pill({ tone, children }: { tone: "neutral" | "warning" | "danger" | "accent"; children: string }): JSX.Element {
  const tones = {
    neutral: "border-hairline bg-surface-2 text-muted",
    warning: "border-warning/30 bg-warning-soft text-warning",
    danger: "border-danger/30 bg-danger-soft text-danger",
    accent: "border-accent/25 bg-accent-soft text-accent-hi",
  };
  return <span className={`inline-flex shrink-0 items-center rounded-full border px-2 py-0.5 text-label font-medium ${tones[tone]}`}>{children}</span>;
}

function Bubble({ message, now, onDeliver, busy }: { message: MessageInfo; now: number; onDeliver: (recipient: string) => void; busy: boolean }): JSX.Element {
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
          {message.redacted === 1 ? "1 secret withheld" : `${message.redacted} secrets withheld`}, as you chose for packs
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
    </li>
  );
}

export function ThreadView({ threadId, poolId, members, now }: { threadId: string; poolId: string; members: Member[]; now: number }): JSX.Element {
  const [page, setPage] = useState<ThreadPage | null>(null);
  const [earlier, setEarlier] = useState<MessageInfo[]>([]);
  const [more, setMore] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
        <span className="h-12 w-2/3 rounded-2xl bg-surface-3 motion-safe:animate-pulse" />
        <span className="ml-auto h-10 w-1/2 rounded-2xl bg-surface-3 motion-safe:animate-pulse" />
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
          <p className="text-meta text-muted">
            {thread.people.join(", ")}
            {thread.pool_name ? ` · in «${thread.pool_name}»` : ""}
          </p>
          <div className="flex flex-wrap items-center gap-2 pt-0.5">
            {thread.state !== "open" && <Pill tone={thread.state === "stopped" ? "danger" : "warning"}>{THREAD_WORDS[thread.state] ?? thread.state}</Pill>}
            {hops && <Pill tone="neutral">{hops}</Pill>}
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
          <Bubble key={message.id} message={message} now={now} busy={busy} onDeliver={(recipient) => act(() => messagesDeliver(message.id, recipient))} />
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
