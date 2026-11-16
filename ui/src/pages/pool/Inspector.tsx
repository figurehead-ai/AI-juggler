/**
 * The canvas's inspector (right): it follows the selection — the pool's
 * settings, one member, or one edge (the conversation between the two,
 * with what waits on it) — and, with nothing selected, the pool's live
 * activity, newest first.
 *
 * AJ-E3012: it always leads with what is selected — its title, its kind
 * and App as a subtitle, its status as a pill — and the title's edge
 * carries the selected card's accent. AJ-E3011: the setup's checklist is
 * its side card, in order, until every step is done.
 */
import { forwardRef, useState, type JSX, type ReactNode } from "react";

import { AppIcon } from "../../components/AppIcon";
import { appOfSession } from "../../lib/app-of";
import { appName } from "../../lib/app-name";
import { sessionState } from "../../lib/rail-items";
import type { SETUP_STEPS } from "../../lib/pool-words";
import { inspectorHead, type HeadOf, type InspectorHead } from "./inspector-head";
import { SELECTED_EDGE } from "./selection";
import { sessionAlert } from "./NodeParts";
import { PoolChecklist } from "./setup/PoolChecklist";
import { PoolSummary, shapeOfPool } from "./PoolSummary";
import { useTeamTools } from "./setup/TeamTools";
import { setupBlockers, setupReady, type SetupRow } from "../../lib/pool-setup";
import { FirstPrompt } from "./setup/FirstPrompt";
import { nextLine } from "../../lib/pool-sentences";
import { toolsLeftLine, type ToolsPlace } from "../../lib/team-tools";

import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import type { PoolInfo } from "../../lib/ipc/pools";
import type { TaskInfo } from "../../lib/ipc/tasks";
import { agoWords } from "../../lib/time-words";
import { lazyNamed } from "../../lib/lazy";
import { PendingChip, edgeWords } from "./CanvasEdges";
import { InspectNode } from "./InspectNode";
import { InspectPool } from "./InspectPool";
import { Section } from "./Section";
import type { Selection } from "./Canvas";
import { edgeKey, type MemberNode } from "./useCanvas";

// The conversation loads only when an edge is opened.
import { EdgeTalk } from "./EdgeTalk";
import { InspectRule, type RuleLead } from "./InspectRule";
import { setPoolLead } from "./pool-lead";
import { startNextLead } from "../../lib/start-lead";
import type { PoolRules } from "../../lib/pool-rules";
import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import { reachOf, useDesktopMembers } from "../../lib/connector";
import { Splitter, readEdgeWidth, type SplitterEdge } from "../../components/Splitter";

/** The inspector's edge (AJ-E3011 round 5, under D-55: the 340 px panel
 * hid Who leads and the role menu): 360 to 420 px, 380 at first, kept per
 * viewer. */
export const INSPECTOR_EDGE: SplitterEdge = { min: 360, max: 420, initial: 380, store: "aij.pool.inspector.width", label: "Resize the inspector", side: "left", testId: "inspector-splitter" };
import type { DesktopMemberInfo } from "../../lib/ipc/connector";

const ThreadView = lazyNamed(() => import("../messages/ThreadView"), "ThreadView");

function Activity({ pool, canvas, now, onSelect }: { pool: PoolInfo; canvas: PoolCanvasInfo | null; now: number; onSelect: (selection: Selection) => void }): JSX.Element {
  // An empty pool says it's empty once, on its canvas (design review #85).
  // Round 3 (#2): the pool in short is the summary above, in sentences;
  // here the way to all its settings, and what happens live.
  const empty = pool.members === 0;
  // Live shows while the pool has members, and for an empty one while its
  // stream says something (who just left, P5 step 4).
  const live = !empty || (canvas?.stream.length ?? 0) > 0;
  return (
    <div className="flex flex-col" data-testid="inspect-activity">
      <div className="px-4 pb-3">
        <button type="button" onClick={() => onSelect({ kind: "pool" })} className="rounded text-meta text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="inspect-open-settings">
          All of this pool's settings…
        </button>
      </div>
      {live && (
        <Section title="Live" testId="inspect-stream">
          {canvas === null ? (
            <span className="h-16 rounded-lg bg-skeleton motion-safe:animate-pulse" role="status" aria-label="Reading the pool's activity" />
          ) : canvas.stream.length === 0 ? (
            <p className="text-label text-muted">Nothing yet. Messages and task changes between its agents show here as they happen.</p>
          ) : (
            <ol className="flex flex-col gap-2" aria-live="polite" aria-relevant="additions">
              {canvas.stream.map((line, index) => (
                <li key={`${line.at_ms}-${index}`} className="flex gap-2 text-meta">
                  <span className="w-[4.75rem] shrink-0 whitespace-nowrap text-label tabular-nums text-faint">{agoWords(line.at_ms, now)}</span>
                  {line.session_id ? (
                    <button
                      type="button"
                      onClick={() => onSelect(line.other_session_id ? { kind: "edge", key: edgeKey(line.session_id!, line.other_session_id) } : { kind: "node", id: line.session_id! })}
                      className="min-w-0 rounded text-left text-ink outline-none hover:text-accent-hi focus-visible:ring-1 focus-visible:ring-accent/60"
                    >
                      {line.words}
                    </button>
                  ) : (
                    <span className="min-w-0 text-ink">{line.words}</span>
                  )}
                </li>
              ))}
            </ol>
          )}
          {!empty && <p className="text-label text-faint">Select a member or an edge and this panel shows its tasks, messages, usage and rules.</p>}
        </Section>
      )}
    </div>
  );
}

function InspectEdge({ pool, canvas, edgeId, names, now }: { pool: PoolInfo; canvas: PoolCanvasInfo | null; edgeId: string; names: ReadonlyMap<string, string>; now: number }): JSX.Element {
  const edge = canvas?.edges.find((row) => edgeKey(row.from_session_id, row.to_session_id) === edgeId);
  const [fromId = "", toId = ""] = edgeId.split(">");
  // D-62: the line's talk rule, whether or not anything passed yet.
  const talk = <EdgeTalk poolId={pool.id} from={fromId} to={toId} fromName={names.get(fromId) ?? "A member"} toName={names.get(toId) ?? "a member"} />;
  if (!edge) {
    return (
      <div className="flex flex-col gap-3" data-testid="inspect-edge">
        {talk}
        <p className="px-4 text-meta text-ink-2">Nothing passes between these two this week.</p>
      </div>
    );
  }
  const from = names.get(edge.from_session_id) ?? "A member";
  const to = names.get(edge.to_session_id) ?? "a member";
  const members = [
    { session_id: edge.from_session_id, name: from },
    { session_id: edge.to_session_id, name: to },
  ];
  return (
    <div className="flex flex-col" data-testid="inspect-edge">
      <div className="flex flex-col gap-1 px-4 pb-3 pt-1">
        <span className="text-meta text-muted">{edgeWords(edge, from, to).count} this week · last {agoWords(edge.last_at_ms, now)}</span>
      </div>
      <div className="pb-3">{talk}</div>
      {edge.pending.length > 0 && (
        <Section title="Waiting on you">
          <ul className="flex flex-col gap-1.5">
            {edge.pending.map((pending) => (
              <li key={pending.message_id} className="flex flex-col gap-1 rounded-lg border border-hairline/70 px-2.5 py-2">
                <span className="truncate text-meta text-ink">«{pending.subject}»</span>
                <span className="text-label text-muted">
                  {pending.kind === "hop_limit" ? "Paused: these replies went back and forth as many times as the pool allows." : pending.kind === "asked" ? `Above what the pool allows: answer it on ${from}.` : `Waits for ${to} to read it, or for your Deliver.`}
                </span>
              </li>
            ))}
          </ul>
          <PendingChip edge={edge} />
        </Section>
      )}
      <Section title="The conversation">
        {edge.thread_id ? (
          <div className="-mx-2 min-w-0" data-canvas-scroll>
            <ThreadView threadId={edge.thread_id} poolId={pool.id} members={members} now={now} />
          </div>
        ) : (
          <p className="text-label text-muted">Only task hand-offs so far: {from} gave {to} {edge.tasks === 1 ? "a task" : `${edge.tasks} tasks`}.</p>
        )}
      </Section>
    </div>
  );
}

/** A session member's head: its App, role and state (L3 when it stopped
 * by itself, L2 when it needs you). */
function sessionHead(member: MemberNode): HeadOf {
  const state = sessionState(member.session);
  const alert = sessionAlert(member);
  const app = (member.card.tool ? appName(member.card.tool, member.card.account) : member.card.account) ?? "its App";
  return { kind: "session", name: member.name, app, role: member.role, state: alert?.word ?? state.word, level: alert?.tone === "danger" ? 3 : state.needs ? 2 : 0 };
}

/** What the selection is, for its title, subtitle and pill. */
/** Who leads, as a member rule's inspector sets it (round 2 #2, round 3
 * #1): whether this App's next session leads, the pool's one setter,
 * "Start one now" in its one project, and its Who leads. */
function ruleLeadOf(rule: PoolRuleInfo | undefined, rules: PoolRules, members: readonly MemberNode[], input: { poolId: string; where: string; dir: string | null; onChange: () => void }): RuleLead | null {
  if (!rule) return null;
  const next = rules.rules?.lead_next;
  const leads = members.filter((member) => member.role === "lead").map((member) => member.id);
  return {
    leadsNext: !!next && rule.member === `app:${next}`,
    where: input.where,
    onStart: (ref) => startNextLead(ref, input.dir),
    onChange: input.onChange,
    onLead: (choice) => setPoolLead(input.poolId, leads, choice),
  };
}

function selectionHead(input: {
  selection: Selection;
  member: MemberNode | undefined;
  rule: PoolRuleInfo | undefined;
  watched: PoolRules["watched"][number] | undefined;
  canvas: PoolCanvasInfo | null;
  names: ReadonlyMap<string, string>;
  ruleLabel: (rule: PoolRuleInfo) => string;
  lead: RuleLead | null;
  pool: PoolInfo;
  project: ProjectInfo | undefined;
  where: string;
  needs: number;
  /** AJ-E3013: the pool's desktop members' connectors. */
  desktop?: ReadonlyMap<string, DesktopMemberInfo>;
}): HeadOf {
  const { selection, member, rule, watched, pool } = input;
  if (member) return sessionHead(member);
  // A session another app runs is its own, whatever its rule row says.
  if (watched) return { kind: "watched", title: watched.title ?? "A session in another app", app: watched.tool, role: watched.role, reach: reachOf(input.desktop?.get(watched.member)) };
  if (rule) return { kind: "rule", label: input.ruleLabel(rule), memberKind: rule.kind, role: rule.role, open: rule.sessions, leadsNext: input.lead?.leadsNext };
  if (selection.kind === "edge") {
    const [fromId = "", toId = ""] = selection.key.split(">");
    const edge = input.canvas?.edges.find((row) => edgeKey(row.from_session_id, row.to_session_id) === selection.key);
    const from = input.names.get(fromId) ?? "A member";
    const to = input.names.get(toId) ?? "a member";
    return { kind: "edge", from, to, count: edge ? `${edgeWords(edge, from, to).count} this week` : "Nothing yet this week", waits: (edge?.pending.length ?? 0) > 0 };
  }
  return { kind: "pool", name: pool.name, own: pool.kind === "default" ? (input.project?.name ?? null) : null, where: input.where, sessions: pool.members, sealed: pool.sealed, needs: input.needs };
}

/** The status under the title: calm at L0 and L1, an amber word and dot
 * at L2, red with a glyph on its tint at L3 (D-117). */
export function StatusPill({ pill }: { pill: NonNullable<InspectorHead["pill"]> }): JSX.Element {
  const shape = "inline-flex w-fit items-center gap-1.5 rounded-full px-2 py-0.5 text-label";
  if (pill.level === 3)
    return (
      <span className={shape + " bg-danger-soft font-semibold text-danger"} data-testid="inspector-pill" data-level={3}>
        <span aria-hidden="true">✕</span>
        {pill.words}
      </span>
    );
  if (pill.level === 2)
    return (
      <span className={shape + " bg-surface-2 font-semibold text-warning"} data-testid="inspector-pill" data-level={2}>
        <span aria-hidden="true" className="level-mark" data-level="2" />
        {pill.words}
      </span>
    );
  return (
    <span className={shape + " bg-surface-2 text-ink-2"} data-testid="inspector-pill" data-level={pill.level}>
      {pill.words}
    </span>
  );
}

type Step = (typeof SETUP_STEPS)[number];

/** The pool's setup progress: the checklist on the pool, one line over
 * its settings; none over a member, a line or the lead's card. */
/** The pool's own inspector: the team tools while they're off for it
 * (round 2, #1: above the checklist, and still there once every step is
 * done), then the checklist. */
function PoolProgress({ kind, setup, pool, rules, members, ruleLabel, where, needs }: { kind: Selection["kind"]; setup: InspectorSetup; pool: PoolInfo; rules: PoolRules; members: readonly MemberNode[]; ruleLabel: (rule: PoolRuleInfo) => string; where: string; needs: number }): JSX.Element | null {
  const tools = useTeamTools(setup.tools.projects, setup.tools.apps);
  const desktop = useDesktopMembers(pool.id);
  if (kind !== "none" && kind !== "pool") return null;
  const shape = shapeOfPool(pool, rules.rules, members, ruleLabel);
  // Round 6: what can't work is said, and the pool is never "ready" then.
  const names = new Map<string, string>([...(rules.rules?.members ?? []).map((rule) => [rule.member, ruleLabel(rule)] as const), ...rules.watched.map((one) => [one.member, one.title ?? "A session in another app"] as const)]);
  const blockers = setupBlockers(tools, [...(desktop?.values() ?? [])].map((one) => ({ name: names.get(one.member) ?? "A desktop member", reach: reachOf(one) })));
  const ready = setupReady(setup.rows, tools?.kind === "on") && blockers.length === 0;
  return (
    <>
      <PoolSummary shape={shape} where={where} needs={needs} tools={setup.tools} onOpen={setup.onOpen} />
      <PoolChecklist rows={setup.rows} onOpen={setup.onOpen} compact={kind === "pool"} ready={ready} next={ready ? nextLine(shape, where) : null} toolsLeft={toolsLeftLine(tools)} blockers={blockers} prompt={<FirstPrompt shape={shape} where={where} testId="pool-first-prompt" />} />
    </>
  );
}

/** The setup's checklist, opening one of its steps, and where the team
 * tools are switched (its projects and CLI Apps). */
type InspectorSetup = { rows: readonly SetupRow[]; onOpen: (step: Step) => void; tools: { projects: readonly ToolsPlace[]; apps: readonly ToolsPlace[] } };

export const Inspector = forwardRef<HTMLHeadingElement, {
  pool: PoolInfo;
  project: ProjectInfo | undefined;
  canvas: PoolCanvasInfo | null;
  members: MemberNode[];
  names: ReadonlyMap<string, string>;
  selection: Selection;
  now: number;
  onSelect: (selection: Selection) => void;
  onOpenTask: (task: TaskInfo) => void;
  onDeleted: () => void;
  /** D-62: the pool's member rules and Watched members, and their names. */
  rules: PoolRules;
  ruleLabel: (rule: PoolRuleInfo) => string;
  /** An App or Tool member's shared tool and App (its "Shell only"). */
  ruleTool?: (rule: PoolRuleInfo) => { tool: string; account: string | null } | null;
  /** Its projects in words ("web", "every project"). */
  where: string;
  /** What waits on you here (its Pools card's count). */
  needs: number;
  /** AJ-E3011: the setup's checklist, and opening one of its steps. */
  setup: InspectorSetup;
  /** The pool's one project folder: that App's "Start one now" opens there. */
  leadDir: string | null;
  /** The pool's setup controls (who leads, everyone else, talk, on their own). */
  setupSections?: ReactNode;
}>(function Inspector({ pool, project, canvas, members, names, selection, now, onSelect, onOpenTask, onDeleted, rules, ruleLabel, ruleTool, where, needs, setup, leadDir, setupSections }, heading): JSX.Element {
  const member = selection.kind === "node" ? members.find((row) => row.id === selection.id) : undefined;
  const rule = selection.kind === "rule" ? rules.rules?.members.find((row) => row.member === selection.ref) : undefined;
  const watched = selection.kind === "rule" ? rules.watched.find((row) => row.member === selection.ref) : undefined;
  const lead = ruleLeadOf(rule, rules, members, { poolId: pool.id, where, dir: leadDir, onChange: () => setup.onOpen("lead") });
  const desktop = useDesktopMembers(pool.id);
  const [width, setWidth] = useState(() => readEdgeWidth(INSPECTOR_EDGE));
  const of = selectionHead({ selection, member, rule, watched, canvas, names, ruleLabel, lead, pool, project, where, needs, desktop });
  const head = inspectorHead(of);
  // The selected card and the inspector are linked by the same accent.
  const linked = of.kind !== "pool";
  return (
    <aside id="pool-inspector" aria-labelledby="pool-inspector-title" style={{ width }} className="relative flex h-full min-h-0 shrink-0 flex-col bg-surface-2/40 shadow-[inset_1px_0_0_var(--color-divider)]" data-testid="pool-inspector" data-selected={of.kind}>
      {/* Round 5 (under D-55): wider, and resizable; its width is kept. */}
      <Splitter edge={INSPECTOR_EDGE} width={width} onWidth={setWidth} controls="pool-inspector" />
      <div className={"flex shrink-0 items-start gap-2.5 px-4 pb-3 pt-3 " + (linked ? SELECTED_EDGE : "")} data-testid="inspector-head">
        {member && <AppIcon {...appOfSession(member.session)} size={32} />}
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <h3 id="pool-inspector-title" ref={heading} tabIndex={-1} className="text-section font-semibold leading-snug text-ink outline-none [overflow-wrap:anywhere]" data-testid="inspector-title">
            {head.title}
          </h3>
          <p className="text-meta text-muted" data-testid="inspector-subtitle">
            {head.subtitle}
          </p>
          {head.pill && <StatusPill pill={head.pill} />}
        </div>
        {selection.kind !== "none" && (
          <button type="button" onClick={() => onSelect({ kind: "none" })} className="shrink-0 rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="inspector-back">
            {selection.kind === "pool" ? "Done" : "Pool"}
          </button>
        )}
      </div>
      {/* A new selection opens at its top (the list kept the last one's
          scroll: the pool's settings opened halfway down). */}
      <div key={JSON.stringify(selection)} className="min-h-0 flex-1 overflow-y-auto pb-6" data-canvas-scroll>
        <PoolProgress kind={selection.kind} setup={setup} pool={pool} rules={rules} members={members} ruleLabel={ruleLabel} where={where} needs={needs} />
        {selection.kind === "pool" ? (
          <>
            {setupSections}
            <InspectPool pool={pool} project={project} now={now} members={members.map((row) => ({ id: row.id, name: names.get(row.id) ?? row.card.name }))} onDeleted={onDeleted} />
          </>
        ) : member ? (
          <InspectNode
            pool={pool}
            member={member}
            members={members}
            asks={(canvas?.asks ?? []).filter((ask) => ask.session_id === member.id)}
            edges={canvas?.edges ?? []}
            names={names}
            now={now}
            onSelectEdge={(key) => onSelect({ kind: "edge", key })}
            onSelectPool={() => onSelect({ kind: "pool" })}
            onOpenTask={onOpenTask}
          />
        ) : selection.kind === "rule" ? (
          <InspectRule
            pool={pool}
            rule={rule}
            watched={watched}
            label={watched ? (watched.title ?? "A session in another app") : rule ? ruleLabel(rule) : "A member"}
            shared={rule ? (ruleTool?.(rule) ?? null) : null}
            lead={lead}
            onChanged={rules.reload}
            onGone={() => onSelect({ kind: "none" })}
          />
        ) : selection.kind === "edge" ? (
          <InspectEdge pool={pool} canvas={canvas} edgeId={selection.key} names={names} now={now} />
        ) : (
          <Activity pool={pool} canvas={canvas} now={now} onSelect={onSelect} />
        )}
      </div>
    </aside>
  );
});
