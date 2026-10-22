/**
 * AJ-E1533 — each item type's identity header on its page (pages/
 * PageShell's PageHeader): its mark and name, its parent as the crumb,
 * its state, two or three facts, and its actions with its ⋯ menu.
 */
import type { ReactNode } from "react";
import { PageHeader, type Crumb } from "./PageShell";
import type { PageKind } from "./registry";
import { ProjectFacts } from "./ProjectPage";
import { Button, Chip, type ChipTone } from "../components/primitives";
import { AccountDisc, STATE_WORDS, isActiveStatus, stateChip } from "../components/indicator";
import { ToolMark } from "../components/ToolMark";
import { InspectorHeaderMenu } from "../components/InspectorHeaderMenu";
import { useActionHost } from "../components/ItemMenu";
import { sessionTitle } from "../components/ManageWork";
import { freshnessText } from "../components/RemoteTabs";
import { IconFleet, IconJuggler, IconProfiles, IconProjects, IconTerminal } from "../components/icons";
import type { InspectorData, InspectorKind } from "../components/RightInspector";
import { outside, useConversations } from "../lib/conversations";
import { resourceLine, useHeavyNote } from "../lib/machine";
import { since } from "../lib/time-words";
import { useTools } from "../lib/tools";
import { signedIn, type AccountInfo, type InstanceInfo, type LaunchRecipeInfo, type ProjectInfo } from "../lib/bridge";
import type { ConversationInfo } from "../lib/ipc/conversations";

function KindIcon({ kind }: { kind: InspectorKind }) {
  const props = { width: 16, height: 16 };
  switch (kind) {
    case "instance":
      return <IconTerminal {...props} />;
    case "provider":
      return <IconJuggler {...props} />;
    case "account":
      return <IconProfiles {...props} />;
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
    <span className={"font-mono text-[11px] " + (heavy ? "text-warning" : "text-faint")} data-testid="session-resources">
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
  const tools = useTools();
  const { all: conversations } = useConversations();
  const remote = data.remote ?? null;
  const instance = data.instance ?? null;

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
            {freshnessText(remote.machine, now)} <span className="text-faint">(⌘[ back)</span>
          </span>
        }
        folded={folded}
        onToggleFold={onToggleFold}
      />
    );
  }

  const menu = <InspectorHeaderMenu kind={kind} data={data} sessions={sessions} onChanged={onStopped} />;
  const project = data.project ?? (instance ? (projects.find((p) => p.id === instance.project_id) ?? null) : null);
  const account = data.account ?? null;
  const tool = account ? tools.find((t) => t.id === account.provider) : undefined;

  const name =
    kind === "instance"
      ? instance
        ? sessionTitle(instance)
        : "Session"
      : kind === "provider"
        ? (data.provider?.name ?? "Tool")
        : kind === "account"
          ? (account?.label ?? "Account")
          : kind === "project"
            ? (data.project?.name ?? "Project")
            : "Machine";
  const crumbs: Crumb[] =
    kind === "instance" && project
      ? [{ label: project.name, onOpen: () => onOpenProject(project.id), testId: "crumb-project" }]
      : kind === "account" && tool
        ? [{ label: tool.display_name, onOpen: () => host.openTool?.(tool), testId: "crumb-tool" }]
        : [];
  const mark =
    kind === "provider" && data.provider ? (
      <ToolMark tool={data.provider.id} size={18} />
    ) : kind === "account" && account ? (
      <AccountDisc accountId={account.id} label={account.label} size={20} />
    ) : (
      <KindIcon kind={kind} />
    );
  const state = instance && kind === "instance" ? statusTone(instance.status) : null;
  // An ended session starts again from its own launch settings, in its folder.
  const relaunch =
    kind === "instance" && instance && ENDED.includes(instance.status)
      ? (profiles.find((recipe) => recipe.id === instance.launch_recipe_id) ?? null)
      : null;
  const chips = (
    <>
      {state && <Chip tone={state.tone}>{state.label}</Chip>}
      {/* AJ-E510: re-adopted after a crash; its earlier output is gone. */}
      {kind === "instance" && instance?.adopted && (
        <span data-testid="instance-adopted-chip">
          <Chip tone="accent">↻ re-adopted</Chip>
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
        <span data-testid="session-subline">{sessionFacts(instance, now)}</span>
        {isActiveStatus(instance.status) && <SessionResources session={instance} />}
      </>
    ) : kind === "project" && data.project ? (
      <ProjectFacts project={data.project} sessions={sessions} />
    ) : kind === "account" && account ? (
      <span>{accountFacts(account, tool?.display_name ?? account.provider)}</span>
    ) : kind === "provider" && data.provider ? (
      <span>{toolFacts(data.provider.id, sessions, conversations)}</span>
    ) : null;
  const actions = (
    <>
      {/* D-25: the session's one-step-away move, beside its Stop. */}
      {kind === "instance" && instance && instance.kind !== "desktop" && (
        <Button size="sm" variant="secondary" data-testid="header-switch" onClick={() => window.dispatchEvent(new Event("aij:focus-handover"))}>
          Switch account…
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
      {kind === "provider" && (
        <Button size="sm" variant="secondary" data-testid="header-add-account" onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))}>
          Add account…
        </Button>
      )}
      {relaunch && instance && (
        <Button
          size="sm"
          variant="secondary"
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
      )}
      {primaryAction ?? null}
      {menu}
    </>
  );
  return (
    <PageHeader
      kind={page}
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

/** A session's facts: its account, how long, and how much is left once
 * that runs low (its project is the crumb above). */
function sessionFacts(session: InstanceInfo, now: number): string {
  const left = session.quota ? Math.max(0, Math.round(100 - session.quota.used_percentage)) : null;
  return [session.account_label, `started ${since(session.started_at_ms, now)} ago`, left !== null && left <= 25 ? `${left}% left` : null]
    .filter(Boolean)
    .join(" · ");
}

/** An account's facts: its tool, and whether it's signed in. */
function accountFacts(account: AccountInfo, toolName: string): string {
  return [toolName, signedIn(account.login_state) ? "signed in" : "not signed in"].join(" · ");
}

/** How many of a tool's sessions run now, outside ones too (AJ-E1529). */
export function toolRunning(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): number {
  return (
    sessions.filter((s) => s.provider === toolId && isActiveStatus(s.status)).length +
    outside(conversations).filter((c) => c.tool === toolId && c.live).length
  );
}

/** A tool's facts: how many of its sessions run now. */
function toolFacts(toolId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): string {
  const running = toolRunning(toolId, sessions, conversations);
  return running === 0 ? "Nothing running" : `${running} running`;
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
