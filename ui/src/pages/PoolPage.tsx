/**
 * A pool's page (redesign §5): one live canvas you watch and change in
 * place — its members as nodes, who talks to whom as edges, what waits on
 * you where it concerns — with an inspector that follows the selection.
 * Tasks and Messages open over the canvas; View as list shows the same as
 * rows. Nothing about a pool is on another page.
 *
 * Links in still land where they meant to: a Needs you card's Messages or
 * Tasks (the remembered tab, `focusThread`, `focusTask`) opens that panel;
 * Agent access opens the pool's settings in the inspector.
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";

import { Card } from "../components/primitives";
import { AgentToolsQuestion } from "../components/AgentTools";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { FOCUS_THREAD, useMessageNeeds } from "../lib/ipc/messages";
import { FOCUS_TASK, focusTask, useTaskNeeds, type TaskInfo } from "../lib/ipc/tasks";
import { needWords, poolById, poolIdOf, poolNeedCount, usePools } from "../lib/pools";
import { liveOutside, useConversations } from "../lib/conversations";
import { sessionState } from "../lib/rail-items";
import { lazyNamed } from "../lib/lazy";
import { globalId } from "../lib/nav";
import { useActionHost } from "../components/ItemMenu";
import { showToast } from "../components/Toast";
import { pageTabs, rememberTab, rememberedTab } from "./registry";
import { Canvas, type Selection } from "./pool/Canvas";
import { Inspector } from "./pool/Inspector";
import { MemberList } from "./pool/MemberList";
import { PoolHeader, type Panel } from "./pool/PoolHeader";
import { usePoolMode } from "./pool/mode";
import { addToPool, cannotJoin, nameOf } from "./pool/actions";
import { useCanvas } from "./pool/useCanvas";
import type { Point } from "./pool/layout";

/** What a pool page counts, said precisely: its members (the sessions AI
 * Juggler runs or carried in) and, beside them, its project's sessions
 * open in other apps now (the 0.2 walk: "0 sessions" while three were
 * open in the Claude app). */
export function poolCountWords(members: number, openElsewhere: number): string {
  const own = `${members} ${members === 1 ? "session" : "sessions"} in AI Juggler`;
  return openElsewhere > 0 ? `${own} · ${openElsewhere} open in other apps` : own;
}

// Tasks and Messages load only when first opened.
const PoolMessages = lazyNamed(() => import("./messages/PoolMessages"), "PoolMessages");
const PoolTasks = lazyNamed(() => import("./tasks/PoolTasks"), "PoolTasks");

/** Where a link in asked the page to open (read once, on arrival). */
function arrival(): { panel: Panel; settings: boolean } {
  const tab = rememberedTab("pool", pageTabs("pool"));
  return { panel: tab === "tasks" || tab === "messages" ? tab : null, settings: tab === "access" };
}

export function PoolPage({ poolId, sessions, projects, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, poolId);
  if (pools !== null && !pool) {
    return (
      <Card className="m-6 p-6 text-sm text-muted" data-testid="pool-gone">
        That pool isn't there any more.
      </Card>
    );
  }
  if (!pool) {
    return <div className="h-full rounded-card bg-surface-2 motion-safe:animate-pulse" role="status" aria-label="Reading the pool" data-testid="pool-page-loading" />;
  }
  return <PoolCanvasPage key={pool.id} poolId={pool.id} sessions={sessions} projects={projects} now={now} />;
}

function PoolCanvasPage({ poolId, sessions, projects, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; now: number }): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, poolId)!;
  const host = useActionHost();
  const { all } = useConversations();
  const messageNeeds = useMessageNeeds();
  const taskNeeds = useTaskNeeds();
  const data = useCanvas(poolId, sessions);
  const first = useMemo(arrival, []);
  const [panel, setPanelState] = useState<Panel>(first.panel);
  const [taskKey, setTaskKey] = useState(0);
  // Under 1100 px a pool starts as rows, unless you chose (kept per pool).
  const [mode, setMode] = usePoolMode(poolId);
  const [selection, setSelection] = useState<Selection>(first.settings ? { kind: "pool" } : { kind: "none" });
  const inspectorTitle = useRef<HTMLHeadingElement | null>(null);
  const setPanel = useCallback((next: Panel) => {
    setPanelState(next);
    rememberTab("pool", next ?? "members");
  }, []);
  // Arrived for Agent access: the settings show; the next visit is the canvas.
  useEffect(() => {
    if (first.settings) rememberTab("pool", "members");
  }, [first.settings]);
  // A link while the page is open (a Needs you card for this pool).
  useEffect(() => {
    const thread = () => setPanel("messages");
    const task = () => (setPanel("tasks"), setTaskKey((key) => key + 1));
    window.addEventListener(FOCUS_THREAD, thread);
    window.addEventListener(FOCUS_TASK, task);
    return () => {
      window.removeEventListener(FOCUS_THREAD, thread);
      window.removeEventListener(FOCUS_TASK, task);
    };
  }, [setPanel]);
  useEffect(() => {
    if (!panel) return;
    const onKey = (event: KeyboardEvent) => event.key === "Escape" && !event.defaultPrevented && setPanel(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [panel, setPanel]);

  const select = useCallback((next: Selection, focusInspector = false) => {
    setSelection(next);
    if (focusInspector) requestAnimationFrame(() => inspectorTitle.current?.focus());
  }, []);
  const names = useMemo(() => new Map(data.members.map((member) => [member.id, nameOf(member.session)])), [data.members]);
  const project = pool.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
  const candidates = useMemo(
    () => sessions.filter((session) => sessionState(session).state !== "ended" && poolIdOf(pools, session) !== pool.id && cannotJoin(pool, session) === null),
    [sessions, pools, pool],
  );
  const add = useCallback(
    (session: InstanceInfo, at: Point | null) => {
      const from = poolIdOf(pools, session);
      void addToPool(pool, session, at, data.place, from);
    },
    [pools, pool, data.place],
  );
  const openTask = useCallback(
    (task: TaskInfo) => {
      focusTask(task.id);
      setPanel("tasks");
      setTaskKey((key) => key + 1);
    },
    [setPanel],
  );
  // What needs you, counted as its Pools card counts it.
  const needs = poolNeedCount(
    pool.id,
    messageNeeds,
    taskNeeds,
    sessions.filter((session) => sessionState(session).state !== "ended" && poolIdOf(pools, session) === pool.id),
  );
  const elsewhere = project ? liveOutside(all).filter((c) => c.project_id === project.id).length : 0;
  const facts = [poolCountWords(pool.members, elsewhere), project ? `for ${project.name}` : null, needs > 0 ? needWords(needs) : null].filter(Boolean).join(" · ");
  const unplaced = (data.canvas?.asks ?? []).filter((ask) => ask.session_id === null);
  const members = data.members.map((member) => ({ session_id: member.id, name: names.get(member.id) ?? member.card.name }));

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid="pool-page">
      <PoolHeader pool={pool} facts={facts} mode={mode} panel={panel} onMode={setMode} onPanel={setPanel} />
      {/* D-30: asked once, the first time pools are used. */}
      <AgentToolsQuestion className="mx-4 mt-3" />
      {unplaced.length > 0 && (
        <p className="mx-4 mt-3 rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-meta text-ink" data-testid="pool-unplaced-asks">
          {unplaced.length === 1 ? "An agent that isn't on the canvas asks you something" : `${unplaced.length} agents that aren't on the canvas ask you something`}: answer it in Needs you, or in the List view.
        </p>
      )}
      <div className="flex min-h-0 flex-1">
        <div className="relative min-w-0 flex-1">
          {mode === "canvas" ? (
            <Canvas pool={pool} data={data} candidates={candidates} projects={projects} selection={selection} onSelect={select} onAdd={add} />
          ) : (
            <MemberList data={data} canvas={data.canvas} sessions={sessions} names={names} now={now} onSelect={(next) => select(next, true)} />
          )}
          {panel && (
            <section
              aria-label={panel === "tasks" ? "Tasks" : "Messages"}
              className="absolute inset-3 z-30 flex min-h-0 flex-col overflow-hidden rounded-xl border border-hairline bg-surface shadow-lg"
              data-testid={`pool-panel-${panel}-open`}
            >
              <div className="flex h-11 shrink-0 items-center gap-2 border-b border-hairline px-4">
                <h3 className="flex-1 text-section text-ink">{panel === "tasks" ? "Tasks" : "Messages"}</h3>
                <button type="button" onClick={() => setPanel(null)} className="rounded-md px-2 py-1 text-meta text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="pool-panel-close">
                  Back to the canvas
                </button>
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto p-4" data-canvas-scroll>
                {panel === "tasks" ? <PoolTasks key={taskKey} poolId={poolId} sessions={sessions} projects={projects} now={now} /> : <PoolMessages poolId={poolId} members={members} now={now} />}
              </div>
            </section>
          )}
        </div>
        {/* Tasks and Messages are the page's other tabs (D-55): open, they
            take the whole width — beside the inspector their list and
            their open task or thread were squeezed into a column a few
            words wide at 1280 px (the 0.3 re-render). The inspector stays
            mounted, as it was, for the canvas. */}
        <div className={panel ? "hidden" : "contents"} data-testid="pool-inspector-slot">
        <Inspector
          ref={inspectorTitle}
          pool={pool}
          project={project}
          canvas={data.canvas}
          members={data.members}
          names={names}
          selection={selection}
          now={now}
          onSelect={(next) => select(next)}
          onOpenTask={openTask}
          onDeleted={() => {
            showToast({ message: `Deleted ${pool.name}`, detail: "Its sessions are back in their projects' own pools." });
            // To the Pools screen (Home when there is none).
            if (globalId("pools")) window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "pools" } }));
            else host.openHome?.();
          }}
        />
        </div>
      </div>
    </div>
  );
}

