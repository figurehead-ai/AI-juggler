/**
 * D-62 (AJ-E3008 req. 2, AJ-E3006): the members that aren't one session
 * of AI Juggler's, drawn on the canvas as group nodes (wireframe 6): an
 * App, Account, Tool, Mac or everyone in scope says how many of the
 * sessions it brings are open now and that new ones join (its ended ones
 * are in the List view); a session another app runs is a member through
 * its app's connector, or Watched with why and Add (AJ-E3013,
 * DesktopMemberCard). Selecting one opens it in the inspector.
 *
 * 0.4.6: a broad member is a stacked card, never a dashed one (AJ-E3012:
 * dashes never mean selection; the selected one has the one ring). Its
 * name takes two lines before an ellipsis. Each says how its sessions hear
 * a message (D-119). Above them, the lead's place (AJ-E3011): "No lead for
 * now" with what a lead would add. Round 2 (#2, #3): an App whose next
 * session leads is one card, its own ("Its next session leads · its
 * others join as Member"), with "Start one now" under it (New session on
 * that App, in the pool's project; the user starts it).
 */
import { useEffect, useLayoutEffect, useRef, useState, type JSX } from "react";

import { Button, Chip } from "../../components/primitives";
import { StartLeadButton } from "./setup/StartLead";
import { ShellOnly } from "../../components/ShellOnly";
import { MACHINE_WORD } from "../../lib/machine-words";
import type { PoolRuleInfo, WatchedMemberInfo } from "../../lib/ipc/pool-model";
import { levelLabel, roleLabel } from "../../lib/pools";
import { reachOf, useDesktopMembers, useDesktopSessions } from "../../lib/connector";
import { DesktopSessions, quiet, sessionsUnder } from "./DesktopSessions";
import { DesktopMemberCard } from "./DesktopMemberCard";
import { everyoneJoins, openNowWords } from "../../lib/pool-open-now";
import { LEADS_WHEN, LEAD_WOULD, nextLeadsLine, wakeWords } from "../../lib/pool-words";
import { cliFolderOf } from "../../lib/start-lead";
import { settleAdds, settleRoles, usePendingAdds, usePendingRoles } from "../../lib/pool-pending";
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
  where,
  onStartLead,
  poolId,
}: {
  /** Its pool: members being added show at once, pending (round 4 #5). */
  poolId?: string;
  /** AJ-E3011: the lead's place, drawn while no session leads. */
  lead?: boolean;
  /** The App whose next session will lead (null: no lead for now). */
  leadNext?: { ref: string; name: string } | null;
  /** The pool's projects in words ("web"): "new ones in web join". */
  where?: string;
  /** Opens New session on that App, in the pool's project. */
  onStartLead?: (member: string) => void;
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
  const broad = broadOf(rules);
  // AJ-E3013: the desktop members' connectors (an App's, a session's), and
  // the desktop sessions an App brings in through it, under their App.
  const desktop = useDesktopMembers(poolId);
  const desktopSessions = useDesktopSessions(poolId);
  const under = (rule: PoolRuleInfo) => sessionsUnder(rule, desktopSessions, broad, labelOf);
  const waiting = watched.filter((one) => one.continued_session === null);
  // Round 4 (#5): a member being added shows at once, pending, until the
  // rules read back have it; its role change likewise.
  const adding = usePendingAdds(poolId ?? "").filter((one) => one.kind !== "session" && !broad.some((rule) => rule.member === one.ref));
  const pendingRoles = usePendingRoles(poolId ?? "");
  useEffect(() => {
    if (!poolId) return;
    settleAdds(poolId, new Set(broad.map((rule) => rule.member)));
    settleRoles(poolId, new Map([...broad.map((rule) => [rule.member, rule.role] as const), ...(leadNext ? [[leadNext.ref, "lead"] as const] : [])]));
  });
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
  // Round 2 (#2): the App whose next session leads is its own card; the
  // lead's place is drawn only while no one leads and none is waited for.
  const nextRef = leadPlace && leadNext ? leadNext.ref : null;
  const noLead = leadPlace && nextRef === null;
  if (broad.length === 0 && waiting.length === 0 && !noLead && adding.length === 0) return null;
  const selected = (ref: string) => selection.kind === "rule" && selection.ref === ref;
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
      {noLead && (
        <button
          type="button"
          onClick={() => onSelect({ kind: "pool", focus: "lead" })}
          className={"pointer-events-auto flex flex-col gap-1 rounded-xl bg-surface-2 px-3 py-2 text-left hover:bg-surface-3 " + FOCUS_RING}
          data-testid="canvas-lead-card"
          data-kind="none"
        >
          <span className="flex items-center gap-1.5 text-label text-muted">
            <IconLead aria-hidden="true" width={12} height={12} />
            Lead
          </span>
          <span className="text-body font-semibold text-ink">No lead for now</span>
          <span className="text-meta text-ink-2">{LEAD_WOULD}</span>
        </button>
      )}
      {adding.map((one) => (
        <div key={one.ref} aria-busy="true" className={"pointer-events-auto mb-1.5 flex flex-col gap-1 rounded-xl bg-surface px-3 py-2 opacity-70 " + STACK} data-testid="canvas-rule-node-adding" data-rule-ref={one.ref}>
          <span className="text-label font-medium text-muted">{KIND_WORD[one.kind] ?? one.kind}</span>
          <span className="line-clamp-2 text-body font-semibold text-ink [overflow-wrap:normal]">{one.label}</span>
          <span className="text-meta text-ink-2">Adding…</span>
        </div>
      ))}
      {broad.map((rule) => (
        <div key={rule.member} className="flex flex-col gap-1">
        <button
          type="button"
          onClick={() => onSelect({ kind: "rule", ref: rule.member })}
          aria-pressed={selected(rule.member)}
          aria-current={selected(rule.member) ? "true" : undefined}
          aria-busy={pendingRoles.has(rule.member) || undefined}
          className={"pointer-events-auto mb-1.5 flex flex-col gap-1 rounded-xl px-3 py-2 text-left " + FOCUS_RING + " " + (selected(rule.member) ? SELECTED : `bg-surface ${STACK} hover:bg-surface-2`) + (pendingRoles.has(rule.member) ? " opacity-70" : "")}
          data-testid="canvas-rule-node"
          data-kind={rule.kind}
          data-rule-ref={rule.member}
          data-selected={selected(rule.member) || undefined}
        >
          <span className="flex items-center gap-1.5">
            <span className="text-label font-medium text-muted">{KIND_WORD[rule.kind] ?? rule.kind}</span>
            {rule.member === nextRef || pendingRoles.get(rule.member) === "lead" ? (
              <span className="inline-flex items-center gap-1 text-label font-medium text-ink-2" data-testid="canvas-rule-leads-next">
                <IconLead aria-hidden="true" width={12} height={12} />
                {LEADS_WHEN}
              </span>
            ) : (
              <Chip tone="neutral">{roleLabel(pendingRoles.get(rule.member) ?? rule.role)}</Chip>
            )}
            <RuleShellOnly shared={toolOf?.(rule) ?? null} />
          </span>
          <span className="line-clamp-2 text-body font-semibold text-ink [overflow-wrap:normal]" title={labelOf(rule)}>
            {labelOf(rule)}
          </span>
          {rule.member === nextRef && (
            <span className="text-meta text-ink" data-testid="canvas-rule-next-line">
              {nextLeadsLine(rule.role)}
            </span>
          )}
          <span className="text-meta text-ink-2" data-testid="canvas-rule-open">
            {rule.member === "all" ? everyoneJoins(rule.sessions, where) : openNowWords(rule.sessions, where, quiet(under(rule)))}
            {rule.level ? ` · its own choice: ${levelLabel(rule.level)}` : ""}
          </span>
          <span className="text-label text-muted" data-testid="canvas-rule-wake">
            {desktop?.has(rule.member) ? desktopLine(desktop.get(rule.member)) : `Its sessions ${wakeWords(rule.level ?? poolLevel, false, true)}`}
          </span>
        </button>
        <DesktopSessions sessions={under(rule)} />
        <ConnectorFix info={desktop?.get(rule.member)} label={labelOf(rule)} onOpen={() => onSelect({ kind: "rule", ref: rule.member })} />
        {rule.member === nextRef && onStartLead && cliFolderOf(rule.member) && (
          <StartLeadButton member={rule.member} name={labelOf(rule)} where={where ?? "these projects"} onStart={onStartLead} className="pointer-events-auto self-start" testId="canvas-rule-start-lead" />
        )}
        </div>
      ))}
      {waiting.map((one) => (
        // AJ-E3013: connected through its app's connector, or Watched with why.
        <DesktopMemberCard key={one.member} one={one} desktop={desktop?.get(one.member)} selected={selected(one.member)} onSelect={() => onSelect({ kind: "rule", ref: one.member })} />
      ))}
    </div>
    </>
  );
}

/** Whether a desktop member's Add (or Fix) is its way in. */
/** An App's card: Add its connector, or Fix a changed one (as a
 * session's own card says it), opening the consent card. */
function ConnectorFix({ info, label, onOpen }: { info: Parameters<typeof reachOf>[0] | undefined; label: string; onOpen: () => void }): JSX.Element | null {
  const reach = info ? reachOf(info) : null;
  if (reach?.kind !== "watched" || reach.fix === null) return null;
  const word = reach.fix === "add" ? "Add" : "Fix";
  return (
    <Button size="sm" variant="accent" className="pointer-events-auto self-start" aria-label={`${word} AI Juggler's connector ${word === "Add" ? "to" : "in"} ${label}`} onClick={onOpen} data-testid="canvas-rule-connector-add">
      {word}
    </Button>
  );
}

/** A desktop App member's sessions: how a message reaches them through
 * its connector, or why they're Watched (AJ-E3013). */
function desktopLine(info: Parameters<typeof reachOf>[0]): string {
  const reach = reachOf(info);
  return reach.kind === "watched" ? `Watched: ${reach.line.charAt(0).toLowerCase()}${reach.line.slice(1)}` : `Its sessions: ${reach.line.charAt(0).toLowerCase()}${reach.line.slice(1)}`;
}

/** A broad member's "Shell only", as its sessions' nodes say it. */
function RuleShellOnly({ shared, inRow = true }: { shared: { tool: string; account: string | null } | null; inRow?: boolean }): JSX.Element | null {
  if (!shared) return null;
  return <ShellOnly tool={shared.tool} account={shared.account} inRow={inRow} />;
}
