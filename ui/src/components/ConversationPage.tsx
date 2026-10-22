/**
 * AJ-E1529 — a conversation AI Juggler didn't start: what it is, where,
 * on which account, and the two things it can do with it (D-08: never
 * stop or tidy what isn't its own). "Continue in AI Juggler" resumes it
 * as a session of its own on the same account, on the user's click;
 * "Copy resume command" gives the line to run it in a terminal.
 * AJ-E1533: the same page as every session's — its type, its project
 * as the crumb, its actions in the header.
 */
import type { JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { ToolMark } from "./ToolMark";
import { AccountDisc } from "./indicator";
import { PageHeader, PageShell, type Crumb } from "../pages/PageShell";
import { pageTabs } from "../pages/registry";
import { conversationTitle, useConversations } from "../lib/conversations";
import { ago } from "../lib/time-words";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { ProjectInfo } from "../lib/bridge";

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
  const title = conversationTitle(conversation);
  const project = projects.find((p) => p.id === conversation.project_id);
  const crumbs: Crumb[] = project
    ? [{ label: project.name, onOpen: () => host.openProjectById?.(project.id), testId: "crumb-project" }]
    : [];
  const last = conversation.last_activity_ms === null ? null : ago(conversation.last_activity_ms, now);
  const facts: [string, string][] = [
    ["Folder", conversation.cwd ?? "—"],
    ["Branch", conversation.git_branch ?? "—"],
    ["Last activity", last === null ? "—" : last === "now" ? "just now" : `${last} ago`],
    ["Tokens", tokenWords(conversation.tokens)],
  ];
  return (
    <div className="flex h-full flex-col" data-testid="conversation-page" {...menu.triggerProps}>
      <PageShell
        kind="conversation"
        tabs={pageTabs("conversation")}
        header={
          <PageHeader
            kind="conversation"
            mark={<ToolMark tool={conversation.tool} size={18} />}
            name={title}
            crumbs={crumbs}
            chips={
              <>
                <Chip tone={conversation.live ? "accent" : "neutral"} dot={conversation.live}>
                  {conversation.live ? "Running" : "Ended"}
                </Chip>
                <span data-testid="outside-chip">
                  <Chip tone="neutral">outside AI Juggler</Chip>
                </span>
              </>
            }
            facts={
              <span className="inline-flex items-center gap-1.5">
                <AccountDisc accountId={conversation.account_id} label={conversation.account_label} size={16} />
                {conversation.account_label}
                {last !== null && <span>· {last === "now" ? "active just now" : `active ${last} ago`}</span>}
              </span>
            }
            actions={
              <>
                <Button
                  size="sm"
                  variant="primary"
                  disabled={conversation.live}
                  title={conversation.live ? "It's open in another session now" : undefined}
                  onClick={() => void host.continueConversation?.(conversation)}
                  data-testid="conversation-continue"
                >
                  Continue in AI Juggler
                </Button>
                <Button size="sm" variant="secondary" onClick={() => void host.copyConversationResume?.(conversation)} data-testid="conversation-copy-resume">
                  Copy resume command
                </Button>
                {menu.button({ label: `Actions for ${title}`, shown: true, testId: "header-menu" })}
              </>
            }
          />
        }
        render={() => (
          <Card className="flex flex-col gap-3 p-5">
            <p className="text-xs text-muted">
              Started outside AI Juggler, so it shows here to read; AI Juggler doesn't stop or change it.
              {conversation.live && " It's open in another session now: continue here once it's closed there."}
            </p>
            <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5 text-[12px]">
              {facts.map(([label, value]) => (
                <div key={label} className="contents">
                  <dt className="text-muted">{label}</dt>
                  <dd className="min-w-0 truncate text-ink" title={value}>
                    {value}
                  </dd>
                </div>
              ))}
            </dl>
          </Card>
        )}
        footer={() => (
          <footer className="sticky bottom-0 z-10 mt-auto border-t border-hairline/60 bg-canvas pt-2 pb-1 text-[10px] text-muted">
            from the tool's own session files
          </footer>
        )}
      />
      {menu.element}
    </div>
  );
}
