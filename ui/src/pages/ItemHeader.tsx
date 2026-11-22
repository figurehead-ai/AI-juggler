/**
 * AJ-E1533 — each item type's identity header on its page (pages/
 * PageHeader): its mark and name, its parent as the crumb,
 * its state, two or three facts, and its actions with its ⋯ menu.
 */
import { keysOf } from "../lib/shortcuts";
import { useOwnRoom } from "../lib/own-room-notices";
import { ownRoomWantsSwitch } from "../components/SwitchAccountButton";
import type { ReactNode } from "react";
import { PageHeader, type Crumb } from "./PageHeader";
import type { PageKind } from "./registry";
import { ProjectFacts } from "./ProjectPage";
import { Button, Chip } from "../components/primitives";
import { AskBot } from "../components/bot/BotButton";
import { WatchDoor } from "../components/watches/WatchDoor";
import { isActiveStatus, lowLeftWords } from "../components/indicator";
import { ProjectMark } from "../components/ProjectMark";
import { ToolMark } from "../components/ToolMark";
import { InspectorHeaderMenu } from "../components/InspectorHeaderMenu";
import { ResumeButton } from "./session/ResumeButton";
import { Hinted } from "./Hinted";
import { useActionHost } from "../components/ItemMenu";
import { sessionTitle } from "../components/ManageWork";
import { freshnessText } from "../components/RemoteTabs";
import { IconFleet, IconJuggler, IconProjects, IconTerminal } from "../components/icons";
import type { InspectorData, InspectorKind } from "../components/RightInspector";
import { useConversations } from "../lib/conversations";
import { resourceLine, useHeavyNote } from "../lib/machine";
import { agoWords } from "../lib/time-words";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { accountOfProfile, useSubscriptions } from "../lib/subscriptions";
import { GoToButton, SessionChips, SessionMark, SessionWhere } from "./session/SessionHead";
import { fromInstance, sessionAppName } from "./session/model";
import { Dot, MacDoor, SessionContinuesDoor, SessionPoolChip, SessionToolChip } from "./session/HeaderDoors";
import { ProjectChips } from "./project/ProjectChips";
import { statusTone, toolFacts } from "./item-header-facts";

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
  const shownTitle = instance ? sessionTitle(instance) : "Session";
  // rc33 M4: why its switch would refuse, said on its doors here too.
  // D-143: out of room or low, the side column's Switch account… is the primary.
  const ownRoom = useOwnRoom(kind === "instance" ? (instance?.account_id ?? null) : null);

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
        {/* SK-11: a switch's copy says what it continues, a door to it. */}
        <SessionContinuesDoor session={instance} lead={<Dot />} />
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
      {/* D-136: Bot, scoped to this session or project, by id. */}
      {kind === "instance" && instance && <AskBot scope={{ kind: "session", id: instance.id, name, sessions: [] }} />}
      {/* AJ-E3816: watched, or Watch… in the same spot (by its id). */}
      {kind === "instance" && instance && <WatchDoor target={{ kind: "session", id: instance.id }} name={name} />}
      {kind === "project" && data.project && <AskBot scope={{ kind: "project", id: data.project.id, name: data.project.name, sessions: [] }} />}
      {/* D-143: the switch is the side column's Switch account… (PAT-SW). */}
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
        <ResumeButton
          session={instance}
          openIn={conversations.some((c) => c.id === instance.conversation_id && c.live) ? "another app" : null}
          onOpen={host.openSessionById}
          quiet={ownRoomWantsSwitch(ownRoom)}
          onOpenOriginal={host.openConversationById}
          hint={hinted ? "Same conversation" : null}
          whyLine={!folded}
        />
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

/** A session that has ended (it can be relaunched). */
const ENDED = ["finished", "stopped", "failed", "handed_over"];

/** A session's facts: how long, and how much is left once that runs low
 * (its project is the crumb above, its account the Account chip; the
 * account leads only where there's no chip for it). */
function sessionFacts(session: InstanceInfo, now: number, withAccount: boolean): string {
  return [withAccount ? session.account_label : null, startedAgo(session.started_at_ms, now), lowLeftWords(session)]
    .filter(Boolean)
    .join(" · ");
}

/** SK-24 (the 0.4.7 persona run M2): how long ago it started, from the
 * session's own start against the window's clock (it ticks every 15 s):
 * under a minute is "just now", never "started 1s ago" of one that began
 * before the last tick. AJ-E2312 3: in the app's one time words ("started
 * 21 min ago"), as the rail and Home say it, never its own "21m ago".
 * Nothing when the start isn't known. */
function startedAgo(startedMs: number, now: number): string | null {
  const words = agoWords(startedMs, now);
  return words === "" ? null : `started ${words}`;
}
