/**
 * 0.3.4 — beside a session's conversation, on its left, what the tools'
 * own apps list with it, so one click drills in and Esc comes back:
 *
 * - **Conversation**: the session's own (or its terminal), first and
 *   chosen until another is picked;
 * - **Running**: its subagents running now (as Claude's tasks pane and
 *   Codex's "Active" list them), each opening its conversation here;
 * - **Started from here**: the sessions it started, as its app records
 *   them (the Claude app nests them under it in its sidebar), each shown
 *   here with a way to its own page;
 * - **Done · N**: folded, the latest that ended, then the Subagents tab
 *   for all of them (it stays the full list, with search and filters).
 *
 * Shown only when there's something besides the conversation; a narrow
 * page gets a picker above it instead. Read only while the page is shown,
 * again every 15 s while the session runs.
 */
import { useCallback, useEffect, useMemo, useState, type JSX, type ReactNode } from "react";
import { Button } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { useConversations } from "../../lib/conversations";
import { conversationsStarted, type StartedSessionInfo } from "../../lib/ipc/conversations";
import { subagentsConversation, subagentsList, type SubagentInfo } from "../../lib/ipc/subagents";
import { agoWords } from "../../lib/time-words";
import { showTab } from "../PageShell";
import { kindWords, titleOf } from "../subagents/words";
import { LastMessages } from "./LastMessages";
import { fromConversation, type SessionModel } from "./model";
import type { ChatReader } from "./useLastMessages";
import { useHeardSubagentCounts } from "./useSubagents";

/** How many ended subagents the folded list shows. */
export const DONE_SHOWN = 8;
/** How often the lists are read again while the session runs. */
const EVERY_MS = 15_000;

type Picked = { kind: "own" } | { kind: "subagent"; agent: SubagentInfo } | { kind: "started"; session: StartedSessionInfo };

/** The session's subagents running now and the latest ended, and the
 * sessions it started. */
export function useSwitcherLists(model: SessionModel, visible: boolean): { running: SubagentInfo[]; done: SubagentInfo[]; started: StartedSessionInfo[]; total: number } {
  const counts = useHeardSubagentCounts(model.profileId, model.conversationId);
  const [lists, setLists] = useState<{ running: SubagentInfo[]; done: SubagentInfo[]; started: StartedSessionInfo[] }>({ running: [], done: [], started: [] });
  const total = counts?.total ?? 0;
  const conversationId = model.conversationId;
  useEffect(() => {
    if (!visible || !conversationId) return;
    let alive = true;
    const agents = (rows: { agent: SubagentInfo | null; context: boolean }[]) => rows.flatMap((row) => (row.agent && !row.context ? [row.agent] : []));
    const read = () => {
      const subagents =
        total > 0
          ? Promise.all([
              subagentsList(model.profileId, conversationId, { cursor: null, limit: 30 }, { view: "running", search: null, kind: null, sort: "newest", open: [], closed: [] }),
              subagentsList(model.profileId, conversationId, { cursor: null, limit: DONE_SHOWN }, { view: "done", search: null, kind: null, sort: "newest", open: [], closed: [] }),
            ]).then(([running, done]) => ({ running: agents(running.page.items), done: agents(done.page.items) }))
          : Promise.resolve({ running: [], done: [] });
      void Promise.all([subagents, conversationsStarted(model.tool, conversationId).catch(() => [])]).then(
        ([found, started]) => alive && setLists({ ...found, started }),
        () => undefined,
      );
    };
    read();
    const timer = model.live ? window.setInterval(read, EVERY_MS) : null;
    return () => {
      alive = false;
      if (timer !== null) window.clearInterval(timer);
    };
  }, [visible, conversationId, model.profileId, model.tool, model.live, total]);
  return { ...lists, total };
}

/** `ownWords`: what the session's own pane is ("Its terminal", "Its
 * conversation"), under "This session" in the list. */
export function ConversationSwitcher({ model, visible, own, ownWords = "Its conversation" }: { model: SessionModel; visible: boolean; own: ReactNode; ownWords?: string }): JSX.Element {
  const { running, done, started, total } = useSwitcherLists(model, visible);
  const [picked, setPicked] = useState<Picked>({ kind: "own" });
  const back = useCallback(() => setPicked({ kind: "own" }), []);
  useEffect(() => {
    if (picked.kind === "own") return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) back();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [picked.kind, back]);
  const any = running.length + done.length + started.length > 0;
  // The session's own pane stays mounted while another is shown (hidden):
  // a terminal is never rebuilt and replayed, a conversation keeps where it
  // was scrolled, and Back is instant.
  const ownPane = (
    <div hidden={picked.kind !== "own"} className="flex min-h-0 min-w-0 flex-1 flex-col" data-testid="switcher-own-pane">
      {own}
    </div>
  );
  // One tree whether or not there's a list (its slot stays, empty), so the
  // own pane is never remounted when the list arrives or goes.
  return (
    <div className="@container flex min-h-0 flex-1 flex-col" data-testid={any ? "conversation-switcher" : undefined}>
      <div className={"grid min-h-0 flex-1 grid-cols-1 gap-3 " + (any ? "@xl:grid-cols-[12rem_minmax(0,1fr)]" : "")}>
        {any ? <SwitcherList model={model} ownWords={ownWords} picked={picked} onPick={setPicked} running={running} done={done} started={started} total={total} /> : null}
        <div className={"flex min-w-0 flex-col " + (any ? "min-h-[28rem] @xl:min-h-0" : "min-h-0 flex-1")}>
          {ownPane}
          {picked.kind === "subagent" && <SubagentPane model={model} agent={picked.agent} onBack={back} />}
          {picked.kind === "started" && <StartedPane session={picked.session} onBack={back} />}
        </div>
      </div>
    </div>
  );
}

function SwitcherList({
  model,
  ownWords,
  picked,
  onPick,
  running,
  done,
  started,
  total,
}: {
  model: SessionModel;
  ownWords: string;
  picked: Picked;
  onPick: (next: Picked) => void;
  running: SubagentInfo[];
  done: SubagentInfo[];
  started: StartedSessionInfo[];
  total: number;
}): JSX.Element {
  const [doneOpen, setDoneOpen] = useState(false);
  const now = Date.now();
  const isPicked = (next: Picked): boolean =>
    (next.kind === "own" && picked.kind === "own") ||
    (next.kind === "subagent" && picked.kind === "subagent" && next.agent.id === picked.agent.id) ||
    (next.kind === "started" && picked.kind === "started" && next.session.conversation_id === picked.session.conversation_id);
  const ended = Math.max(0, total - running.length);
  return (
    <nav aria-label="Conversations in this session" className="flex max-h-40 min-h-0 flex-col gap-3 overflow-y-auto rounded-card border border-hairline p-1.5 @xl:max-h-none @xl:rounded-none @xl:border-0 @xl:p-0 @xl:pr-1" data-testid="switcher-list">
      <ul className="flex flex-col gap-0.5">
        <Item picked={isPicked({ kind: "own" })} onClick={() => onPick({ kind: "own" })} testId="switcher-own" dot={model.state.working ? "live" : null} title="This session" meta={ownWords} />
      </ul>
      {running.length > 0 && (
        <Section label={`Running · ${running.length}`} testId="switcher-running">
          {running.map((agent) => (
            <Item key={agent.id} picked={isPicked({ kind: "subagent", agent })} onClick={() => onPick({ kind: "subagent", agent })} dot="live" title={titleOf(agent)} meta={kindWords(agent.kind) ?? "Subagent"} testId="switcher-subagent" />
          ))}
        </Section>
      )}
      {started.length > 0 && (
        <Section label={`Started from here · ${started.length}`} testId="switcher-started">
          {started.map((session) => (
            <Item
              key={session.conversation_id}
              picked={isPicked({ kind: "started", session })}
              onClick={() => onPick({ kind: "started", session })}
              dot={null}
              title={session.title || "A session"}
              meta={session.last_activity_ms ? agoWords(session.last_activity_ms, now) : "A session"}
              testId="switcher-started-session"
            />
          ))}
        </Section>
      )}
      {ended > 0 && (
        <section className="flex flex-col gap-1" data-testid="switcher-done">
          <button
            type="button"
            aria-expanded={doneOpen}
            onClick={() => setDoneOpen((open) => !open)}
            className="flex items-center gap-1.5 rounded px-1.5 py-1 text-left text-label font-semibold text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
            data-testid="switcher-done-toggle"
          >
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={"shrink-0 transition-transform " + (doneOpen ? "rotate-90" : "")}>
              <path d="M9 6l6 6-6 6" />
            </svg>
            Done · {ended}
          </button>
          {doneOpen && (
            <ul className="flex flex-col gap-0.5">
              {done.map((agent) => (
                <Item key={agent.id} picked={isPicked({ kind: "subagent", agent })} onClick={() => onPick({ kind: "subagent", agent })} dot={agent.status === "failed" ? "danger" : null} title={titleOf(agent)} meta={agent.status === "failed" ? "Failed" : (kindWords(agent.kind) ?? "Subagent")} testId="switcher-subagent" />
              ))}
              <li>
                <button type="button" onClick={() => showTab("subagents")} className="w-full rounded px-2 py-1 text-left text-label text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="switcher-all">
                  All {total.toLocaleString()} in Subagents ›
                </button>
              </li>
            </ul>
          )}
        </section>
      )}
    </nav>
  );
}

function Section({ label, testId, children }: { label: string; testId: string; children: ReactNode }): JSX.Element {
  return (
    <section className="flex flex-col gap-1" data-testid={testId}>
      <h3 className="px-1.5 text-label font-semibold text-muted">{label}</h3>
      <ul className="flex flex-col gap-0.5">{children}</ul>
    </section>
  );
}

const DOTS = { live: "bg-live motion-safe:animate-pulse", danger: "bg-danger" } as const;

function Item({ picked, onClick, dot, title, meta, testId }: { picked: boolean; onClick: () => void; dot: keyof typeof DOTS | null; title: string; meta: ReactNode; testId: string }): JSX.Element {
  return (
    <li>
      <button
        type="button"
        aria-current={picked ? "true" : undefined}
        onClick={onClick}
        title={title}
        className={
          "flex w-full min-w-0 items-start gap-2 rounded-lg px-2 py-1.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (picked ? "bg-surface-2 text-ink" : "text-ink-2 hover:bg-surface-2/60 hover:text-ink")
        }
        data-testid={testId}
      >
        <span aria-hidden="true" className={"mt-[7px] size-1.5 shrink-0 rounded-full " + (dot ? DOTS[dot] : "bg-transparent")} />
        <span className="flex min-w-0 flex-col">
          <span className="line-clamp-2 break-words text-meta font-medium leading-[1.35]">{title}</span>
          <span className="flex items-center gap-1 text-label text-muted">{meta}</span>
        </span>
      </button>
    </li>
  );
}

function BackButton({ onBack, label }: { onBack: () => void; label: string }): JSX.Element {
  return (
    <Button size="sm" variant="ghost" onClick={onBack} data-testid="switcher-back" aria-label={label} aria-keyshortcuts="Escape">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M15 6l-6 6 6 6" />
      </svg>
      Back
    </Button>
  );
}

/** A subagent's conversation in the session's own chat view. */
function SubagentPane({ model, agent, onBack }: { model: SessionModel; agent: SubagentInfo; onBack: () => void }): JSX.Element {
  const conversationId = model.conversationId ?? "";
  const running = agent.status === "running";
  const reader = useMemo<ChatReader>(
    () => ({ key: `subagent:${agent.id}`, read: (ask) => subagentsConversation(model.profileId, conversationId, agent.id, ask) }),
    [model.profileId, conversationId, agent.id],
  );
  // Its own state: followed while it runs; it asks no one.
  const sub = useMemo<SessionModel>(() => ({ ...model, live: model.live && running, state: { ...model.state, asks: false, working: running } }), [model, running]);
  return <LastMessages key={agent.id} model={sub} visible reader={reader} title={titleOf(agent)} asker="The session asked" head={<BackButton onBack={onBack} label="Back to this session's conversation" />} />;
}

/** A session it started: its conversation here, and its own page a click away. */
function StartedPane({ session, onBack }: { session: StartedSessionInfo; onBack: () => void }): JSX.Element {
  const { all } = useConversations();
  const host = useActionHost();
  const found = all.find((c) => c.tool === session.tool && c.id === session.conversation_id);
  const model = useMemo(() => (found ? fromConversation(found) : null), [found]);
  if (!found || !model) {
    return (
      <div className="flex flex-col items-start gap-3 rounded-card border border-hairline bg-surface p-5" data-testid="switcher-started-missing">
        <BackButton onBack={onBack} label="Back to this session's conversation" />
        <p className="text-body text-ink-2">{session.title || "This session"} isn't in this computer's list of conversations yet. It shows here once its tool has written it.</p>
      </div>
    );
  }
  return (
    <LastMessages
      key={found.id}
      model={model}
      visible
      title={session.title || "A session it started"}
      head={
        <span className="flex items-center gap-1">
          <BackButton onBack={onBack} label="Back to this session's conversation" />
          {host.openConversation && (
            <Button size="sm" variant="secondary" onClick={() => host.openConversation?.(found)} data-testid="switcher-open-page">
              Open its page
            </Button>
          )}
        </span>
      }
    />
  );
}
