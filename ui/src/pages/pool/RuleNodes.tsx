/**
 * D-62 (AJ-E3008 req. 2, AJ-E3006): the members that aren't one session
 * of AI Juggler's, drawn on the canvas as dashed group nodes (wireframe
 * 6): an App, Account, Tool, Mac or everyone in scope says how many
 * sessions it brings now and that new ones join; a Watched session (one
 * another app runs) says it can't reply, with its one fix, Continue in AI
 * Juggler. Selecting one opens it in the inspector.
 */
import type { JSX } from "react";

import { Chip } from "../../components/primitives";
import { MACHINE_WORD } from "../../lib/machine-words";
import { useActionHost } from "../../components/ItemMenu";
import { useConversations } from "../../lib/conversations";
import type { PoolRuleInfo, WatchedMemberInfo } from "../../lib/ipc/pool-model";
import { roleLabel } from "../../lib/pools";
import type { Selection } from "./Canvas";

const KIND_WORD: Record<string, string> = { app: "App", account: "Account", tool: "Tool", mac: MACHINE_WORD, everyone: "Everyone" };

export function RuleNodes({
  rules,
  watched,
  labelOf,
  selection,
  onSelect,
}: {
  rules: PoolRuleInfo[];
  watched: WatchedMemberInfo[];
  /** A member rule's name on screen ("Claude Code · Work", "This Mac"). */
  labelOf: (rule: PoolRuleInfo) => string;
  selection: Selection;
  onSelect: (selection: Selection) => void;
}): JSX.Element | null {
  const host = useActionHost();
  const { all } = useConversations();
  const broad = rules.filter((rule) => rule.kind !== "session" && !rule.member.startsWith("conversation:"));
  const waiting = watched.filter((one) => one.continued_session === null);
  if (broad.length === 0 && waiting.length === 0) return null;
  const selected = (ref: string) => selection.kind === "rule" && selection.ref === ref;
  return (
    <div className="pointer-events-none absolute right-3 top-3 z-10 flex max-h-[calc(100%-5rem)] w-56 flex-col gap-2 overflow-y-auto" data-testid="canvas-rule-nodes" data-canvas-scroll>
      {broad.map((rule) => (
        <button
          key={rule.member}
          type="button"
          onClick={() => onSelect({ kind: "rule", ref: rule.member })}
          className={"pointer-events-auto flex flex-col gap-1 rounded-xl border border-dashed bg-surface-2 px-3 py-2 text-left shadow-card outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (selected(rule.member) ? "border-accent" : "border-hairline")}
          data-testid="canvas-rule-node"
          data-kind={rule.kind}
        >
          <span className="flex items-center gap-1.5">
            <span className="text-label font-semibold uppercase tracking-wide text-muted">{KIND_WORD[rule.kind] ?? rule.kind}</span>
            <Chip tone={rule.role === "lead" ? "accent" : rule.role === "reviewer" ? "live" : "neutral"}>{roleLabel(rule.role)}</Chip>
          </span>
          <span className="truncate text-body font-semibold text-ink">{labelOf(rule)}</span>
          <span className="text-meta text-ink-2">
            {rule.sessions} {rule.sessions === 1 ? "session" : "sessions"} now · new ones join{rule.level ? ` · its own level` : ""}
          </span>
        </button>
      ))}
      {waiting.map((one) => {
        const conversation = all.find((row) => row.account_id === one.folder && row.id === one.conversation_id);
        return (
          <div
            key={one.member}
            className={"pointer-events-auto flex flex-col gap-1 rounded-xl border border-dashed bg-surface px-3 py-2 shadow-card " + (selected(one.member) ? "border-accent" : "border-warning/50")}
            data-testid="canvas-watched-node"
          >
            <button type="button" onClick={() => onSelect({ kind: "rule", ref: one.member })} className="flex flex-col gap-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60">
              <span className="flex items-center gap-1.5">
                <Chip tone="warning">Watched</Chip>
                <span className="text-label text-muted">{roleLabel(one.role)}</span>
              </span>
              <span className="truncate text-body font-semibold text-ink">{one.title ?? "A session in another app"}</span>
              <span className="text-meta text-ink-2">{[one.tool, "can't reply"].filter(Boolean).join(" · ")}</span>
            </button>
            {conversation && (
              <button
                type="button"
                onClick={() => void host.continueConversation?.(conversation)}
                className="self-start text-meta font-medium text-accent-hi underline-offset-2 hover:underline"
                data-testid="canvas-watched-continue"
              >
                Continue in AI Juggler
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
