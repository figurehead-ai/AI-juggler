/**
 * AJ-E1704, D-30 — AI Juggler's agent tools in the sessions it starts.
 *
 * - The one-time question ({@link AgentToolsQuestion}), on a pool's page
 *   and in Settings › Agent access until it's answered: never a pop-up.
 * - Settings › Agent access ({@link AgentToolsSettings}): the switch for
 *   every session, and behind a fold the switches per project and per
 *   profile, read only when the fold opens.
 */
import { useState, type JSX } from "react";
import { accountsList, errorText, projectsList, type AccountInfo, type ProjectInfo } from "../lib/bridge";
import { agentBridgeAnswer, agentBridgeSet, setAgentBridge, useAgentBridge, type AgentBridgeInfo, type AgentToolsScope } from "../lib/ipc/agent-bridge";
import { Button, Card } from "./primitives";
import { SettingSwitch } from "./SettingSwitch";

export const QUESTION = "Let agents in your projects see each other and work together?";
const WHY =
  "Sessions AI Juggler starts get its agent tools: each agent can see who else is in its pool and what they're doing, and say what it's doing itself. What each pool lets agents do stays yours to set.";

function useSave(): [string | null, boolean, (work: () => Promise<AgentBridgeInfo>) => void] {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const run = (work: () => Promise<AgentBridgeInfo>) => {
    setBusy(true);
    setError(null);
    work().then(setAgentBridge, (cause) => setError(errorText(cause))).finally(() => setBusy(false));
  };
  return [error, busy, run];
}

/** The question, asked once; nothing once it's answered. */
export function AgentToolsQuestion({ className = "" }: { className?: string }): JSX.Element | null {
  const info = useAgentBridge();
  const [error, busy, run] = useSave();
  if (info === null || info.asked) return null;
  return (
    <Card className={`flex flex-col gap-2.5 p-4 ${className}`} role="region" aria-labelledby="agent-tools-question" data-testid="agent-tools-question">
      <h2 id="agent-tools-question" className="text-sm font-semibold text-ink">
        {QUESTION}
      </h2>
      <p className="text-xs leading-relaxed text-muted">
        {WHY} You can switch this off for a project or a profile in Settings › Agent access.
      </p>
      <div className="flex gap-2">
        <Button size="sm" variant="primary" disabled={busy} onClick={() => run(() => agentBridgeAnswer(true))} data-testid="agent-tools-yes">
          Yes, let them
        </Button>
        <Button size="sm" variant="secondary" disabled={busy} onClick={() => run(() => agentBridgeAnswer(false))} data-testid="agent-tools-no">
          Not now
        </Button>
      </div>
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </Card>
  );
}

/** Settings › Agent access: the question until it's answered, then the
 * switches. */
export function AgentToolsSettings(): JSX.Element {
  const info = useAgentBridge();
  const [error, busy, run] = useSave();
  if (info === null) {
    return (
      <Card className="p-5 text-xs text-muted" data-testid="settings-agent-tools">
        Loading…
      </Card>
    );
  }
  if (!info.asked) {
    return (
      <div data-testid="settings-agent-tools">
        <AgentToolsQuestion />
      </div>
    );
  }
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-agent-tools">
      <SettingSwitch
        id="agent-tools"
        label="Agent tools in the sessions AI Juggler starts"
        hint="Who's in their pool, what each is doing, and their own status line. Sessions get them, or lose them, when they next start or continue in AI Juggler."
        checked={info.on}
        disabled={busy}
        note={info.available ? null : "This build of AI Juggler doesn't include them."}
        onChange={(on) => run(() => agentBridgeAnswer(on))}
      />
      {info.on && <OffWhere info={info} busy={busy} run={run} />}
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </Card>
  );
}

/** The switches per project and profile, read when the fold opens. */
function OffWhere({ info, busy, run }: { info: AgentBridgeInfo; busy: boolean; run: (work: () => Promise<AgentBridgeInfo>) => void }): JSX.Element {
  const [rows, setRows] = useState<{ projects: ProjectInfo[]; profiles: AccountInfo[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const off = info.off_projects.length + info.off_profiles.length;
  const load = (open: boolean) => {
    if (!open || rows !== null) return;
    Promise.all([projectsList(), accountsList()]).then(
      ([projects, profiles]) => setRows({ projects: projects.filter((project) => !project.forgotten), profiles }),
      (cause) => setError(errorText(cause)),
    );
  };
  const toggle = (scope: AgentToolsScope, id: string, on: boolean) => run(() => agentBridgeSet(scope, id, on));
  return (
    <details className="group ml-5 rounded-lg border border-hairline/60 px-3 py-2" onToggle={(event) => load(event.currentTarget.open)} data-testid="agent-tools-where">
      <summary className="flex cursor-pointer select-none items-center gap-2 text-xs outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
        <span aria-hidden="true" className="text-faint transition-transform group-open:rotate-90">›</span>
        <span className="flex-1 font-medium text-ink">Where they're off</span>
        <span className="text-muted">{off === 0 ? "None" : `${off} off`}</span>
      </summary>
      {error && (
        <p className="mt-2 text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      {rows === null && !error && <p className="mt-2 text-xs text-muted">Loading…</p>}
      {rows !== null && (
        <div className="mt-2.5 flex flex-col gap-3">
          <Group title="Projects" empty="No projects yet.">
            {rows.projects.map((project) => (
              <SettingSwitch
                key={project.id}
                id={`agent-tools-project-${project.id}`}
                label={project.name}
                hint="Its sessions get the agent tools."
                checked={!info.off_projects.includes(project.id)}
                disabled={busy}
                onChange={(on) => toggle("project", project.id, on)}
              />
            ))}
          </Group>
          <Group title="Profiles" empty="No profiles yet.">
            {rows.profiles.map((profile) => (
              <SettingSwitch
                key={profile.id}
                id={`agent-tools-profile-${profile.id}`}
                label={profile.label}
                hint="Sessions on this profile get the agent tools."
                checked={!info.off_profiles.includes(profile.id)}
                disabled={busy}
                onChange={(on) => toggle("profile", profile.id, on)}
              />
            ))}
          </Group>
        </div>
      )}
    </details>
  );
}

function Group({ title, empty, children }: { title: string; empty: string; children: JSX.Element[] }): JSX.Element {
  return (
    <div className="flex flex-col gap-1.5" role="group" aria-label={title}>
      <p className="text-label font-medium uppercase tracking-wide text-faint">{title}</p>
      {children.length === 0 ? <p className="text-xs text-muted">{empty}</p> : children}
    </div>
  );
}
