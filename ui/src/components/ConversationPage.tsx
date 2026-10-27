/**
 * AJ-E1529, AJ-E2108 — the page of a session another app runs (or one
 * AI Juggler's index found in the tools' files): where it runs, its
 * conversation, and Go to it. D-08: AI Juggler never stops or tidies what
 * isn't its own; "Continue in AI Juggler" resumes it as a session of its
 * own on the same profile, on the user's click, and Continue on another
 * account opens the switch screen from it (AJ-E2108: it stays as it is,
 * and continues as a new session here). AJ-E1533: the same page as every
 * session's, its project as the crumb.
 *
 * Steady while the list refreshes under it: its tabs are always the same
 * two (Usage says when nothing is measured, it never comes and goes), the
 * row last seen stays on screen when one refresh misses it, and the page
 * says it is gone only once it has stayed missing for GONE_MS.
 */
import { ProcessesView } from "../screens/processes/ProcessesView";
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { PageHeader, PageShell, type Crumb } from "../pages/PageShell";
import { pageTabs } from "../pages/registry";
import { conversationTitle, useConversations } from "../lib/conversations";
import { useSubscriptions, accountOfProfile } from "../lib/subscriptions";
import { lazyNamed } from "../lib/lazy";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { ProjectInfo } from "../lib/bridge";
import { GoToButton, SessionChips, SessionMark } from "../pages/session/SessionHead";
import { fromConversation, type SessionModel } from "../pages/session/model";

const SessionOverview = lazyNamed(() => import("../pages/session/SessionOverview"), "SessionOverview");
const UsagePanel = lazyNamed(() => import("../pages/session/UsagePanel"), "UsagePanel");

/** How long a row may be missing from the list before the page says it's
 * gone: a refresh that briefly misses it never flashes "gone". */
export const GONE_MS = 10_000;

/** The row for this page: the list's, else the last one seen while it is
 * missing; `gone` once it has been missing for GONE_MS. */
function useSteadyRow(all: ConversationInfo[], accountId: string, id: string): { row: ConversationInfo | null; gone: boolean } {
  const found = all.find((row) => row.account_id === accountId && row.id === id) ?? null;
  const key = `${accountId}\u0000${id}`;
  const last = useRef<{ key: string; row: ConversationInfo } | null>(null);
  if (found) last.current = { key, row: found };
  else if (last.current?.key !== key) last.current = null;
  const [gone, setGone] = useState<string | null>(null);
  const missing = found === null;
  useEffect(() => {
    if (!missing) {
      setGone(null);
      return;
    }
    const timer = window.setTimeout(() => setGone(key), GONE_MS);
    return () => window.clearTimeout(timer);
  }, [missing, key]);
  return { row: found ?? last.current?.row ?? null, gone: missing && gone === key };
}

/** Opens the switch screen from a session (its id, or the key of one
 * another app runs) to a profile's launch settings. */
export type OpenSwitch = (source: string, targetProfileId: string, targetLabel: string) => void;

export function ConversationPage({
  accountId,
  id,
  now,
  projects = [],
  onSwitch,
}: {
  accountId: string;
  id: string;
  now: number;
  projects?: ProjectInfo[];
  onSwitch?: OpenSwitch;
}): JSX.Element {
  const { all } = useConversations();
  const { row, gone } = useSteadyRow(all, accountId, id);
  if (gone) {
    return (
      <Card className="m-6 p-6 text-body text-muted" data-testid="conversation-gone">
        This conversation isn't there any more.
      </Card>
    );
  }
  if (!row) {
    return (
      <div className="m-6 flex flex-col gap-3" role="status" aria-label="Loading the session" data-testid="conversation-loading">
        <span className="h-12 w-2/3 rounded-xl bg-surface-3 motion-safe:animate-pulse" />
        <span className="h-64 rounded-xl bg-surface-3 motion-safe:animate-pulse" />
      </div>
    );
  }
  return <ConversationView conversation={row} now={now} projects={projects} onSwitch={onSwitch} />;
}

/** The page's model, the same object while nothing it shows has changed
 * (a list refresh hands over new rows with the same values). */
function useStableModel(conversation: ConversationInfo): SessionModel {
  const next = fromConversation(conversation);
  const json = JSON.stringify(next);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useMemo(() => next, [json]);
}

function ConversationView({
  conversation,
  now,
  projects,
  onSwitch,
}: {
  conversation: ConversationInfo;
  now: number;
  projects: ProjectInfo[];
  onSwitch?: OpenSwitch;
}): JSX.Element {
  const host = useActionHost();
  const title = conversationTitle(conversation);
  // On its own page: no "Open".
  const menu = useItemMenu("conversation", conversation, title, { openConversation: undefined });
  const accounts = useSubscriptions();
  const model = useStableModel(conversation);
  const project = projects.find((p) => p.id === conversation.project_id);
  const crumbs: Crumb[] = project
    ? [{ label: project.name, onOpen: () => host.openProjectById?.(project.id), testId: "crumb-project" }]
    : [];
  const latest = useRef(conversation);
  useEffect(() => {
    latest.current = conversation;
  }, [conversation]);
  const goTo = useCallback(() => void host.goToConversation?.(latest.current), [host]);
  const carryIn = useCallback(() => void host.continueConversation?.(latest.current), [host]);
  const account = accounts ? accountOfProfile(accounts, model.profileId) : undefined;
  // Always both tabs: Usage says when nothing is measured (a working
  // session's tokens come and go between refreshes).
  // AJ-E2404: a session open in its app has its processes (read-only but
  // for what it leaves behind once it ends).
  const live = conversation.live;
  const tabs = useMemo(() => pageTabs("conversation", { hasUsage: true, live }), [live]);
  // A refresh that briefly lacks the counts keeps the last ones.
  const tokens = conversation.tokens;
  const lastTokens = useRef(tokens);
  useEffect(() => {
    if (tokens) lastTokens.current = tokens;
  }, [tokens]);
  return (
    <div className="mx-auto flex h-full w-full max-w-[1440px] flex-col" data-testid="conversation-page" {...menu.triggerProps}>
      <PageShell
        kind="conversation"
        tabs={tabs}
        header={
          <PageHeader
            kind="conversation"
            layout="session"
            mark={<SessionMark model={model} />}
            name={title}
            crumbs={crumbs}
            chips={<SessionChips model={model} account={account} now={now} />}
            actions={
              <>
                {conversation.live ? (
                  host.goToConversation && <GoToButton onGo={goTo} why={null} />
                ) : (
                  <>
                    <Button size="md" variant="primary" onClick={carryIn} data-testid="conversation-continue">
                      Continue in AI Juggler
                    </Button>
                    <Button size="md" variant="secondary" onClick={() => void host.copyConversationResume?.(latest.current)} data-testid="conversation-copy-resume">
                      Copy resume command
                    </Button>
                  </>
                )}
                {menu.button({ label: `Actions for ${title}`, shown: true, testId: "header-menu" })}
              </>
            }
          />
        }
        render={(tab, visible) =>
          tab === "usage" ? (
            <UsagePanel model={model} visible={visible} tokens={tokens ?? lastTokens.current} account={account} />
          ) : tab === "processes" ? (
            <ProcessesView sessionId={conversation.id} height="calc(100vh - 22rem)" testId="conversation-processes" />
          ) : (
            <SessionOverview
              model={model}
              visible={visible}
              outside
              onGoTo={conversation.live && host.goToConversation ? goTo : undefined}
              onPick={onSwitch ? (recipeId, label) => onSwitch(model.switchSource, recipeId, label) : undefined}
              onCarryIn={host.continueConversation ? carryIn : undefined}
            />
          )
        }
        footer={(tab) => (
          <footer className="sticky bottom-0 z-10 mt-auto border-t border-hairline/60 bg-canvas pt-2 pb-1 text-label text-muted">from {tab.source}</footer>
        )}
      />
      {menu.element}
    </div>
  );
}

