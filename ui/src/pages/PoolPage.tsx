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
import { useAgentBridge } from "../lib/ipc/agent-bridge";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { useApps } from "../lib/apps";
import { usePoolRules } from "../lib/pool-rules";
import { useSubscriptions } from "../lib/subscriptions";
import { useTools } from "../lib/tools";
import { poolSetScope, type PoolRuleInfo } from "../lib/ipc/pool-model";
import type { PoolInfo } from "../lib/ipc/pools";
import { CanvasTeach } from "./pool/CanvasTeach";
import { PaletteMore, type MoreKind } from "./pool/PaletteMore";
import { broadCandidates, draftOf, macLabel, sessionCandidates, thisMacOf, type Candidate } from "../lib/new-pool";
import { usePairedMacs } from "../lib/ipc/fleet-launch";
import { templateById } from "../lib/pool-templates";
import { RuleNodes, ruleColumnRoom } from "./pool/RuleNodes";
import { ScopeSheet } from "./pool/ScopeSheet";
import { NewPool } from "../screens/pools/NewPool";
import { FOCUS_THREAD, useMessageNeeds } from "../lib/ipc/messages";
import { FOCUS_TASK, focusTask, useTaskNeeds, type TaskInfo } from "../lib/ipc/tasks";
import { needWords, poolById, poolIdOf, poolIdsOf, poolNeedCount, scopeChipWords, scopeLabel, setPools, usePools } from "../lib/pools";
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
import { usePoolMode } from "./pool/mode";
import { addToPool, cannotJoin, nameOf } from "./pool/actions";
import { useCanvas } from "./pool/useCanvas";
import type { Point } from "./pool/layout";
import { isActiveStatus } from "../components/indicator";

/** What a pool page counts, said precisely: its members (the sessions AI
 * Juggler runs or carried in) and, beside them, its project's sessions
 * open in other apps now (the 0.2 walk: "0 sessions" while three were
 * open in the Claude app). */
export function poolCountWords(members: number, openElsewhere: number): string {
  // Empty, with nothing open elsewhere: its canvas says so, once (design
  // review #85).
  if (members === 0 && openElsewhere === 0) return "";
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
  return <PoolCanvasPage key={pool.id} poolId={pool.id} sessions={sessions} projects={projects} accounts={accounts} now={now} />;
}

/** D-62: the palette's other candidates for `pool`, by kind, as the
 * wizard lists them (one rule for both), less the members it has. */
function useMoreCandidates(pool: PoolInfo, sessions: InstanceInfo[], projects: ProjectInfo[], accounts: AccountInfo[], members: string[]): Record<MoreKind, Candidate[]> {
  const pools = usePools();
  const tools = useTools();
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const paired = usePairedMacs();
  const { all } = useConversations();
  const bridge = useAgentBridge();
  return useMemo(() => {
    const draft = { ...draftOf(templateById("blank"), null), every: pool.scope_all, projects: pool.scope };
    const projectName = (id: string | null) => (id === null ? "No project" : (projects.find((one) => one.id === id)?.name ?? "A project"));
    const named = new Set(members);
    const own = sessionCandidates({ draft, sessions, conversations: all, pools, tools, bridge, projectName, nameOf, now: Date.now() }).filter((row) => !named.has(row.ref) && !(pools?.placements ?? []).some((place) => `session:${place.session_id}` === row.ref && place.pool_id === pool.id));
    const thisMac = thisMacOf(sessions);
    const broad = (kind: "app" | "account" | "tool" | "mac") => broadCandidates({ kind, apps: apps ?? [], subscriptions: subscriptions ?? [], tools, sessions, thisMac, paired }).filter((row) => !named.has(row.ref));
    return {
      watched: own.filter((row) => row.reach === "watched" && row.blocked === null),
      app: broad("app"),
      account: broad("account"),
      tool: broad("tool"),
      mac: broad("mac"),
      everyone: named.has("all") ? [] : [{ ref: "all", kind: "everyone", label: "Everyone in this pool's projects", detail: "every session in scope, now and later", group: "open", reach: "later", blocked: null, lastMs: 0 }],
      // Open sessions waiting to join; an ended one's place is History
      // (the P3 walk: "docs · Ended" offered Add docs, which did nothing).
      blocked: own.filter((row) => row.blocked !== null && row.group === "open"),
    };
  }, [pool, sessions, projects, members, pools, tools, bridge, apps, subscriptions, all, paired]);
}

/** D-62: a member rule's name, from the same Apps, Accounts and Tools the
 * other screens list. */
function useRuleLabel(accounts: AccountInfo[], sessions: InstanceInfo[]): (rule: PoolRuleInfo) => string {
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const tools = useTools();
  const paired = usePairedMacs();
  return useCallback(
    (rule: PoolRuleInfo) => {
      const [kind, ...rest] = rule.member.split(":");
      const id = rest.join(":");
      if (rule.member === "all") return "Everyone in this pool's projects";
      if (kind === "app") return apps?.find((app) => app.id === id)?.name ?? "An App";
      if (kind === "account") return subscriptions?.find((account) => account.key === id)?.display_name ?? "An Account";
      if (kind === "tool") return tools.find((tool) => tool.id === id)?.display_name ?? "A tool";
      if (kind === "mac") return macLabel(id, thisMacOf(sessions), paired);
      return "A member";
    },
    [apps, subscriptions, tools, sessions, paired],
  );
}

function PoolCanvasPage({ poolId, sessions, projects, accounts, now }: { poolId: string; sessions: InstanceInfo[]; projects: ProjectInfo[]; accounts: AccountInfo[]; now: number }): JSX.Element {
  const pools = usePools();
  const pool = poolById(pools, poolId)!;
  const host = useActionHost();
  const { all } = useConversations();
  const messageNeeds = useMessageNeeds();
  const taskNeeds = useTaskNeeds();
  const data = useCanvas(poolId, sessions);
  const rules = usePoolRules(poolId);
  const ruleLabel = useRuleLabel(accounts, sessions);
  const memberRules = rules.rules?.members ?? [];
  const lineRules = useMemo(() => ownLineRules(rules.rules?.talk ?? []), [rules.rules]);
  const hasRules = memberRules.some((row) => row.member !== "all" || pool.kind !== "default") || rules.watched.length > 0;
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
  const first = useMemo(arrival, []);
  const [panel, setPanelState] = useState<Panel>(first.panel);
  const [taskKey, setTaskKey] = useState(0);
  // Under 1100 px a pool starts as rows, unless you chose (kept per pool);
  // an empty one opens on the canvas, which teaches setting it up.
  const [mode, setMode] = usePoolMode(poolId, pool.members === 0 && !hasRules);
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
  const names = useMemo(() => new Map(data.members.map((member) => [member.id, member.name])), [data.members]);
  const project = pool.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
  const candidates = useMemo(
    () => sessions.filter((session) => sessionState(session).state !== "ended" && !poolIdsOf(pools, session).includes(pool.id) && cannotJoin(pool, session) === null),
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
    sessions.filter((session) => sessionState(session).state !== "ended" && poolIdsOf(pools, session).includes(pool.id)),
  );
  const elsewhere = project ? liveOutside(all).filter((c) => c.project_id === project.id).length : 0;
  const scopeWords = scopeChipWords(pool, projects);
  const facts = [poolCountWords(pool.members, elsewhere), pool.kind === "default" && project ? `for ${project.name}` : null, needs > 0 ? needWords(needs) : null].filter(Boolean).join(" · ");
  const unplaced = (data.canvas?.asks ?? []).filter((ask) => ask.session_id === null);
  const members = data.members.map((member) => ({ session_id: member.id, name: names.get(member.id) ?? member.card.name, live: isActiveStatus(member.session.status) }));

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
          onByHand={() => setTemplate(false)}
        />
      )}
      <PoolHeader pool={pool} facts={facts} scope={scopeWords === null ? undefined : { words: scopeWords, onOpen: () => setScopeOpen(true) }} mode={mode} panel={panel} onMode={setMode} onPanel={setPanel} />
      {/* D-30: asked once, the first time pools are used; never on a sealed
          pool, where "see each other" read as the opposite of sealing. */}
      {!pool.sealed && <AgentToolsQuestion className="mx-4 mt-3" quiet />}
      {unplaced.length > 0 && (
        <p className="mx-4 mt-3 rounded-lg border border-warning/40 bg-warning-soft px-3 py-2 text-meta text-ink" data-testid="pool-unplaced-asks">
          {unplaced.length === 1 ? "An agent that isn't on the canvas asks you something" : `${unplaced.length} agents that aren't on the canvas ask you something`}: answer it in Needs you, or in the List view.
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
              overlayRoom={ruleColumnRoom(memberRules, rules.watched)}
              lineRules={lineRules}
              overlay={(placeOf) => <RuleNodes rules={memberRules} watched={rules.watched} labelOf={ruleLabel} selection={selection} onSelect={(next) => select(next, true)} placeOf={placeOf} />}
              empty={(paletteOpen, openPalette) =>
                hasRules ? (
                  <p className={"pointer-events-none absolute inset-x-0 bottom-16 text-center text-meta text-ink-2" + (paletteOpen ? " pl-[16.5rem]" : "")} data-testid="canvas-rules-only">
                    Its sessions show here as they start. Click a member for its role and what it may do.{" "}
                    <button type="button" onClick={() => setTemplate(true)} className="pointer-events-auto font-medium text-accent-hi underline-offset-2 hover:underline" data-testid="canvas-rules-template">
                      Use a template
                    </button>
                  </p>
                ) : (
                <CanvasTeach
                  scope={scopeLabel(pool, projects)}
                  scopeChosen={!pool.scope_all || pool.kind === "default"}
                  onScope={() => setScopeOpen(true)}
                  onAdd={openPalette}
                  onTemplate={() => setTemplate(true)}
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

