/**
 * 0.4.6 (D-126 §3): the pool's inspector with nothing selected, in the
 * page's two phases.
 *
 * - **Setting up**: one checklist (its ring, each step's value and
 *   Change, the open one expanded as the view's one primary), and at the
 *   panel's foot what happens once a session joins, with Hide setup and
 *   all of the pool's settings.
 * - **Running**: calm — what's happening now, its tasks, its lead, what
 *   they may do on their own and its messages this week, then Live; at
 *   the foot, Edit setup (and Show setup while a hidden setup isn't done).
 *
 * The pool's own settings (selected: the pool) keep its summary in
 * sentences and the checklist as one line (`PoolProgress`).
 */
import type { JSX } from "react";

import { Alert } from "../../components/Alert";
import { IconChevronRight } from "../../components/icons-controls";
import type { PoolCanvasInfo } from "../../lib/ipc/pool-canvas";
import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { useTaskCounts } from "../../lib/ipc/tasks";
import { nextLine } from "../../lib/pool-sentences";
import { stepValues } from "../../lib/pool-values";
import type { SetupRow } from "../../lib/pool-setup";
import type { PoolRules } from "../../lib/pool-rules";
import { nextSessionWords, ownMay, pickedCount, type SETUP_STEPS } from "../../lib/pool-words";
import { sessionState } from "../../lib/rail-items";
import { toolsLeftLine, type ToolsPlace } from "../../lib/team-tools";
import { agoWords } from "../../lib/time-words";
import type { Selection } from "./Canvas";
import { PoolSummary, shapeOfPool } from "./PoolSummary";
import { Section } from "./Section";
import { FirstPrompt } from "./setup/FirstPrompt";
import { PoolChecklist } from "./setup/PoolChecklist";
import type { SetupPhase, SetupProgress } from "./setup-progress";
import { SETUP } from "./setup-words";
import { edgeKey, type MemberNode } from "./useCanvas";

type Step = (typeof SETUP_STEPS)[number];

/** The setup's checklist, opening one of its steps, and where the team
 * tools are switched (its projects and CLI Apps). */
export type InspectorSetup = { rows: readonly SetupRow[]; onOpen: (step: Step) => void; tools: { projects: readonly ToolsPlace[]; apps: readonly ToolsPlace[] } };

type OverviewProps = {
  pool: PoolInfo;
  canvas: PoolCanvasInfo | null;
  members: readonly MemberNode[];
  rules: PoolRules;
  ruleLabel: (rule: PoolRuleInfo) => string;
  where: string;
  needs: number;
  now: number;
  setup: InspectorSetup;
  progress: SetupProgress;
  onSelect: (selection: Selection) => void;
};

/** The pool's own settings: its summary in sentences, and the checklist
 * as one line while the setup isn't done. */
export function PoolProgress({ pool, rules, members, ruleLabel, where, needs, setup, progress }: OverviewProps): JSX.Element {
  const shape = shapeOfPool(pool, rules.rules, members, ruleLabel);
  return (
    <>
      <PoolSummary shape={shape} where={where} needs={needs} tools={setup.tools} onOpen={setup.onOpen} />
      <PoolChecklist rows={setup.rows} onOpen={setup.onOpen} compact ready={progress.complete} blockers={progress.blockers} tools={progress.toolsItem ? progress.tools : null} />
    </>
  );
}

/** Setting up: the one checklist, and Live once it says something. */
export function SetupView(props: OverviewProps): JSX.Element {
  const { pool, rules, members, ruleLabel, where, setup, progress } = props;
  const shape = shapeOfPool(pool, rules.rules, members, ruleLabel);
  return (
    <>
      <PoolChecklist
        title={SETUP.title(pool.name)}
        rows={setup.rows}
        onOpen={setup.onOpen}
        values={stepValues(shape, where)}
        tools={progress.toolsItem ? progress.tools : null}
        ready={progress.complete}
        next={progress.complete ? nextLine(shape, where) : null}
        toolsLeft={progress.toolsItem ? null : toolsLeftLine(progress.tools)}
        blockers={progress.blockers}
        prompt={<FirstPrompt shape={shape} where={where} testId="pool-first-prompt" />}
      />
      <Live {...props} quietWhenEmpty />
    </>
  );
}

/** "1 working · 1 asks you · 1 idle", from its open members. */
function nowWords(members: readonly MemberNode[]): string {
  const states = members.map((member) => sessionState(member.session));
  const count = (n: number, words: string) => (n > 0 ? `${n} ${words}` : null);
  const parts = [count(states.filter((one) => one.state === "working" && !one.needs).length, "working"), count(states.filter((one) => one.needs).length, "asks you"), count(states.filter((one) => one.state === "idle" && !one.needs).length, "idle")].filter(Boolean);
  return parts.join(" · ") || "Nothing open now";
}

/** "3 open · 1 in review", read with the pool's tasks. */
function useTaskWords(poolId: string): string | null {
  const counts = useTaskCounts(poolId);
  if (!counts) return null;
  const open = counts.ready + counts.todo + counts.in_progress + counts.in_review + counts.blocked;
  if (open === 0) return "None open";
  return [`${open} open`, counts.in_review > 0 ? `${counts.in_review} in review` : null, counts.blocked > 0 ? `${counts.blocked} blocked` : null].filter(Boolean).join(" · ");
}

/** "14 · last 2 min ago": the messages between its members this week
 * (the canvas's lines count the last week). */
function messageWords(canvas: PoolCanvasInfo | null, now: number): string | null {
  if (!canvas) return null;
  const sent = canvas.edges.filter((edge) => edge.messages > 0);
  const total = sent.reduce((sum, edge) => sum + edge.messages, 0);
  if (total === 0) return "None yet";
  return `${total} · last ${agoWords(Math.max(...sent.map((edge) => edge.last_at_ms)), now)}`;
}

/** Running: the pool in a few calm facts, then Live. */
export function RunningSummary(props: OverviewProps & { leadNext: string | null }): JSX.Element {
  const { pool, members, canvas, now, leadNext } = props;
  const tasks = useTaskWords(pool.id);
  const lead = members.find((member) => member.role === "lead");
  const facts: [string, string | null, string][] = [
    ["Now", nowWords(members.filter((member) => sessionState(member.session).state !== "ended")), "now"],
    ["Tasks", tasks, "tasks"],
    ["Lead", lead ? lead.name : leadNext ? nextSessionWords(leadNext) : "No lead for now", "lead"],
    ["On their own", ownMay(pool.access.level, pickedCount(pool)), "own"],
    ["Messages this week", messageWords(canvas, now), "messages"],
  ];
  return (
    <>
      <dl className="flex flex-col gap-3.5 px-4 pb-4" data-testid="pool-running">
        {facts.map(([term, value, key]) => (
          <div key={key} className="flex flex-col gap-0.5" data-fact={key}>
            <dt className="section-label">{term}</dt>
            <dd className="text-meta text-ink">{value ?? <span className="inline-block h-3.5 w-24 rounded bg-skeleton motion-safe:animate-pulse" aria-label="Reading" />}</dd>
          </div>
        ))}
      </dl>
      <Live {...props} />
    </>
  );
}

/** The pool's Live stream: shown while it has members, and for an empty
 * one (or one being set up, `quietWhenEmpty`) only once it says something
 * (who just left, P5 step 4). */
function Live({ pool, canvas, now, onSelect, quietWhenEmpty = false }: OverviewProps & { quietWhenEmpty?: boolean }): JSX.Element {
  const empty = pool.members === 0;
  const said = (canvas?.stream.length ?? 0) > 0;
  const live = quietWhenEmpty ? said : !empty || said;
  return (
    <div className="flex flex-col" data-testid="inspect-activity">
      {live && (
        <Section title="Live" testId="inspect-stream">
          {canvas === null ? (
            <output className="h-16 rounded-lg bg-skeleton motion-safe:animate-pulse" aria-label="Reading the pool's activity" />
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

/** The panel's foot: setting up, what happens once a session joins with
 * Hide setup, and all of the pool's settings; running, Edit setup (the
 * checklist again), saying how far a hidden setup got. */
export function OverviewFoot({ phase, progress, hidden, onHide, onEdit, onSettings }: { phase: SetupPhase; progress: SetupProgress; hidden: boolean; onHide: () => void; onEdit: () => void; onSettings: () => void }): JSX.Element {
  const link = "inline-flex w-fit items-center gap-1 rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60";
  return (
    <div className="mt-auto flex shrink-0 flex-col gap-3 px-4 pb-4 pt-2" data-testid="pool-overview-foot">
      {phase === "setup" && (
        <Alert level="info" live={null} className="text-label" testId="pool-setup-foot">
          {SETUP.foot}{" "}
          <button type="button" onClick={onHide} className="rounded font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="pool-setup-hide">
            {SETUP.hide}
          </button>
        </Alert>
      )}
      {phase === "running" && hidden && (
        <p className="text-meta text-ink-2" data-testid="pool-setup-hidden">
          Setup hidden: {progress.done} of {progress.total} done.
        </p>
      )}
      {phase === "setup" ? (
        <button type="button" onClick={onSettings} className={link} data-testid="inspect-open-settings">
          {SETUP.settings}
          <IconChevronRight aria-hidden="true" width={13} height={13} />
        </button>
      ) : (
        <button type="button" onClick={onEdit} className={link} data-testid="pool-edit-setup">
          {SETUP.edit}
          <IconChevronRight aria-hidden="true" width={13} height={13} />
        </button>
      )}
    </div>
  );
}
