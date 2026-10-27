/**
 * AJ-E1520 (0.2) — a session page's Subagents tab: every subagent the
 * session started (and theirs), from its tool's own records, read-only.
 * The same tab for a session AI Juggler runs and one another app runs.
 *
 * Rows in the task row's shape: its type and task first (the whole task
 * on hover), its status as a word in a pill (never colour alone), when it
 * started and how long it took, and whether it ran in the background;
 * the ones it started under it, indented. Newest first, a page at a time,
 * a skeleton at once. A row whose conversation the tool keeps opens it in
 * the session page's own chat view, with a way back. "Running" is said
 * only while the session is open; once it has ended, a subagent its
 * records never saw end is "unknown" (D-06).
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Pill, type PillTone } from "../../components/Pill";
import { formatDuration } from "../../components/indicator";
import { usePagedList } from "../../lib/paged";
import { ago } from "../../lib/time-words";
import { subagentsConversation, subagentsList, type SubagentInfo, type SubagentsInfo } from "../../lib/ipc/subagents";
import type { PageRequest } from "../../types/generated/PageRequest";
import { LastMessages } from "./LastMessages";
import type { ChatReader } from "./useLastMessages";
import type { SessionModel } from "./model";

/** How often the list is read again while something runs. */
export const SUBAGENTS_FOLLOW_MS = 4_000;

const STATUS_WORDS: Record<string, string> = {
  running: "Running",
  done: "Done",
  failed: "Failed",
  stopped: "Stopped",
  unknown: "Unknown",
};

const STATUS_TONE: Record<string, PillTone> = {
  running: "live",
  done: "success",
  failed: "danger",
  stopped: "warning",
  unknown: "neutral",
};

/** What the status word means, on hover. */
const STATUS_HINT: Record<string, string> = {
  running: "Still working: nothing in its records says it ended.",
  done: "It finished.",
  failed: "It ended with an error.",
  stopped: "It was stopped before it finished.",
  unknown: "Its records don't say how it ended.",
};

type Meta = Pick<SubagentsInfo, "running" | "reading" | "live" | "conversations_note" | "state" | "reason">;

/** `took 4m`, `running 2m`, or nothing when the times aren't recorded. */
export function durationWords(item: SubagentInfo, now: number): string | null {
  if (item.started_ms === null) return null;
  if (item.ended_ms !== null) return `took ${formatDuration(item.ended_ms - item.started_ms)}`;
  if (item.status === "running") return `running ${formatDuration(now - item.started_ms)}`;
  return null;
}

export function SubagentsTab({ model, visible, now }: { model: SessionModel; visible: boolean; now: number }): JSX.Element {
  const [open, setOpen] = useState<SubagentInfo | null>(null);
  const scrollKeep = useRef(0);
  const back = useCallback(() => setOpen(null), []);
  if (open) {
    return <SubagentChat model={model} item={open} onBack={back} />;
  }
  return <SubagentsList model={model} visible={visible} now={now} onOpen={setOpen} keep={scrollKeep} />;
}

function SubagentsList({
  model,
  visible,
  now,
  onOpen,
  keep,
}: {
  model: SessionModel;
  visible: boolean;
  now: number;
  onOpen: (item: SubagentInfo) => void;
  keep: React.MutableRefObject<number>;
}): JSX.Element {
  const conversationId = model.conversationId;
  const [meta, setMeta] = useState<Meta | null>(null);
  const fetchPage = useCallback(
    (page: PageRequest) =>
      conversationId === null
        ? Promise.resolve({ items: [], next_cursor: null, total: 0 })
        : subagentsList(model.profileId, conversationId, page).then((info) => {
            setMeta(info);
            return info.page;
          }),
    [model.profileId, conversationId],
  );
  const key = conversationId ? `subagents:${model.profileId}:${conversationId}` : null;
  const list = usePagedList<SubagentInfo>(key, fetchPage, { keyOf: (item) => item.id });
  // Followed while something runs (or a long session is still being
  // read), the page shown and the window in view.
  const following = visible && !!meta && ((meta.live && meta.running > 0) || meta.reading);
  const refresh = list.refresh;
  useEffect(() => {
    if (!following) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState !== "hidden") refresh();
    }, SUBAGENTS_FOLLOW_MS);
    return () => window.clearInterval(timer);
  }, [following, refresh]);
  const scroller = useRef<HTMLDivElement | null>(null);
  // Back from a conversation: where you were.
  useEffect(() => {
    if (scroller.current && keep.current) scroller.current.scrollTop = keep.current;
  }, [keep, list.loading]);
  const total = list.total ?? list.items.length;
  const summary = useMemo(() => {
    if (!meta) return null;
    const all = `${total} ${total === 1 ? "subagent" : "subagents"}`;
    return meta.running > 0 ? `${all} · ${meta.running} running` : all;
  }, [meta, total]);
  return (
    <div className="flex w-full min-w-0 max-w-[1080px] flex-col gap-3" data-testid="session-subagents">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h3 className="text-section font-semibold text-ink">Subagents</h3>
        {summary && (
          <span className="text-meta text-muted" data-testid="subagents-summary">
            {summary}
          </span>
        )}
      </div>
      {meta?.reading && (
        <p className="text-meta text-muted" role="status" data-testid="subagents-reading">
          Still reading a long session: more may appear.
        </p>
      )}
      {meta?.conversations_note && (
        <p className="text-meta text-muted" data-testid="subagents-note">
          {meta.conversations_note}
        </p>
      )}
      {list.loading ? (
        <div className="flex flex-col gap-1.5" role="status" aria-label="Loading its subagents" data-testid="subagents-loading">
          {[0, 1, 2].map((row) => (
            <span key={row} className="h-14 rounded-lg bg-surface-3 motion-safe:animate-pulse" />
          ))}
        </div>
      ) : meta && meta.state !== "listed" ? (
        <p className="text-body text-muted" data-testid="subagents-unlisted" data-state={meta.state}>
          {meta.reason ?? "There's no record of this session's subagents on this Mac."}
        </p>
      ) : list.items.length === 0 ? (
        <p className="text-body text-muted" data-testid="subagents-empty">
          It hasn't started any subagents.
        </p>
      ) : (
        <div
          ref={scroller}
          onScroll={() => {
            keep.current = scroller.current?.scrollTop ?? 0;
          }}
          className="max-h-[calc(100vh-18rem)] min-h-0 overflow-y-auto"
        >
          <ul className="flex flex-col gap-0.5" aria-label="Subagents">
            {list.items.map((item) => (
              <SubagentRow key={item.id} item={item} now={now} onOpen={() => onOpen(item)} />
            ))}
          </ul>
        </div>
      )}
      {list.hasMore && (
        <Button size="sm" variant="ghost" onClick={list.loadMore} disabled={list.loadingMore} className="self-start" data-testid="subagents-more">
          {list.loadingMore ? "Reading more subagents…" : "Show more subagents"}
        </Button>
      )}
    </div>
  );
}

export function SubagentRow({ item, now, onOpen }: { item: SubagentInfo; now: number; onOpen: () => void }): JSX.Element {
  const task = item.task ?? (item.kind ? `A ${item.kind} subagent` : "A subagent");
  const duration = durationWords(item, now);
  const indent = Math.max(0, item.depth - 1);
  const body = (
    <>
      <span className="flex min-w-0 items-baseline gap-2">
        {item.kind && <span className="shrink-0 text-label font-semibold uppercase tracking-wider text-muted">{item.kind}</span>}
        <span className="min-w-0 flex-1 truncate text-body font-medium text-ink" title={item.task ?? undefined} data-testid="subagent-task">
          {task}
        </span>
        {item.started_ms !== null && (
          <time dateTime={new Date(item.started_ms).toISOString()} title={new Date(item.started_ms).toLocaleString()} className="shrink-0 text-label text-muted">
            {ago(item.started_ms, now)}
          </time>
        )}
      </span>
      <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
        <span title={STATUS_HINT[item.status]} className="inline-flex">
          <Pill tone={STATUS_TONE[item.status] ?? "neutral"} testId="subagent-status">
            {STATUS_WORDS[item.status] ?? item.status}
          </Pill>
        </span>
        {duration && <span className="text-meta text-muted">{duration}</span>}
        {item.background === true && (
          <span className="text-meta text-muted" data-testid="subagent-background">
            in the background
          </span>
        )}
        {indent > 0 && <span className="sr-only">started by another subagent</span>}
        {item.has_conversation && <span className="ml-auto text-meta text-muted">Its conversation ›</span>}
      </span>
    </>
  );
  const style = indent ? { paddingLeft: `${12 + indent * 18}px` } : undefined;
  const shape = "flex w-full min-w-0 flex-col gap-1 rounded-lg px-3 py-2.5 text-left";
  return (
    <li data-testid="subagent-row" data-status={item.status} data-depth={item.depth}>
      {item.has_conversation ? (
        <button
          type="button"
          onClick={onOpen}
          style={style}
          className={shape + " outline-none transition-colors hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60"}
        >
          {body}
        </button>
      ) : (
        <div style={style} className={shape}>
          {body}
        </div>
      )}
    </li>
  );
}

/** A subagent's conversation, in the session page's own chat view. */
function SubagentChat({ model, item, onBack }: { model: SessionModel; item: SubagentInfo; onBack: () => void }): JSX.Element {
  const conversationId = model.conversationId ?? "";
  const running = item.status === "running";
  const reader = useMemo<ChatReader>(
    () => ({ key: `subagent:${item.id}`, read: (ask) => subagentsConversation(model.profileId, conversationId, item.id, ask) }),
    [model.profileId, conversationId, item.id],
  );
  // Its own state: followed while it runs; it asks no one.
  const sub = useMemo<SessionModel>(
    () => ({ ...model, live: model.live && running, state: { ...model.state, asks: false, working: running } }),
    [model, running],
  );
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack]);
  return (
    <div className="flex h-[calc(100vh-16rem)] min-h-[24rem] w-full min-w-0 max-w-[1080px] flex-col" data-testid="subagent-conversation">
      <LastMessages
        model={sub}
        visible
        reader={reader}
        title={item.task ?? item.kind ?? "A subagent"}
        asker="The session asked"
        head={
          <Button size="sm" variant="ghost" onClick={onBack} data-testid="subagent-back" aria-label="Back to the subagents">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M15 6l-6 6 6 6" />
            </svg>
            Subagents
          </Button>
        }
      />
    </div>
  );
}
