/**
 * AJ-E1529, AJ-E2108 — the page of a session another app runs (or one
 * AI Juggler's index found in the tools' files): where it runs, its last
 * messages, and Go to it. D-08: AI Juggler never stops or tidies what
 * isn't its own; "Continue in AI Juggler" resumes it as a session of its
 * own on the same profile, on the user's click. AJ-E1533: the same page
 * as every session's, its project as the crumb.
 */
import { useCallback, type JSX } from "react";
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
import { fromConversation } from "../pages/session/model";

const SessionOverview = lazyNamed(() => import("../pages/session/SessionOverview"), "SessionOverview");

function tokenWords(tokens: { input: number; output: number; cache_read: number; cache_write: number } | null): string {
  if (!tokens) return "Not measured";
  const total = tokens.input + tokens.output + tokens.cache_read + tokens.cache_write;
  return total >= 1_000_000 ? `${(total / 1_000_000).toFixed(1)}M tokens` : total >= 1000 ? `${Math.round(total / 1000)}k tokens` : `${total} tokens`;
}

export function ConversationPage({ accountId, id, now, projects = [] }: { accountId: string; id: string; now: number; projects?: ProjectInfo[] }): JSX.Element {
  const { all } = useConversations();
  const conversation = all.find((row) => row.account_id === accountId && row.id === id);
  if (!conversation) {
    return (
      <Card className="m-6 p-6 text-sm text-muted" data-testid="conversation-gone">
        This conversation isn't there any more.
      </Card>
    );
  }
  return <ConversationView conversation={conversation} now={now} projects={projects} />;
}

function ConversationView({ conversation, now, projects }: { conversation: ConversationInfo; now: number; projects: ProjectInfo[] }): JSX.Element {
  const host = useActionHost();
  // On its own page: no "Open".
  const menu = useItemMenu("conversation", conversation, conversationTitle(conversation), { openConversation: undefined });
  const accounts = useSubscriptions();
  const title = conversationTitle(conversation);
  const model = fromConversation(conversation);
  const project = projects.find((p) => p.id === conversation.project_id);
  const crumbs: Crumb[] = project
    ? [{ label: project.name, onOpen: () => host.openProjectById?.(project.id), testId: "crumb-project" }]
    : [];
  const goTo = useCallback(() => void host.goToConversation?.(conversation), [host, conversation]);
  const carryIn = () => void host.continueConversation?.(conversation);
  const hasUsage = conversation.tokens !== null;
  return (
    <div className="flex h-full flex-col" data-testid="conversation-page" {...menu.triggerProps}>
      <PageShell
        kind="conversation"
        tabs={pageTabs("conversation", { hasUsage })}
        header={
          <PageHeader
            kind="conversation"
            layout="session"
            mark={<SessionMark model={model} />}
            name={title}
            crumbs={crumbs}
            chips={<SessionChips model={model} account={accounts ? accountOfProfile(accounts, model.profileId) : undefined} now={now} />}
            facts={
              <span>
                Started <span data-testid="outside-chip">outside AI Juggler</span>: it shows here to read, and AI Juggler never stops or changes it.
              </span>
            }
            actions={
              <>
                {conversation.live ? (
                  host.goToConversation && <GoToButton onGo={goTo} why={null} />
                ) : (
                  <>
                    <Button size="sm" variant="primary" onClick={carryIn} data-testid="conversation-continue">
                      Continue in AI Juggler
                    </Button>
                    <Button size="sm" variant="secondary" onClick={() => void host.copyConversationResume?.(conversation)} data-testid="conversation-copy-resume">
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
            <Card className="flex flex-col gap-1.5 p-4 text-[12px]" data-testid="conversation-usage">
              <h3 className="text-[11px] font-medium uppercase tracking-wider text-muted">Usage</h3>
              <p className="text-ink">{tokenWords(conversation.tokens)}, each message counted once.</p>
            </Card>
          ) : (
            <SessionOverview model={model} visible={visible} onCarryIn={host.continueConversation ? carryIn : undefined} />
          )
        }
        footer={(tab) => (
          <footer className="sticky bottom-0 z-10 mt-auto border-t border-hairline/60 bg-canvas pt-2 pb-1 text-[10px] text-muted">from {tab.source}</footer>
        )}
      />
      {menu.element}
    </div>
  );
}
