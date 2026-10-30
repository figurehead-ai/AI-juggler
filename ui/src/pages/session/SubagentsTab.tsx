/**
 * AJ-E1520 — a session page's Subagents tab: every subagent the session
 * started (and theirs), from its tool's own records, read-only. The same
 * tab for a session AI Juggler runs and one another app runs.
 *
 * 0.3: what's running, what failed and what it cost first — the grouped
 * tree view (`pages/subagents/SubagentsView`), a subagent's own page
 * (`SubagentDetail`), and from it its conversation in the session page's
 * own chat view. Esc goes back a step, the list where it was scrolled.
 * The Usage tab's "see which ›" opens it sorted by most tokens, a
 * Processes row's "started by subagent" opens that subagent's page
 * (`lib/subagent-links`). "Running" is said only while the session is
 * open; once it has ended, one its records never saw end is "unknown"
 * (D-06).
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { onSubagentsAsk, takeSubagentsAsk } from "../../lib/subagent-links";
import { subagentsConversation, type SubagentInfo, type SubagentRowInfo } from "../../lib/ipc/subagents";
import { SubagentDetail } from "../subagents/SubagentDetail";
import { SubagentsView, type SortId, type ViewMemory } from "../subagents/SubagentsView";
import type { SubagentsSource } from "../subagents/useSubagentRows";
import { titleOf } from "../subagents/words";
import { LastMessages } from "./LastMessages";
import type { ChatReader } from "./useLastMessages";
import type { SessionModel } from "./model";
import { useHeardSubagentCounts } from "./useSubagents";

type Step = { at: "list" } | { at: "detail"; id: string } | { at: "chat"; agent: SubagentInfo; from: string };

/** `title`: the session's own title, as its page's header names it — the
 * first step of a subagent's breadcrumb ("Fix the cart totals ›
 * Subagents › …", D-56). */
export function SubagentsTab({ model, title, visible, now }: { model: SessionModel; title: string; visible: boolean; now: number }): JSX.Element {
  const [step, setStep] = useState<Step>({ at: "list" });
  const [sort, setSort] = useState<SortId>("newest");
  const keep = useRef(0);
  const memory = useRef<ViewMemory>({});
  const counts = useHeardSubagentCounts(model.profileId, model.conversationId);
  // What another tab asked for: an order, or one subagent's page.
  useEffect(() => {
    const take = () => {
      const ask = takeSubagentsAsk();
      if (!ask) return;
      if (ask.sort) {
        memory.current = { sort: ask.sort, view: "all" };
        keep.current = 0;
        setSort(ask.sort);
        setStep({ at: "list" });
      }
      if (ask.subagent) setStep({ at: "detail", id: ask.subagent });
    };
    take();
    return onSubagentsAsk(take);
  }, []);
  const source = useMemo<SubagentsSource | null>(
    () => (model.conversationId ? { kind: "session", profileId: model.profileId, conversationId: model.conversationId } : null),
    [model.profileId, model.conversationId],
  );
  const back = useCallback(() => setStep({ at: "list" }), []);
  const open = useCallback((row: SubagentRowInfo) => row.agent && setStep({ at: "detail", id: row.agent.id }), []);
  if (step.at === "chat") {
    return <SubagentChat model={model} item={step.agent} onBack={() => setStep({ at: "detail", id: step.from })} />;
  }
  if (step.at === "detail" && model.conversationId) {
    return (
      <div className="w-full min-w-0 max-w-[1280px]">
        <SubagentDetail
          target={{ profileId: model.profileId, conversationId: model.conversationId, subagentId: step.id }}
          sessionTitle={title}
          visible={visible}
          onBack={back}
          onOpen={(id) => setStep({ at: "detail", id })}
          onConversation={(agent) => setStep({ at: "chat", agent, from: agent.id })}
        />
      </div>
    );
  }
  return (
    <div className="w-full min-w-0 max-w-[1280px]" data-testid="session-subagents">
      <SubagentsView
        key={sort}
        source={source}
        visible={visible}
        now={now}
        onOpen={open}
        firstView={sort === "newest" && (counts?.running ?? 0) > 0 ? "running" : "all"}
        firstSort={sort}
        keep={keep}
        memory={memory}
      />
    </div>
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
        title={titleOf(item)}
        asker="The session asked"
        head={
          <Button size="sm" variant="ghost" onClick={onBack} data-testid="subagent-back" aria-label="Back to the subagent">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M15 6l-6 6 6 6" />
            </svg>
            Back
          </Button>
        }
      />
    </div>
  );
}
