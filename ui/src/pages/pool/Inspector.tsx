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
import { forwardRef, useState, type ForwardedRef, type JSX, type ReactNode } from "react";

import { AppIcon } from "../../components/AppIcon";
import { appOfSession } from "../../lib/app-of";
import { appName } from "../../lib/app-name";
import { sessionState } from "../../lib/rail-items";
import { inspectorHead, type HeadOf, type InspectorHead } from "./inspector-head";
import { ScrollEdge } from "../../components/ScrollEdge";
import { SELECTED_EDGE } from "./selection";
import { sessionAlert } from "./NodeParts";
import { OverviewFoot, PoolProgress, RunningSummary, SetupView, type InspectorSetup } from "./PoolOverview";
import type { SetupPhase, SetupProgress } from "./setup-progress";

import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
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

function InspectEdge({ pool, canvas, edgeId, names, now, onOther }: { pool: PoolInfo; canvas: PoolCanvasInfo | null; edgeId: string; names: ReadonlyMap<string, string>; now: number; onOther: (key: string) => void }): JSX.Element {
  const edge = canvas?.edges.find((row) => edgeKey(row.from_session_id, row.to_session_id) === edgeId);
  const [fromId = "", toId = ""] = edgeId.split(">");
  // A pair is one line on the canvas: its other way, one click from here.
  const other = canvas?.edges.find((row) => row.from_session_id === toId && row.to_session_id === fromId);
  // D-62: the line's talk rule, whether or not anything passed yet.
  const talk = <EdgeTalk poolId={pool.id} from={fromId} to={toId} anchors={canvas?.anchors} fromName={names.get(fromId) ?? "A member"} toName={names.get(toId) ?? "a member"} />;
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
        {other && (
          <button
            type="button"
            onClick={() => onOther(edgeKey(other.from_session_id, other.to_session_id))}
            className="w-fit rounded text-left text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="inspect-edge-other"
          >
            The other way: {to} → {from} · {edgeWords(other, to, from).count}
          </button>
        )}
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
          // On the panel's gutter, as every section's text (it sat 8 px left).
          <div className="min-w-0" data-canvas-scroll>
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
function StatusPill({ pill }: { pill: NonNullable<InspectorHead["pill"]> }): JSX.Element {
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

/** The inspector's head: what is selected, its kind and App, its status;
 * Done or Pool back. Setting up with nothing selected, it's for screen
 * readers and focus only (the checklist's title heads the panel). */
function HeadOfInspector({ head, member, kind, quiet, linked, heading, onBack }: { head: InspectorHead; member: MemberNode | undefined; kind: Selection["kind"]; quiet: boolean; linked: boolean; heading: ForwardedRef<HTMLHeadingElement>; onBack: () => void }): JSX.Element {
  return (
    <div className={(quiet ? "sr-only " : "flex shrink-0 items-start gap-2.5 px-4 pb-3 pt-4 ") + (linked ? SELECTED_EDGE : "")} data-testid="inspector-head">
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
      {kind !== "none" && (
        <button type="button" onClick={onBack} className="shrink-0 rounded-md px-1.5 py-0.5 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="inspector-back">
          {kind === "pool" ? "Done" : "Pool"}
        </button>
      )}
    </div>
  );
}

export const Inspector = forwardRef<HTMLHeadingElement, {
  pool: PoolInfo;
  project: ProjectInfo | undefined;
  canvas: PoolCanvasInfo | null;
  members: MemberNode[];
  /** Its sessions open now, as its header counts them. */
  open: readonly InstanceInfo[];
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
  /** D-126 §3: setting up (the checklist) or running (calm), how far the
   * setup is, and Hide setup. */
  phase: SetupPhase;
  progress: SetupProgress;
  hidden: boolean;
  onHide: () => void;
  /** Edit setup: the checklist again. */
  onEdit: () => void;
  /** The App whose next session leads, by name. */
  leadNext: string | null;
}>(function Inspector({ pool, project, canvas, members, open, names, selection, now, onSelect, onOpenTask, onDeleted, rules, ruleLabel, ruleTool, where, needs, setup, leadDir, setupSections, phase, progress, hidden, onHide, onEdit, leadNext }, heading): JSX.Element {
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
  const overview = { pool, canvas, members, open, rules, ruleLabel, where, needs, now, setup, progress, onSelect };
  // Setting up, the checklist's title heads the panel (the pool's own
  // title stays for screen readers and focus).
  const quietHead = selection.kind === "none" && phase === "setup";
  return (
    <aside id="pool-inspector" aria-labelledby="pool-inspector-title" style={{ width }} className="relative flex h-full min-h-0 shrink-0 flex-col bg-surface-2/40 shadow-[inset_1px_0_0_var(--color-divider)]" data-testid="pool-inspector" data-selected={of.kind}>
      {/* Round 5 (under D-55): wider, and resizable; its width is kept. */}
      <Splitter edge={INSPECTOR_EDGE} width={width} onWidth={setWidth} controls="pool-inspector" />
      <HeadOfInspector head={head} member={member} kind={selection.kind} quiet={quietHead} linked={linked} heading={heading} onBack={() => onSelect({ kind: "none" })} />
      {/* A new selection opens at its top (the list kept the last one's
          scroll: the pool's settings opened halfway down). */}
      {/* With nothing selected its foot is in the list's flow, under the
          items, at the panel's foot when they're short (the mock's; fixed
          over the list it hid the open item's Turn on at 1280 × 800). */}
      <ScrollEdge key={JSON.stringify(selection)} className={"flex min-h-0 flex-1 flex-col overflow-y-auto [&>*]:shrink-0 " + (selection.kind === "none" ? (quietHead ? "pt-4" : "") : "pb-6")} data-canvas-scroll data-testid="inspector-body">
        {selection.kind === "pool" ? (
          <>
            <PoolProgress {...overview} />
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
          <InspectEdge pool={pool} canvas={canvas} edgeId={selection.key} names={names} now={now} onOther={(key) => onSelect({ kind: "edge", key })} />
        ) : phase === "setup" ? (
          <SetupView {...overview} />
        ) : (
          <RunningSummary {...overview} leadNext={leadNext} />
        )}
        {selection.kind === "none" && <OverviewFoot phase={phase} progress={progress} hidden={hidden} onHide={onHide} onEdit={onEdit} onSettings={() => onSelect({ kind: "pool" })} />}
      </ScrollEdge>
    </aside>
  );
});
