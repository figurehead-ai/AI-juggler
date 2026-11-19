/**
 * D-62 decision 18 (AJ-E3007), AJ-E3011: New pool and a project's pool
 * open this one guided setup, in order, each step with its question and
 * one line on why: how to start (the templates, and "Start from scratch"
 * as a full card), where, who's in, who leads, who builds and who
 * checks, who may message whom, what they may do on their own (then the
 * seal), the team tools while unanswered, and a review in sentences.
 *
 * - A name and its projects make a working pool: from Who's in on, Make
 *   pool is enabled, and the foot says the rest is prefilled.
 * - Back keeps every choice; Escape and Cancel leave nothing made. Making
 *   the pool is the user's own calls in one go, then its canvas opens,
 *   its checklist ticked for the steps walked here.
 *
 * D-126: the making screens' layout (FlowLayout) — the steps beside in
 * three groups, each with its value under its name; the form; a live
 * preview of the pool's diagram matching the form on every step; the
 * actions at the foot, one filled primary. It covers the window like the
 * app's other full sheets (wireframes 1–4).
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";

import { FlowFoot, FlowLayout } from "../../components/FlowLayout";
import { IconCheck, IconChevronRight } from "../../components/icons-controls";
import { useActionHost } from "../../components/ItemMenu";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { useApps } from "../../lib/apps";
import { useConversations } from "../../lib/conversations";
import { useAgentBridge } from "../../lib/ipc/agent-bridge";
import { MEMBER_KINDS, type MemberKind } from "../../lib/ipc/pool-model";
import { broadCandidates, draftOf, freeName, makePool, sessionCandidates, startDraft, thisMacOf, type Candidate, type Draft } from "../../lib/new-pool";
import { NewPoolFrame, fitExisting, foldsTemplates, sayNotAdded, type ExistingPool } from "./NewPoolFrame";
import { usePairedMacs } from "../../lib/ipc/fleet-launch";
import { templateById, type PoolTemplate } from "../../lib/pool-templates";
import { hasUserPools, refreshPools, usePools } from "../../lib/pools";
import { asksInNewPool } from "../../lib/team-tools";
import { markSeen } from "../../lib/pool-setup";
import { shapeOfDraft } from "../../lib/pool-sentences";
import { everyoneLabel, whereWords, type SetupStep } from "../../lib/pool-words";
import { eyebrowOf, stepValue, WIZARD_HEADS } from "../../lib/pool-wizard-words";
import { sessionState } from "../../lib/rail-items";
import { useSubscriptions } from "../../lib/subscriptions";
import { useTools } from "../../lib/tools";
import { nameOf } from "../../pages/pool/actions";
import { openNowByMember } from "../../lib/pool-open-now";
import { NewPoolSteps } from "./NewPoolSteps";
import { sideFor } from "./NewPoolSide";
import { NewPoolStep } from "./NewPoolStep";
import { navOf } from "./preview-model";
import { StepEyebrow } from "./StepHead";

/** The steps, in order (AJ-E3011 §1). */
export const STEPS: readonly SetupStep[] = ["template", "where", "who", "lead", "everyone", "talk", "own", "review"];
/** With D-30's question still unanswered, it's its own step before
 * Review (round 4, main's ruling). */
export const stepsFor = (ask: boolean): readonly SetupStep[] => (ask ? [...STEPS.slice(0, -1), "tools", "review"] : STEPS);
/** From this step on, Make pool is enabled (a name and its projects make
 * a working pool). */
const MAKE_FROM = STEPS.indexOf("who");

/** The name a draft has once `template` is chosen: one the user typed
 * stays; the last template's own (or none) gives way to this one's, and
 * Blank's is none (rc7 walk: Blank read "Writer and reviewer 2"). */
export function nameOnChoose(was: Pick<Draft, "name" | "template">, template: PoolTemplate, taken: string[]): string {
  const typed = was.name.trim() !== "" && was.name !== freeName(templateById(was.template).name, taken);
  if (typed) return was.name;
  return template.id === "blank" ? "" : freeName(template.name, taken);
}

/** Where a one-project template starts: the project with the most
 * sessions open now; with none open, the one worked in last; with no
 * session, the only project there is; else `null` (the rc26 P1 walk: a
 * lab whose web sessions had all ended started on "none yet", though web
 * was its one project). */
export function busiestProject(sessions: InstanceInfo[], projects: readonly Pick<ProjectInfo, "id">[] = []): string | null {
  const counts = new Map<string, number>();
  for (const session of sessions) {
    if (session.project_id && sessionState(session).state !== "ended") counts.set(session.project_id, (counts.get(session.project_id) ?? 0) + 1);
  }
  const open = [...counts].sort((a, b) => b[1] - a[1])[0]?.[0];
  if (open) return open;
  const last = sessions
    .filter((session) => session.project_id)
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms ?? 0) - (a.last_activity_ms ?? a.started_at_ms ?? 0))[0]?.project_id;
  if (last) return last;
  return projects.length === 1 ? projects[0]!.id : null;
}

export function NewPool({
  sessions,
  projects,
  accounts,
  project,
  into,
  startWith,
  onClose,
  startAt,
  embedded = false,
  existing,
  fromProject,
}: {
  /** Opened from a project (round 6): its templates fold away behind
   * "Or start from a template". */
  fromProject?: boolean;
  /** AJ-E3011 round 5: an empty pool's own page shows this setup in
   * place (not over the window); its "Show the diagram" is `onClose`. */
  embedded?: boolean;
  /** Setting up an existing pool (`into`): its projects, its everyone
   * member's role, and whether its scope is fixed (a project's own). */
  existing?: ExistingPool;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  /** The project in view, the scope it starts with. */
  project: string | null;
  /** Set up this existing pool (an empty one's "Use a template") rather
   * than make a new one. */
  into?: { id: string; name: string };
  /** The template it starts on (a project's "Keep it apart…": sealed). */
  startWith?: string;
  onClose: () => void;
  /** The step it opens on (the canvas's checklist: the step it shows). */
  startAt?: SetupStep;
}): JSX.Element {
  const pools = usePools();
  const host = useActionHost();
  const tools = useTools();
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const conversations = useConversations().all;
  const taken = useMemo(() => (pools?.pools ?? []).map((pool) => pool.name), [pools]);
  // The project in view, else where the work is (`busiestProject`): a
  // one-project template starts there.
  const start = useMemo(() => project ?? busiestProject(sessions, projects), [project, sessions, projects]);
  const [step, setStep] = useState(() => Math.max(0, STEPS.indexOf(startAt ?? "template")));
  // The furthest step reached: the side's steps up to it are a click away.
  const [furthest, setFurthest] = useState(step);
  const go = (to: number) => {
    setStep(to);
    setFurthest((was) => Math.max(was, to));
  };
  const firstPool = useMemo(() => pools !== null && !hasUserPools(pools), [pools]);
  const [draft, setDraft] = useState<Draft>(() => {
    // AJ-E3011: Start from scratch is where a newcomer starts. A
    // project's "Keep it apart…" makes this same start (R4).
    const made = startDraft(templateById(startWith ?? "blank"), start, taken);
    return { ...fitExisting(made, existing), name: into?.name ?? made.name };
  });
  const [busy, setBusy] = useState(false);
  const dialogRef = useRef<HTMLDialogElement | null>(null);
  const primaryRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => primaryRef.current?.focus(), [step]);

  const projectName = (id: string | null) => (id === null ? "No project" : (projects.find((one) => one.id === id)?.name ?? "A project"));
  const projectNames = draft.projects.map(projectName);
  const where = whereWords(draft.every, projectNames);
  // A member's name on screen: everyone by its scope ("Everyone in web").
  const memberName = (ref: string, label: string) => (ref === "all" ? everyoneLabel(draft.every, projectNames) : label);
  // Left empty, a pool is called after its project ("web team").
  const suggestedName = into?.name ?? freeName(draft.every || projectNames.length !== 1 ? "New pool" : `${projectNames[0]} team`, taken);
  const name = draft.name.trim() || suggestedName;
  const thisMac = useMemo(() => thisMacOf(sessions), [sessions]);
  const bridge = useAgentBridge();
  // D-30's question is a step while unanswered; answering it there keeps
  // the step (it then says where they stand).
  const [answeredHere, setAnsweredHere] = useState(false);
  const steps = stepsFor(asksInNewPool(bridge, answeredHere));
  const paired = usePairedMacs();
  const candidates = useMemo(() => {
    const now = Date.now();
    const out = {} as Record<MemberKind, Candidate[]>;
    for (const kind of MEMBER_KINDS) {
      if (kind === "session") out[kind] = sessionCandidates({ draft, sessions, conversations, pools, tools, bridge, projectName, nameOf, now });
      else if (kind === "everyone") out[kind] = [{ ref: "all", kind, label: everyoneLabel(draft.every, draft.projects.map(projectName)), detail: "every session in these projects, now and later", group: "open", reach: "later", blocked: null, lastMs: 0 }];
      else out[kind] = broadCandidates({ kind, apps: apps ?? [], subscriptions: subscriptions ?? [], tools, sessions, thisMac, paired, bridge });
    }
    return out;
    // projectName reads projects, which is a dependency.
  }, [draft, sessions, conversations, pools, tools, bridge, apps, subscriptions, thisMac, paired, projects]);

  // The rc27 P1 walk: Review says what each broad member brings now, as
  // its canvas card will.
  const openNow = useMemo(() => openNowByMember(draft, sessions, { apps: apps ?? [], subscriptions: subscriptions ?? [] }), [draft, sessions, apps, subscriptions]);

  const choose = (id: string) => {
    const template = templateById(id);
    setDraft((was) => ({ ...fitExisting(draftOf(template, start), existing), members: was.members, name: into ? was.name : nameOnChoose(was, template, taken) }));
  };
  const fix = (row: Candidate) => {
    if (!row.fix) return;
    if (row.fix.action === "scope") setDraft((was) => ({ ...was, every: false, projects: [...was.projects, row.fix!.arg || null] }));
    else if (row.fix.action === "open-pool") host.openPool?.(row.fix.arg);
    else if (row.fix.action === "resume") {
      const session = sessions.find((one) => one.id === row.fix!.arg);
      if (session) void host.continueSession?.(session);
    } else {
      const [accountId, id] = row.fix.arg.split("/");
      const conversation = conversations.find((one) => one.account_id === accountId && one.id === id);
      if (conversation) void host.continueConversation?.(conversation);
    }
  };
  const make = async () => {
    if (!draft.every && draft.projects.length === 0) {
      showToast({ message: "Pick a project first", detail: "Where: only sessions in its projects can join.", tone: "warning" });
      go(steps.indexOf("where"));
      return;
    }
    setBusy(true);
    try {
      const { id, notAdded } = await makePool({ ...draft, name }, into?.id, { keepScope: existing?.fixedScope === true });
      // The canvas's checklist: the steps walked here are seen.
      markSeen(id, steps.slice(1, Math.max(step, furthest) + 1));
      await refreshPools();
      onClose();
      host.openPool?.(id);
      // Round 6: a chosen member is never dropped silently.
      void sayNotAdded(notAdded, bridge);
    } catch (cause) {
      showToast({ message: "Couldn't make the pool", detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const last = step === steps.length - 1;
  const current = steps[step]!;
  // The draft as its words read it: the steps' values and the preview's
  // rows from the same shape as the canvas checklist's.
  const shape = shapeOfDraft(draft, memberName);
  const toolsValue = answeredHere ? (bridge?.on ? "On" : "Not now") : "Asked once";
  const valueOf = (id: SetupStep) => stepValue(id, { shape, where, template: draft.template === "blank" ? "From scratch" : templateById(draft.template).name, tools: toolsValue });
  const makeWords = into ? "Set up the pool" : "Make pool";
  const prefilled = step >= MAKE_FROM && !last;
  const visited = (id: SetupStep) => steps.indexOf(id) <= furthest && steps.indexOf(id) !== step;
  const stepProjects = projects.map((one) => ({ id: one.id, name: one.name, root: one.root, open: sessions.some((session) => session.project_id === one.id && sessionState(session).state !== "ended") }));
  return (
    <NewPoolFrame embedded={embedded} dialogRef={dialogRef} onClose={onClose} title={into ? `Set up ${into.name}` : "New pool"} name={step > 0 ? name : null}>
      <StepEyebrow.Provider value={eyebrowOf(steps, current)}>
        <FlowLayout
          label={WIZARD_HEADS[current].title}
          at={current}
          steps={<NewPoolSteps steps={steps} step={step} furthest={furthest} valueOf={valueOf} onPick={go} />}
          {...sideFor({
            current, draft, name, where, shape, visited, sources: { apps: apps ?? [], sessions }, memberName, first: firstPool,
            // A preview row goes to its step once that step is reached.
            pick: { go: (row) => go(steps.indexOf(row)), can: (row) => steps.indexOf(row) >= 0 && steps.indexOf(row) <= furthest },
          })}
          foot={
            <FlowFoot
              ref={primaryRef}
              cancel={embedded ? undefined : { label: "Cancel", onClick: onClose, testId: "new-pool-cancel" }}
              back={step > 0 ? { label: "Back", onClick: () => go(step - 1), testId: "new-pool-back" } : undefined}
              secondary={prefilled ? { label: `${makeWords} now`, onClick: () => void make(), disabled: busy, testId: "new-pool-make" } : undefined}
              note={
                prefilled ? (
                  <span className="inline-flex items-center gap-2" data-testid="new-pool-prefilled">
                    <IconCheck aria-hidden="true" width={15} height={15} strokeWidth={2.4} className="shrink-0 text-accent-hi" />
                    The rest is prefilled. You can make the pool now.
                  </span>
                ) : undefined
              }
              primary={{
                label: last ? (
                  <>
                    <IconCheck aria-hidden="true" width={16} height={16} strokeWidth={2.4} className="mr-1.5" />
                    {makeWords}
                  </>
                ) : (
                  <>
                    Next: {navOf(steps[step + 1]!)}
                    <IconChevronRight aria-hidden="true" width={16} height={16} className="ml-1" />
                  </>
                ),
                onClick: last ? () => void make() : () => go(step + 1),
                disabled: busy,
                testId: "new-pool-next",
              }}
            />
          }
          testId="new-pool-flow"
        >
          <NewPoolStep
            current={current}
            apps={apps}
            draft={draft}
            setDraft={setDraft}
            name={name}
            where={where}
            memberName={memberName}
            candidates={candidates}
            sessions={sessions}
            projects={projects}
            stepProjects={stepProjects}
            openNow={openNow}
            onFix={fix}
            onChoose={choose}
            onGo={(to) => go(steps.indexOf(to))}
            onToolsAnswer={() => setAnsweredHere(true)}
            firstPool={firstPool}
            foldTemplates={foldsTemplates(fromProject, existing)}
            nameFixed={!!into}
            suggestedName={suggestedName}
            scopeFixed={existing?.fixedScope ? `Every session in ${where} is part of this team.` : undefined}
          />
        </FlowLayout>
      </StepEyebrow.Provider>
    </NewPoolFrame>
  );
}
