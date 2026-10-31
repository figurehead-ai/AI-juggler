/**
 * E18 (AJ-E1802) — a pool's messages, or a session's: its threads on the
 * left (newest first, a page at a time, searchable), the open thread as a
 * conversation on the right, and a new message. A pool whose messages are
 * paused (by you, or because two agents kept answering each other or one
 * kept sending past its limit) says so at the top, with Resume.
 *
 * The same rows and fold patterns as the rest of the app; nothing here
 * reaches a session but your own Deliver.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { ago } from "../../lib/time-words";
import {
  FOCUS_THREAD,
  messagesSearch,
  messagesSetPool,
  messagesThreads,
  onMessagesChanged,
  takeFocusedThread,
  type MessageInfo,
  type ThreadInfo,
  type ThreadsPage,
} from "../../lib/ipc/messages";
import { Compose, type Member } from "./Compose";
import { ThreadView } from "./ThreadView";
import { THREAD_WORDS } from "./words";

function ThreadRow({ thread, selected, now, onOpen }: { thread: ThreadInfo; selected: boolean; now: number; onOpen: () => void }): JSX.Element {
  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        aria-current={selected ? "true" : undefined}
        className={
          "flex w-full min-w-0 flex-col gap-0.5 rounded-lg px-3 py-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (selected ? "bg-surface-3" : "hover:bg-surface-2")
        }
        data-testid="thread-row"
        data-state={thread.state}
      >
        <span className="flex min-w-0 items-baseline gap-2">
          <span className={"min-w-0 flex-1 truncate text-body text-ink " + (thread.unread > 0 ? "font-semibold" : "font-medium")}>{thread.subject}</span>
          <span className="shrink-0 text-label text-muted">{ago(thread.updated_at_ms, now)}</span>
        </span>
        <span className="flex min-w-0 items-center gap-2">
          <span className="min-w-0 flex-1 truncate text-meta text-muted">
            {thread.last_from}: {thread.last_line}
          </span>
          {thread.state !== "open" && <span className="shrink-0 text-label font-medium text-warning">{THREAD_WORDS[thread.state]}</span>}
          {thread.unread > 0 && (
            <span className="shrink-0 rounded-full bg-accent px-1.5 text-label font-semibold text-white" aria-label={`${thread.unread} new for you`}>
              {thread.unread}
            </span>
          )}
        </span>
      </button>
    </li>
  );
}

export function MessagesView({
  poolId,
  sessionId,
  members,
  now,
  composeTo,
}: {
  /** The pool whose messages these are (and where you write). */
  poolId: string;
  /** Only this session's threads (its page). */
  sessionId?: string;
  members: Member[];
  now: number;
  /** A session's page writes to that session only. */
  composeTo?: string;
}): JSX.Element {
  const [page, setPage] = useState<ThreadsPage | null>(null);
  const [more, setMore] = useState<ThreadInfo[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(takeFocusedThread);
  const [writing, setWriting] = useState(false);
  const [query, setQuery] = useState("");
  const [found, setFound] = useState<MessageInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const scope = sessionId ? { sessionId } : { poolId };

  useEffect(() => {
    let alive = true;
    const read = () =>
      void messagesThreads(scope).then(
        (first) => {
          if (!alive) return;
          setPage(first);
          // Pages shown beyond the first stay; their cursor stays too.
          setNext((was) => was ?? first.next);
          setOpen((was) => was ?? first.threads[0]?.id ?? null);
        },
        (cause) => alive && setError(errorText(cause)),
      );
    read();
    const off = onMessagesChanged(read);
    return () => {
      alive = false;
      off();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [poolId, sessionId]);

  // E19: a task's thread, asked for while this tab is already open.
  useEffect(() => {
    const take = () => {
      const thread = takeFocusedThread();
      if (thread) (setOpen(thread), setWriting(false));
    };
    window.addEventListener(FOCUS_THREAD, take);
    return () => window.removeEventListener(FOCUS_THREAD, take);
  }, []);

  useEffect(() => {
    const words = query.trim();
    if (!words) {
      setFound(null);
      return;
    }
    let alive = true;
    const timer = window.setTimeout(() => void messagesSearch(poolId, words).then((hits) => alive && setFound(hits), () => undefined), 200);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [query, poolId]);

  const threads = [...(page?.threads ?? []), ...more.filter((t) => !page?.threads.some((p) => p.id === t.id))];
  const shown = found ? threads.filter((t) => found.some((m) => m.thread_id === t.id)) : threads;
  const paused = page?.paused ?? null;
  return (
    <div className="flex w-full min-w-0 max-w-[1440px] flex-col gap-4" data-testid="messages-view">
      {paused && (
        <div className="flex flex-wrap items-center gap-3 rounded-xl border border-warning/35 bg-warning-soft/60 px-4 py-3" role="status" data-testid="messages-paused">
          <p className="min-w-0 flex-1 text-body text-ink">
            <span className="font-semibold">Messages in this pool are paused</span> — {paused.reason}. No agent can send or be woken until you resume them.
          </p>
          <Button size="sm" variant="primary" onClick={() => void messagesSetPool(poolId, false).then(setPage, (cause) => setError(errorText(cause)))} data-testid="messages-resume">
            Resume messages
          </Button>
        </div>
      )}
      <div className="grid min-w-0 gap-4 lg:grid-cols-[minmax(280px,360px)_minmax(0,1fr)]">
        <aside className="flex min-w-0 flex-col gap-2" aria-label="Threads">
          <div className="flex items-center gap-2">
            <input
              type="search"
              aria-label="Search messages"
              placeholder="Search messages"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              className="h-9 min-w-0 flex-1 rounded-lg border border-hairline bg-surface-2 px-3 text-meta text-ink outline-none focus:border-accent/60"
            />
            <Button size="md" variant="primary" onClick={() => setWriting(true)} data-testid="messages-new">
              New message
            </Button>
          </div>
          {page === null ? (
            <div className="flex flex-col gap-2" role="status" aria-label="Loading the threads">
              {[0, 1, 2].map((n) => (
                <span key={n} className="h-14 rounded-lg bg-skeleton motion-safe:animate-pulse" />
              ))}
            </div>
          ) : shown.length === 0 ? (
            <p className="rounded-lg border border-dashed border-hairline px-3 py-4 text-meta text-ink-2" data-testid="messages-empty">
              {found ? "No message has those words." : sessionId ? "No messages to or from this session yet." : "No messages yet. When agents here write to each other or to you, their threads show here."}
            </p>
          ) : (
            <ul className="flex flex-col gap-0.5" data-testid="thread-list">
              {shown.map((thread) => (
                <ThreadRow key={thread.id} thread={thread} now={now} selected={!writing && thread.id === open} onOpen={() => (setOpen(thread.id), setWriting(false))} />
              ))}
            </ul>
          )}
          {next && !found && (
            <Button size="sm" variant="ghost" onClick={() => void messagesThreads(scope, next).then((older) => (setMore((was) => [...was, ...older.threads]), setNext(older.next)))} data-testid="threads-more">
              Show more threads
            </Button>
          )}
          {!paused && !sessionId && (
            <button type="button" className="self-start text-label text-muted underline decoration-hairline underline-offset-2 hover:text-ink" onClick={() => void messagesSetPool(poolId, true).then(setPage)} data-testid="messages-pause">
              Pause this pool's messages
            </button>
          )}
          {error && (
            <p role="alert" className="text-meta text-danger">
              {error}
            </p>
          )}
        </aside>
        <div className="min-w-0 rounded-xl border border-hairline bg-surface p-4">
          {writing || (open === null && page !== null) ? (
            <div className="flex flex-col gap-3">
              <h3 className="text-[16px] font-semibold text-ink">New message</h3>
              <Compose
                poolId={poolId}
                members={members}
                to={composeTo}
                onSent={(sent) => {
                  setWriting(false);
                  setOpen(sent.thread.id);
                }}
                onCancel={open ? () => setWriting(false) : undefined}
              />
            </div>
          ) : open ? (
            <ThreadView key={open} threadId={open} poolId={poolId} members={members} now={now} />
          ) : null}
        </div>
      </div>
    </div>
  );
}
