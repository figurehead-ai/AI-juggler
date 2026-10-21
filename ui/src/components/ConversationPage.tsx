/**
 * AJ-E1529 — a conversation AI Juggler didn't start: what it is, where,
 * on which account, and the two things it can do with it (D-08: never
 * stop or tidy what isn't its own). "Continue in AI Juggler" resumes it
 * as a session of its own on the same account, on the user's click;
 * "Copy resume command" gives the line to run it in a terminal.
 */
import type { JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { useActionHost, useItemMenu } from "./ItemMenu";
import { ToolChip, ToolMark } from "./ToolMark";
import { conversationTitle, useConversations } from "../lib/conversations";
import type { ConversationInfo } from "../lib/ipc/conversations";

function when(ms: number | null, now: number): string {
  if (ms === null) return "—";
  const minutes = Math.max(0, Math.round((now - ms) / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours} h ago` : `${Math.floor(hours / 24)} d ago`;
}

function tokenWords(tokens: { input: number; output: number; cache_read: number; cache_write: number } | null): string {
  if (!tokens) return "Not measured";
  const total = tokens.input + tokens.output + tokens.cache_read + tokens.cache_write;
  return total >= 1_000_000 ? `${(total / 1_000_000).toFixed(1)}M tokens` : total >= 1000 ? `${Math.round(total / 1000)}k tokens` : `${total} tokens`;
}

export function ConversationPage({ accountId, id, now }: { accountId: string; id: string; now: number }): JSX.Element {
  const { all } = useConversations();
  const conversation = all.find((row) => row.account_id === accountId && row.id === id);
  if (!conversation) {
    return (
      <Card className="m-6 p-6 text-sm text-muted" data-testid="conversation-gone">
        This conversation isn't there any more.
      </Card>
    );
  }
  return <ConversationView conversation={conversation} now={now} />;
}

function ConversationView({ conversation, now }: { conversation: ConversationInfo; now: number }): JSX.Element {
  const host = useActionHost();
  // On its own page: no "Open".
  const menu = useItemMenu("conversation", conversation, conversationTitle(conversation), { openConversation: undefined });
  const title = conversationTitle(conversation);
  const facts: [string, string][] = [
    ["Account", conversation.account_label],
    ["Folder", conversation.cwd ?? "—"],
    ["Branch", conversation.git_branch ?? "—"],
    ["Last activity", when(conversation.last_activity_ms, now)],
    ["Tokens", tokenWords(conversation.tokens)],
  ];
  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto p-6" data-testid="conversation-page" {...menu.triggerProps}>
      <header className="flex flex-wrap items-center gap-3">
        <ToolMark tool={conversation.tool} size={18} className="text-accent-hi" />
        <h2 className="min-w-0 truncate text-lg font-semibold tracking-tight">{title}</h2>
        <Chip tone={conversation.live ? "accent" : "neutral"} dot={conversation.live}>
          {conversation.live ? "Running" : "Ended"}
        </Chip>
        <span data-testid="outside-chip">
          <Chip tone="neutral">outside AI Juggler</Chip>
        </span>
        <div className="ml-auto flex items-center gap-2">
          {menu.button({ label: `Actions for ${title}`, shown: true, testId: "header-menu" })}
        </div>
      </header>
      <p className="-mt-2 text-xs text-muted">
        Started outside AI Juggler, so it shows here to read; AI Juggler doesn't stop or change it.
      </p>
      <Card className="flex flex-col gap-3 p-5">
        <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5 text-[12px]">
          {facts.map(([label, value]) => (
            <div key={label} className="contents">
              <dt className="text-muted">{label}</dt>
              <dd className="min-w-0 truncate text-ink" title={value}>
                {label === "Account" ? (
                  <span className="inline-flex items-center gap-1.5">
                    <ToolChip tool={conversation.tool} /> {value}
                  </span>
                ) : (
                  value
                )}
              </dd>
            </div>
          ))}
        </dl>
        <div className="flex flex-wrap items-center gap-2 border-t border-hairline pt-3">
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
          {conversation.live && <span className="text-[11px] text-muted">It's open in another session now: continue here once it's closed there.</span>}
        </div>
      </Card>
      {menu.element}
    </div>
  );
}
