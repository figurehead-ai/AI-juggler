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
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { usePoolRules } from "../lib/pool-rules";
import { poolSetScope } from "../lib/ipc/pool-model";
import { CanvasTeach, hasWhoIsIn } from "./pool/CanvasTeach";
import { PaletteMore } from "./pool/PaletteMore";
import { RuleNodes } from "./pool/RuleNodes";
import { TeamToolsQuestion } from "./pool/setup/TeamTools";
import { startNextLead } from "../lib/start-lead";
import { useMoreCandidates, useRuleLabel, useRuleTool } from "./pool/pool-page-reads";
import { ScopeSheet } from "./pool/ScopeSheet";
import { NewPool } from "../screens/pools/NewPool";
import { EmptyPoolSetup, SetupSkeleton } from "./pool/EmptyPoolSetup";
import type { PoolRules } from "../lib/pool-rules";
import { FOCUS_THREAD, useMessageNeeds } from "../lib/ipc/messages";
import { FOCUS_TASK, focusTask, useTaskNeeds, type TaskInfo } from "../lib/ipc/tasks";
import { needWords, poolById, poolIdOf, poolIdsOf, poolNeedCount, scopeChipWords, scopeLabel, setPools, usePools } from "../lib/pools";
import { drawnRules, everyoneOf, usePoolSetup } from "./pool/usePoolSetup";
import { desktopRecipients } from "../lib/connector";
import { useDesktopOnCanvas } from "./pool/useDesktopOnCanvas";
import { PoolSetupSections } from "./pool/PoolSetupSections";
import { liveOutside, useConversations } from "../lib/conversations";
import { sessionState } from "../lib/rail-items";
import { lazyNamed } from "../lib/lazy";
import { globalId } from "../lib/nav";
import { useActionHost } from "../components/ItemMenu";
import { showToast } from "../components/Toast";
import { errorText } from "../lib/bridge";
import { pageTabs, rememberTab, rememberedTab } from "./registry";
import { Canvas, type Selection } from "./pool/Canvas";
import { ownLineRules } from "./pool/CanvasEdges";
import { Inspector } from "./pool/Inspector";
import { MemberList } from "./pool/MemberList";
import { PoolHeader, type Panel } from "./pool/PoolHeader";
import { opensInSetup, useDiagramChosen, usePoolMode } from "./pool/mode";
import { addToPool, cannotJoin } from "./pool/actions";
import { useCanvas } from "./pool/useCanvas";
import type { Point } from "./pool/layout";
import { isActiveStatus } from "../components/indicator";

export { poolCountWords } from "./pool/pool-count";
import { poolCountWords } from "./pool/pool-count";

// Tasks and Messages load only when first opened.
const PoolMessages = lazyNamed(() => import("./messages/PoolMessages"), "PoolMessages");
const PoolTasks = lazyNamed(() => import("./tasks/PoolTasks"), "PoolTasks");

/** Where a link in asked the page to open (read once, on arrival). */
function arrival(): { panel: Panel; settings: boolean } {
  const tab = rememberedTab("pool", pageTabs("pool"));
  return { panel: tab === "tasks" || tab === "messages" ? tab : null, settings: tab === "access" };
}

export function PoolPage({ poolId, sessions, projects, accounts, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; accounts: AccountInfo[]; now: number }): JSX.Element {
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
  return <PoolGate key={pool.id} poolId={pool.id} sessions={sessions} projects={projects} accounts={accounts} now={now} />;
}

type PageProps = { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; accounts: AccountInfo[]; now: number };

/** AJ-E3011 round 5: an empty pool opens in the guided setup (New pool's
 * own steps, prefilled; "Show the diagram" leaves it); once it has
 * members, its diagram (D-55). Decided once its rules are read. */
function PoolGate(props: PageProps): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, props.poolId)!;
  const rules = usePoolRules(props.poolId);
  const [diagramChosen, chooseDiagram] = useDiagramChosen(pool.id);
  const peopled = hasWhoIsIn(pool.members, hasMemberRules(pool, rules));
  if (!peopled && rules.rules === null) return <SetupSkeleton title={`Set up ${pool.name}`} />;
  if (opensInSetup({ peopled, rulesRead: true, diagramChosen })) {
    return <EmptyPoolSetup pool={pool} rules={rules} projects={props.projects} sessions={props.sessions} accounts={props.accounts} onDiagram={chooseDiagram} />;
  }
  return <PoolCanvasPage {...props} rules={rules} />;
}

/** Whether a pool has members beyond its sessions: a rule (a project's own
 * everyone aside) or a Watched session. */
function hasMemberRules(pool: { kind: string }, rules: PoolRules): boolean {
  return (rules.rules?.members ?? []).some((row) => row.member !== "all" || pool.kind !== "default") || rules.watched.length > 0;
}

function PoolCanvasPage({ poolId, sessions, projects, accounts, now, rules }: PageProps & { rules: PoolRules }): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, poolId)!;
  const host = useActionHost();
  const { all } = useConversations();
  const messageNeeds = useMessageNeeds();
  const taskNeeds = useTaskNeeds();
  // Round 7 #7: the desktop sessions' lines are read with the canvas.
  const desktop = useDesktopOnCanvas(poolId, rules.watched);
  const data = useCanvas(poolId, sessions, desktop.refs);
  const ruleLabel = useRuleLabel(accounts, sessions, everyoneOf(pool, projects));
  const ruleTool = useRuleTool(accounts);
  const memberRules = rules.rules?.members ?? [];
  const lineRules = useMemo(() => ownLineRules(rules.rules?.talk ?? [], rules.rules?.members), [rules.rules]);
  const hasRules = hasMemberRules(pool, rules);
  // Walk K1: who's in it is its sessions too, not only what the canvas draws.
  const peopled = hasWhoIsIn(pool.members, hasRules);
  const more = useMoreCandidates(pool, sessions, projects, accounts, memberRules.map((row) => row.member));
  const [scopeOpen, setScopeOpen] = useState(false);
  // "Add docs": the project joins the scope in one click, and its sessions
  // with it (the P3 walk); a pool on every project has nothing to add.
  const addToScope = async (project: string) => {
    if (pool.scope_all || !project) return setScopeOpen(true);
    try {
      setPools(await poolSetScope(pool.id, false, [...pool.scope, project]));
      void rules.reload();
      showToast({ message: `${projects.find((one) => one.id === project)?.name ?? "The project"} is now in ${pool.name}`, detail: "Its sessions join by themselves." });
    } catch (cause) {
      showToast({ message: "Couldn't change where it works", detail: errorText(cause), tone: "error" });
    }
  };
  const [template, setTemplate] = useState(false);
  const [paletteRequest, setPaletteRequest] = useState(0);
  const first = useMemo(arrival, []);
  const [panel, setPanelState] = useState<Panel>(first.panel);
  const [taskKey, setTaskKey] = useState(0);
  // Under 1100 px a pool starts as rows, unless you chose (kept per pool);
  // an empty one opens on the canvas, which teaches setting it up.
  const [mode, setMode] = usePoolMode(poolId, !peopled);
  const [selection, setSelection] = useState<Selection>(first.settings ? { kind: "pool" } : { kind: "none" });
  const inspectorTitle = useRef<HTMLHeadingElement | null>(null);
  const setPanel = useCallback((next: Panel) => setPanelState(next), []);
  // A link's landing (Messages, Tasks, Agent access) is used once: the
  // next pool opened opens on its canvas, not on the panel this one showed
  // (the P11 walk).
  useEffect(() => {
    rememberTab("pool", "members");
  }, []);
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
  const names = useMemo(() => new Map([...desktop.names, ...data.members.map((member) => [member.id, member.name] as const)]), [data.members, desktop.names]);
  const project = pool.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
  const candidates = useMemo(
    () => sessions.filter((session) => sessionState(session).state !== "ended" && !poolIdsOf(pools, session).includes(pool.id) && cannotJoin(pool, session) === null),
    [sessions, pools, pool],
  );
  const add = useCallback(
    (session: InstanceInfo, at: Point | null) => {
      const from = poolIdOf(pools, session);
      void addToPool(pool, session, at, data.place, from, poolById(pools, from)?.name ?? null);
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
  const openHere = sessions.filter((session) => sessionState(session).state !== "ended" && poolIdsOf(pools, session).includes(pool.id));
  const needs = poolNeedCount(pool.id, messageNeeds, taskNeeds, openHere);
  const elsewhere = project ? liveOutside(all).filter((c) => c.project_id === project.id).length : 0;
  const scopeWords = scopeChipWords(pool, projects);
  // Round 5: a project's own pool says its scope here, not as a card.
  const facts = [pool.kind === "default" && project ? `Every session in ${project.name} is part of this team` : null, poolCountWords(pool.members, openHere.length, elsewhere), needs > 0 ? needWords(needs) : null].filter(Boolean).join(" · ");
  const unplaced = (data.canvas?.asks ?? []).filter((ask) => ask.session_id === null);
  // AJ-E3013: a desktop session through the connector is a recipient too.
  const members = [...data.members.map((member) => ({ session_id: member.id, name: names.get(member.id) ?? member.card.name, live: isActiveStatus(member.session.status) })), ...desktopRecipients(desktop.sessions)];

  // AJ-E3011: the checklist, in the setup's order.
  const setup = usePoolSetup({
    pool,
    projects,
    whoIn: peopled,
    rules: memberRules,
    roles: data.members.map((member) => member.role),
    info: rules.rules,
    watched: rules.watched,
    ruleLabel,
    onScope: () => setScopeOpen(true),
    onWho: () => {
      setMode("canvas");
      setPaletteRequest((n) => n + 1);
    },
    onStep: (step) => select({ kind: "pool", focus: step }),
  });
  const { where } = setup;
  const openProjects = new Set(sessions.filter((session) => sessionState(session).state !== "ended" && session.project_id).map((session) => session.project_id!));
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid="pool-page">
      {scopeOpen && <ScopeSheet pool={pool} projects={projects} open={openProjects} onClose={() => setScopeOpen(false)} />}
      {template && (
        <NewPool
          sessions={sessions}
          projects={projects}
          accounts={accounts}
          project={pool.project_id}
          into={{ id: pool.id, name: pool.name }}
          onClose={() => {
            setTemplate(false);
            rules.reload();
          }}
        />
      )}
      <PoolHeader pool={pool} facts={facts} scope={scopeWords === null ? undefined : { words: scopeWords, onOpen: () => setScopeOpen(true) }} mode={mode} panel={panel} onMode={setMode} onPanel={setPanel} />
      {/* D-30: asked once, the first time pools are used, at the top while
          unanswered (round 4); answered Not now, its quiet line with Turn
          on. Never on a sealed pool, where "see each other" read as the
          opposite of sealing. */}
      {!pool.sealed && (
        <div className="mx-4 mt-3 empty:hidden">
          <TeamToolsQuestion projects={setup.tools.projects} apps={setup.tools.apps} testId="pool-team-tools" />
        </div>
      )}
      {unplaced.length > 0 && (
        <p className="mx-4 mt-3 rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-meta text-ink" data-testid="pool-unplaced-asks">
          {unplaced.length === 1 ? "An agent that isn't on the diagram asks you something" : `${unplaced.length} agents that aren't on the diagram ask you something`}: answer it in Needs you, or in the List view.
        </p>
      )}
      <div className="flex min-h-0 flex-1">
        <div className="relative min-w-0 flex-1">
          {mode === "canvas" ? (
            <Canvas
              pool={pool}
              data={data}
              candidates={candidates}
              projects={projects}
              selection={selection}
              onSelect={select}
              onAdd={add}
              paletteRequest={paletteRequest}
              paletteMore={
                <PaletteMore
                  poolId={pool.id}
                  lists={more}
                  onAdded={rules.reload}
                  onFix={(row) => {
                    if (row.fix?.action === "scope") void addToScope(row.fix.arg);
                    else if (row.fix?.action === "open-pool") host.openPool?.(row.fix.arg);
                    else if (row.fix?.action === "resume") {
                      const session = sessions.find((one) => one.id === row.fix!.arg);
                      if (session) void host.continueSession?.(session);
                    }
                  }}
                />
              }
              overlayRoom={setup.room}
              lineRules={lineRules}
              overlay={(placeOf) => (
                <RuleNodes
                  rules={drawnRules(pool, memberRules)}
                  watched={rules.watched}
                  labelOf={ruleLabel}
                  toolOf={ruleTool}
                  selection={selection}
                  onSelect={(next) => select(next, true)}
                  placeOf={placeOf}
                  edges={data.canvas?.edges}
                  names={names}
                  lineRules={lineRules}
                  lead={setup.leadCard}
                  leadNext={setup.leadNext}
                  poolId={pool.id}
                  where={where}
                  onStartLead={(member) => startNextLead(member, setup.leadDir)}
                  poolLevel={pool.access.level}
                />
              )}
              empty={(paletteOpen, openPalette) =>
                peopled ? (
                  <p className={"pointer-events-none absolute inset-x-0 bottom-16 text-center text-meta text-ink-2" + (paletteOpen ? " pl-[16.5rem]" : "")} data-testid="canvas-rules-only">
                    {/* Walk K1: sessions in it, none open (no member to click). */}
                    {hasRules ? "Its sessions show here as they start. Click a member for its role and what it may do." : "None of its sessions is open: they show here as they start, and every one is in the List view."}{" "}
                    <button type="button" onClick={() => setTemplate(true)} className="pointer-events-auto font-medium text-accent-hi underline-offset-2 hover:underline" data-testid="canvas-rules-template">
                      Use a template
                    </button>
                  </p>
                ) : (
                <CanvasTeach
                  scope={scopeLabel(pool, projects)}
                  onAdd={openPalette}
                  own={pool.kind === "default"}
                  onSetup={() => setTemplate(true)}
                  inset={paletteOpen}
                />
                )
              }
            />
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
          rules={rules}
          ruleLabel={ruleLabel}
          ruleTool={ruleTool}
          where={where}
          needs={needs}
          setup={{ rows: setup.rows, onOpen: setup.open, tools: setup.tools }}
          leadDir={setup.leadDir}
          setupSections={
            <PoolSetupSections
              pool={pool}
              rules={rules}
              members={data.members}
              ruleLabel={ruleLabel}
              where={where}
              selection={selection}
              onAddMembers={() => setup.open("who")}
              leadDir={setup.leadDir}
            />
          }
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
