/**
 * D-62 (AJ-E3008 req. 2, AJ-E3006): the members that aren't one session
 * of AI Juggler's, drawn on the canvas as group nodes (wireframe 6): an
 * App, Account, Tool, Mac or everyone in scope says how many of the
 * sessions it brings are open now and that new ones join (its ended ones
 * are in the List view); a Watched session (one another app runs) says it
 * can't reply, with its one fix, Continue in AI Juggler. Selecting one
 * opens it in the inspector.
 *
 * 0.4.6: a broad member is a stacked card, never a dashed one (AJ-E3012:
 * dashes never mean selection; the selected one has the one ring). Its
 * name takes two lines before an ellipsis. Each says how its sessions hear
 * a message (D-119). Above them, the lead's place (AJ-E3011): "Lead: [App]'s
 * next session" while the pool waits for it, or "No lead" with what a
 * lead would add.
 */
import { useLayoutEffect, useRef, useState, type JSX } from "react";

import { Chip } from "../../components/primitives";
import { ShellOnly } from "../../components/ShellOnly";
import { WhyButton } from "../../components/WhyTip";
import { continueWhy } from "../../lib/resume-refusal";
import { MACHINE_WORD } from "../../lib/machine-words";
import { useActionHost } from "../../components/ItemMenu";
import { useConversations } from "../../lib/conversations";
import type { PoolRuleInfo, WatchedMemberInfo } from "../../lib/ipc/pool-model";
import { CONTINUE_WATCHED, CONTINUE_WATCHED_SHORT, levelLabel, roleLabel } from "../../lib/pools";
import { openNowWords } from "../../lib/pool-open-now";
import { LEAD_WOULD, nextSessionWords, wakeWords } from "../../lib/pool-words";
import { IconLead } from "../../components/icons-canvas";
import type { Selection } from "./Canvas";
import { FOCUS_RING, SELECTED } from "./selection";

/** A broad member's stack: two cards' edges behind it (surface tone, no
 * border), so it reads as many sessions, not one. */
const STACK = "shadow-[0_3px_0_-1px_var(--color-surface-3),0_6px_0_-2px_var(--color-surface-2)]";

const KIND_WORD: Record<string, string> = { app: "App", account: "Account", tool: "Tool", mac: MACHINE_WORD, everyone: "Everyone" };

/** The member rules drawn as group nodes: an App, Account, Tool, Mac or
 * everyone (a session's own rule is its node). */
const broadOf = (rules: readonly PoolRuleInfo[]) => rules.filter((rule) => rule.kind !== "session" && !rule.member.startsWith("conversation:"));

/** The room the group nodes' column takes at the canvas's right (w-56
 * and its 12 px from the edge), so the fit keeps members out from under
 * it; none when there's no column. */
export function ruleColumnRoom(rules: readonly PoolRuleInfo[], watched: readonly WatchedMemberInfo[], leadCard = false): number {
  return leadCard || broadOf(rules).length > 0 || watched.some((one) => one.continued_session === null) ? 240 : 0;
}

export function RuleNodes({
  rules,
  watched,
  labelOf,
  selection,
  onSelect,
  placeOf,
  toolOf,
  lead: leadPlace = false,
  leadNext = null,
  poolLevel = "work_together",
}: {
  /** AJ-E3011: the lead's place, drawn while no session leads. */
  lead?: boolean;
  /** The App whose next session will lead, by name (null: no lead for now). */
  leadNext?: string | null;
  /** The pool's level: a member's own, else this, says how it wakes. */
  poolLevel?: string;
  /** The tool and App an App or Tool member's sessions all share: its
   * "Shell only" is theirs (the P8 walk); `null` when they may differ. */
  toolOf?: (rule: PoolRuleInfo) => { tool: string; account: string | null } | null;
  /** Where a member's node is on screen now (its lines end there). */
  placeOf?: (id: string) => { x: number; y: number; w: number; h: number } | null;
  rules: PoolRuleInfo[];
  watched: WatchedMemberInfo[];
  /** A member rule's name on screen ("Claude Code · Work", "This Mac"). */
  labelOf: (rule: PoolRuleInfo) => string;
  selection: Selection;
  onSelect: (selection: Selection) => void;
}): JSX.Element | null {
  const host = useActionHost();
  const { all } = useConversations();
  const broad = broadOf(rules);
  const waiting = watched.filter((one) => one.continued_session === null);
  const column = useRef<HTMLDivElement | null>(null);
  const [lines, setLines] = useState<{ key: string; x1: number; y1: number; x2: number; y2: number; on: boolean }[]>([]);
  // Each group node's line to every session it brings in, from its left
  // edge to the session's node, measured after each draw (pans, zooms).
  useLayoutEffect(() => {
    const box = column.current;
    if (!box || !placeOf) return;
    const next: typeof lines = [];
    for (const node of box.querySelectorAll<HTMLElement>("[data-rule-ref]")) {
      const rule = broad.find((one) => one.member === node.dataset.ruleRef);
      const y1 = box.offsetTop + node.offsetTop - box.scrollTop + node.offsetHeight / 2;
      for (const id of rule?.session_ids ?? []) {
        const at = placeOf(id);
        if (at) next.push({ key: `${rule!.member}>${id}`, x1: box.offsetLeft, y1, x2: at.x + at.w, y2: at.y + at.h / 2, on: selection.kind === "rule" && selection.ref === rule!.member });
      }
    }
    if (JSON.stringify(next) !== JSON.stringify(lines)) setLines(next);
  });
  const lead = leadPlace ? { next: leadNext } : null;
  if (broad.length === 0 && waiting.length === 0 && !lead) return null;
  const selected = (ref: string) => selection.kind === "rule" && selection.ref === ref;
  const leadSelected = selection.kind === "lead-next";
  return (
    <>
    {lines.length > 0 && (
      <svg aria-hidden="true" className="pointer-events-none absolute inset-0 z-[5] h-full w-full" data-testid="canvas-rule-lines">
        {lines.map((line) => (
          <path key={line.key} d={`M${line.x1},${line.y1} C${line.x1 - 60},${line.y1} ${line.x2 + 60},${line.y2} ${line.x2},${line.y2}`} fill="none" strokeDasharray={line.on ? undefined : "4 4"} strokeWidth={line.on ? 2 : 1.25} className={line.on ? "stroke-accent" : "stroke-hairline"} />
        ))}
      </svg>
    )}
    <div ref={column} className="pointer-events-none absolute right-3 top-3 z-10 flex max-h-[calc(100%-5rem)] w-56 flex-col gap-3 overflow-y-auto p-1" data-testid="canvas-rule-nodes" data-canvas-scroll onScroll={() => setLines((was) => [...was])}>
      {lead && (
        <button
          type="button"
          onClick={() => onSelect(lead.next ? { kind: "lead-next" } : { kind: "pool", focus: "lead" })}
          aria-pressed={lead.next ? leadSelected : undefined}
          aria-current={leadSelected ? "true" : undefined}
          className={"pointer-events-auto flex flex-col gap-1 rounded-xl px-3 py-2 text-left " + FOCUS_RING + " " + (leadSelected ? SELECTED : "bg-surface-2 hover:bg-surface-3")}
          data-testid="canvas-lead-card"
          data-kind={lead.next ? "next" : "none"}
        >
          <span className="flex items-center gap-1.5 text-label text-muted">
            <IconLead aria-hidden="true" width={12} height={12} />
            Lead
          </span>
          <span className="line-clamp-2 text-body font-semibold text-ink">{lead.next ? nextSessionWords(lead.next) : "No lead for now"}</span>
          <span className="text-meta text-ink-2">{lead.next ? "Leads once it starts here." : LEAD_WOULD}</span>
        </button>
      )}
      {broad.map((rule) => (
        <button
          key={rule.member}
          type="button"
          onClick={() => onSelect({ kind: "rule", ref: rule.member })}
          aria-pressed={selected(rule.member)}
          aria-current={selected(rule.member) ? "true" : undefined}
          className={"pointer-events-auto mb-1.5 flex flex-col gap-1 rounded-xl px-3 py-2 text-left " + FOCUS_RING + " " + (selected(rule.member) ? SELECTED : `bg-surface ${STACK} hover:bg-surface-2`)}
          data-testid="canvas-rule-node"
          data-kind={rule.kind}
          data-rule-ref={rule.member}
          data-selected={selected(rule.member) || undefined}
        >
          <span className="flex items-center gap-1.5">
            <span className="text-label font-medium text-muted">{KIND_WORD[rule.kind] ?? rule.kind}</span>
            <Chip tone="neutral">{roleLabel(rule.role)}</Chip>
            <RuleShellOnly shared={toolOf?.(rule) ?? null} />
          </span>
          <span className="line-clamp-2 text-body font-semibold text-ink [overflow-wrap:normal]" title={labelOf(rule)}>
            {labelOf(rule)}
          </span>
          <span className="text-meta text-ink-2">
            {openNowWords(rule.sessions)}
            {rule.level ? ` · its own choice: ${levelLabel(rule.level)}` : ""}
          </span>
          <span className="text-label text-muted" data-testid="canvas-rule-wake">
            Its sessions {wakeWords(rule.level ?? poolLevel, false, true)}
          </span>
        </button>
      ))}
      {waiting.map((one) => {
        const conversation = all.find((row) => row.account_id === one.folder && row.id === one.conversation_id);
        return (
          <div
            key={one.member}
            className={"pointer-events-auto flex flex-col gap-1 rounded-xl px-3 py-2 " + (selected(one.member) ? SELECTED : "bg-surface shadow-card")}
            data-testid="canvas-watched-node"
          >
            <button type="button" aria-pressed={selected(one.member)} onClick={() => onSelect({ kind: "rule", ref: one.member })} className={"flex flex-col gap-1 rounded-lg text-left " + FOCUS_RING}>
              <span className="flex items-center gap-1.5">
                <Chip tone="neutral">Watched</Chip>
                <span className="text-label text-muted">{roleLabel(one.role)}</span>
              </span>
              <span className="line-clamp-2 text-body font-semibold text-ink">{one.title ?? "A session in another app"}</span>
              <span className="text-meta text-ink-2">{[one.tool, "can't reply"].filter(Boolean).join(" · ")}</span>
            </button>
            {conversation && (
              // rc31: open in the Claude app, it says so here (not only in
              // the app's log), as a greyed Resume does (rc21).
              <WhyButton
                size="sm"
                variant="ghost"
                why={continueWhy(conversation)}
                onClick={() => void host.continueConversation?.(conversation)}
                title={CONTINUE_WATCHED}
                className="self-start text-meta font-medium text-accent-hi"
                data-testid="canvas-watched-continue"
              >
                Continue in AI Juggler
              </WhyButton>
            )}
            {conversation && (
              <span className="text-meta text-ink-2" data-testid="canvas-watched-continue-what">
                {CONTINUE_WATCHED_SHORT}
              </span>
            )}
          </div>
        );
      })}
    </div>
    </>
  );
}

/** A broad member's "Shell only", as its sessions' nodes say it. */
function RuleShellOnly({ shared, inRow = true }: { shared: { tool: string; account: string | null } | null; inRow?: boolean }): JSX.Element | null {
  if (!shared) return null;
  return <ShellOnly tool={shared.tool} account={shared.account} inRow={inRow} />;
}
