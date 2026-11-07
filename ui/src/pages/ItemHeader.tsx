/**
 * AJ-E1533 — each item type's identity header on its page (pages/
 * PageShell's PageHeader): its mark and name, its parent as the crumb,
 * its state, two or three facts, and its actions with its ⋯ menu.
 */
import { keysOf } from "../lib/shortcuts";
import { focusHandover } from "../lib/handover-focus";
import type { ReactNode } from "react";
import { PageHeader, type Crumb } from "./PageShell";
import type { PageKind } from "./registry";
import { ProjectFacts } from "./ProjectPage";
import { Button, Chip, type ChipTone } from "../components/primitives";
import { STATE_WORDS, isActiveStatus, lowLeftWords, stateChip } from "../components/indicator";
import { conversationWorking, openWords, sessionWorking, type ProjectOpen } from "../lib/project-summary";
import { ProjectMark } from "../components/ProjectMark";
import { ToolMark } from "../components/ToolMark";
import { InspectorHeaderMenu } from "../components/InspectorHeaderMenu";
import { ResumeButton } from "./session/ResumeButton";
import { useActionHost } from "../components/ItemMenu";
import { useSessionTitle } from "../components/ManageWork";
import { freshnessText } from "../components/RemoteTabs";
import { IconFleet, IconJuggler, IconProjects, IconTerminal } from "../components/icons";
import type { InspectorData, InspectorKind } from "../components/RightInspector";
import { outside, useConversations } from "../lib/conversations";
import { resourceLine, useHeavyNote } from "../lib/machine";
import { since } from "../lib/time-words";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { accountOfProfile, useSubscriptions } from "../lib/subscriptions";
import { GoToButton, SessionChips, SessionMark, SessionWhere } from "./session/SessionHead";
import { fromInstance, sessionAppName } from "./session/model";
import { Dot, MacDoor, SessionPoolChip, SessionToolChip } from "./session/HeaderDoors";
import { ProjectChips } from "./project/ProjectChips";

function KindIcon({ kind }: { kind: InspectorKind }) {
  const props = { width: 16, height: 16 };
  switch (kind) {
    case "instance":
      return <IconTerminal {...props} />;
    case "provider":
      return <IconJuggler {...props} />;
    case "project":
      return <IconProjects {...props} />;
    case "machine":
      return <IconFleet {...props} />;
  }
}

/** E14 (AJ-E1405): "CPU 12% · 1.1 GB · 3 helpers", amber while heavy. */
function SessionResources({ session }: { session: InstanceInfo }) {
  const heavy = useHeavyNote(session.id);
  return (
    <span className={"text-meta tabular-nums " + (heavy ? "text-warning" : "text-muted")} data-testid="session-resources">
      {resourceLine(session.cpu_pct, session.rss_bytes, session.child_count)}
      {heavy && <span className="font-sans"> — working your Mac hard</span>}
    </span>
  );
}

/** Each type's identity header: its mark and name, its parent, its
 * state, two or three facts, and its actions (the ⋯ menu included). */
export function InspectorHeader({
  page,
  kind,
  data,
  sessions,
  projects,
  accounts = [],
  now,
  primaryAction,
  profiles,
  onLaunch,
  onOpenProject,
  onStopped,
  onRemoteBack,
  onRemoteSelect,
  folded,
  onToggleFold,
}: {
  page: PageKind;
  kind: InspectorKind;
  data: InspectorData;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts?: AccountInfo[];
  now: number;
  primaryAction?: ReactNode;
  profiles: LaunchRecipeInfo[];
  onLaunch: (recipe: LaunchRecipeInfo, project: ProjectInfo) => void;
  onOpenProject: (id: string) => void;
  onStopped: () => void;
  onRemoteBack?: () => void;
  onRemoteSelect?: (instanceId: string | null) => void;
  folded: boolean;
  onToggleFold?: () => void;
}) {
  const host = useActionHost();
  const { all: conversations } = useConversations();
  const subscriptions = useSubscriptions();
  const remote = data.remote ?? null;
  const instance = data.instance ?? null;
  const shownTitle = useSessionTitle(instance);

  if (remote) {
    const crumbs: Crumb[] = [{ label: "Fleet", onOpen: onRemoteBack }];
    if (remote.instance) crumbs.push({ label: remote.machine.label, onOpen: () => onRemoteSelect?.(null) });
    const state = remote.instance ? statusTone(remote.instance.status) : null;
    return (
      <PageHeader
        kind={page}
        mark={<KindIcon kind={remote.instance ? "instance" : "machine"} />}
        name={remote.instance?.account_label ?? remote.machine.label}
        crumbs={crumbs}
        chips={
          <>
            {state && <Chip tone={state.tone}>{state.label}</Chip>}
            <span data-testid="remote-machine-chip">
              <Chip tone="accent">
                <IconFleet width={10} height={10} /> {remote.machine.label}
              </Chip>
            </span>
          </>
        }
        facts={
          <span
            className={remote.machine.freshness === "live" ? "" : "text-warning"}
            aria-live="polite"
            data-testid="remote-freshness"
            data-freshness={remote.machine.freshness}
          >
            {freshnessText(remote.machine, now)} <span className="text-muted">({keysOf("back")[0]} back)</span>
          </span>
        }
        folded={folded}
        onToggleFold={onToggleFold}
      />
    );
  }

  const menu = <InspectorHeaderMenu kind={kind} data={data} onChanged={onStopped} />;
  const project = data.project ?? (instance ? (projects.find((p) => p.id === instance.project_id) ?? null) : null);

  const name =
    kind === "instance"
      ? shownTitle
      : kind === "provider"
        ? (data.provider?.name ?? "Tool")
        : kind === "project"
            ? (data.project?.name ?? "Project")
            : "Mac";
  const crumbs: Crumb[] =
    kind === "instance" && project
      ? [{ label: project.name, onOpen: () => onOpenProject(project.id), testId: "crumb-project" }]
      : kind === "project"
          ? [{ label: "Projects", onOpen: () => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "projects" } })), testId: "crumb-projects" }]
          : [];
  const state = instance && kind === "instance" ? statusTone(instance.status) : null;
  // AJ-E2108: a session's page says where it runs (its conversation as
  // the tools' files show it: where it's open, when it was viewed).
  const model =
    kind === "instance" && instance
      ? fromInstance(instance, conversations.find((c) => c.session_id === instance.id))
      : null;
  // AJ-E1534: each type its own mark shape — a session's app mark with its
  // profile's badge, a project's rounded square, an account's round disc,
  // a tool's own mark.
  const mark = model ? (
    <SessionMark model={model} />
  ) : kind === "provider" && data.provider ? (
    <ToolMark tool={data.provider.id} size={18} />
  ) : kind === "project" && data.project ? (
      <ProjectMark projectId={data.project.id} size={28} />
    ) : (
      <KindIcon kind={kind} />
    );
  // An ended session starts again from its own launch settings, in its folder.
  const relaunch =
    kind === "instance" && instance && ENDED.includes(instance.status)
      ? (profiles.find((recipe) => recipe.id === instance.launch_recipe_id) ?? null)
      : null;
  const resumable = kind === "instance" && instance !== null && ENDED.includes(instance.status);
  // The hints only where both stand, and not in a folded header.
  const hinted = resumable && relaunch !== null && !folded;
  const chips = (
    <>
      {model ? (
        <SessionChips model={model} account={subscriptions ? accountOfProfile(subscriptions, model.profileId) : undefined} now={now} />
      ) : kind === "project" && data.project ? (
        <ProjectChips projectId={data.project.id} sessions={sessions} />
      ) : (
        state && <Chip tone={state.tone}>{state.label}</Chip>
      )}
      {/* AJ-E510: re-adopted after a crash; its earlier output is gone. */}
      {kind === "instance" && instance?.adopted && (
        <span data-testid="instance-adopted-chip">
          <Chip tone="accent">↻ picked up after restart</Chip>
        </span>
      )}
      {data.machineId && data.machineId !== "this-mac" && (
        <Chip tone="accent">
          <IconFleet width={10} height={10} /> {data.machineId}
        </Chip>
      )}
    </>
  );
  const facts =
    kind === "instance" && instance ? (
      <>
        {/* 0.3: its tool and its pool, one click away; where it runs
            after its tool (the polish: it was a chip, beside a profile
            chip that said the account again). */}
        {model && <SessionToolChip tool={instance.provider} said={sessionAppName(model)} />}
        {model && (
          <>
            <SessionWhere model={model} now={now} />
            {(!data.machineId || data.machineId === "this-mac") && <MacDoor lead={<Dot />} />}
            <Dot />
          </>
        )}
        <span data-testid="session-subline">{sessionFacts(instance, now, model === null)}</span>
        {/* Once something's measured: never a line of dashes. */}
        {isActiveStatus(instance.status) && (instance.cpu_pct !== null || instance.rss_bytes !== null || instance.child_count !== null) && (
          <>
            <Dot />
            <SessionResources session={instance} />
          </>
        )}
        {model && <SessionPoolChip sessionId={instance.id} />}
      </>
    ) : kind === "project" && data.project ? (
      <ProjectFacts project={data.project} sessions={sessions} folders={accounts} />
    ) : kind === "provider" && data.provider ? (
      <span>{toolFacts(data.provider.id, sessions, conversations)}</span>
    ) : null;
  const actions = (
    <>
      {/* AJ-E2102: a session in a desktop app's window: go to that window. */}
      {model && instance && instance.kind === "desktop" && model.live && host.focusSession && (
        <GoToButton onGo={() => void host.focusSession!(instance)} why={null} />
      )}
      {/* D-25: the one-step-away move, beside its Stop; only with a conversation to carry over (the menu's rule). */}
      {kind === "instance" && instance && instance.kind !== "desktop" && instance.conversation_id !== null && (
        <Button size="sm" variant="secondary" data-testid="header-switch" onClick={() => focusHandover(instance.id)}>
          Continue in another app…
        </Button>
      )}
      {kind === "project" && data.project && host.openProjectIn && (
        <Button
          size="sm"
          variant="secondary"
          data-testid="header-open-project"
          onClick={() => void host.openProjectIn!(data.project!, host.editorName?.() ? "editor" : "finder")}
        >
          {/* The registry's own words for it (lib/actions.ts). */}
          {host.editorName?.() ? `Open in ${host.editorName()}` : "Open in Finder"}
        </Button>
      )}
      {kind === "project" && data.project && host.newSessionIn && (
        <Button size="sm" variant="secondary" data-testid="header-new-session" onClick={() => host.newSessionIn!(data.project!)}>
          New session here
        </Button>
      )}
      {kind === "provider" && host.openToolSettings && (
        <Button size="sm" variant="secondary" data-testid="header-tool-settings" onClick={() => host.openToolSettings!()}>
          Tool settings
        </Button>
      )}
      {kind === "provider" && (
        <Button size="sm" variant="secondary" data-testid="header-add-account" onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))}>
          Add app…
        </Button>
      )}
      {/* AJ-E2908: a fresh one, then the same conversation: Resume, the
          primary, ends the row (the 0.4 review, 26). Side by side, each
          says what it starts in a line under it. */}
      {relaunch && instance && (
        <Hinted hint={hinted ? "Fresh conversation" : null} testId="header-relaunch-hint">
          <Button
            size="sm"
            variant="secondary"
            title="Relaunch — fresh session, same launch settings and folder."
            data-testid="header-relaunch"
            onClick={() =>
              onLaunch(relaunch, {
                id: project?.id ?? "relaunch", root: instance.cwd ?? project?.root ?? "", name: project?.name ?? "relaunch", last_activity_ms: null,
                active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false,
              })
            }
          >
            Relaunch
          </Button>
        </Hinted>
      )}
      {resumable && instance && (
        <Hinted hint={hinted ? "Same conversation" : null} testId="header-resume-hint">
          <ResumeButton
            session={instance}
            openIn={conversations.some((c) => c.id === instance.conversation_id && c.live) ? "another app" : null}
            onOpen={host.openSessionById}
            onContinue={() => focusHandover(instance.id)}
          />
        </Hinted>
      )}
      {primaryAction ?? null}
      {menu}
    </>
  );
  return (
    <PageHeader
      kind={page}
      layout={model ? "session" : "item"}
      here={kind === "project" ? name : undefined}
      mark={mark}
      name={name}
      crumbs={crumbs}
      chips={chips}
      facts={facts}
      actions={actions}
      folded={folded}
      onToggleFold={onToggleFold}
    />
  );
}

/** An action with a one-line hint under it, in the hint's style; laid
 * out of the row's flow, so the buttons beside it keep their line. */
function Hinted({ hint, testId, children }: { hint: string | null; testId: string; children: ReactNode }) {
  if (hint === null) return <>{children}</>;
  // The line wraps to its button's width and takes no height: side by
  // side, two never print over each other, and the row gets no wider (a
  // wider one cut the App chip to "Claude Code · Work …" at 1280).
  return (
    <span className="flex flex-col items-center">
      {children}
      <span className="pointer-events-none h-0 w-0 min-w-full">
        <span className="relative top-1 block text-center text-label leading-tight text-muted" data-testid={testId}>
          {hint}
        </span>
      </span>
    </span>
  );
}

/** A session that has ended (it can be relaunched). */
const ENDED = ["finished", "stopped", "failed", "handed_over"];

/** A session's facts: how long, and how much is left once that runs low
 * (its project is the crumb above, its account the Account chip; the
 * account leads only where there's no chip for it). */
function sessionFacts(session: InstanceInfo, now: number, withAccount: boolean): string {
  return [withAccount ? session.account_label : null, `started ${since(session.started_at_ms, now)} ago`, lowLeftWords(session)]
    .filter(Boolean)
    .join(" · ");
}

/** How many of a tool's sessions run now, outside ones too (AJ-E1529). */
export function toolRunning(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): number {
  return (
    sessions.filter((s) => s.provider === toolId && isActiveStatus(s.status)).length +
    outside(conversations).filter((c) => c.tool === toolId && c.live).length
  );
}

/** A tool's sessions now, as a project's are counted: open (AI
 * Juggler's and other apps'), and working (one asking you or stuck is
 * open, not working; the 0.4 review). */
export function toolOpen(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): ProjectOpen {
  const mine = sessions.filter((s) => s.provider === toolId && isActiveStatus(s.status));
  const theirs = outside(conversations).filter((c) => c.tool === toolId && c.live);
  return { open: toolRunning(toolId, sessions, conversations), working: mine.filter(sessionWorking).length + theirs.filter(conversationWorking).length, apps: 0 };
}

/** A tool's facts in the project's words: "2 working · 5 open". */
function toolFacts(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): string {
  return openWords(toolOpen(toolId, sessions, conversations));
}


/** A session state as a chip: its tone, and the rail's words for it. */
export function statusTone(status: string): { tone: ChipTone; label: string } {
  const chip = stateChip(status);
  const tone: ChipTone =
    chip.tone === "live"
      ? "live"
      : chip.tone === "success"
        ? "success"
        : chip.tone === "warning"
          ? "warning"
          : chip.tone === "danger"
            ? "danger"
            : chip.tone === "accent"
              ? "accent"
              : "neutral";
  // One source per fact: the state in the rail's own words.
  return { tone, label: STATE_WORDS[status] ?? chip.label };
}
