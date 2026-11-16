/**
 * AJ-E1704, D-30 — AI Juggler's agent tools in the sessions it starts.
 *
 * - The one-time question (TeamToolsQuestion, AJ-E3011 round 4: one
 *   component on the pool page, as New pool's step and here) until it's
 *   answered: never a pop-up.
 * - Settings › Agent access ({@link AgentToolsSettings}): the switch for
 *   every session, and behind a fold the switches per project and per
 *   profile, read only when the fold opens.
 */
import { useState, type JSX } from "react";
import { accountsList } from "../lib/ipc/accounts";
import { errorText } from "../lib/bridge";
import { projectsList } from "../lib/ipc/project-folders";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { cliAppName } from "../lib/app-name";
import { agentBridgeAnswer, agentBridgeSet, useAgentBridge, type AgentBridgeInfo, type AgentToolsScope } from "../lib/ipc/agent-bridge";
import { unavailableLine } from "../lib/team-tools";
import { TeamToolsQuestion, useToolsSave } from "../pages/pool/setup/TeamTools";
import { Card } from "./primitives";
import { SettingSwitch } from "./SettingSwitch";

/** Settings › Agent access: the question until it's answered, then the
 * switches. */
export function AgentToolsSettings(): JSX.Element {
  const info = useAgentBridge();
  const { error, busy, run } = useToolsSave();
  if (info === null) {
    return (
      <Card className="p-5 text-xs text-muted" data-testid="settings-agent-tools">
        Loading…
      </Card>
    );
  }
  // Unable to run here, there's nothing to ask: the switch says why.
  if (!info.asked && info.available) {
    return (
      <div data-testid="settings-agent-tools">
        {/* D-30's one question, the pool page's and New pool's own (round 4). */}
        <TeamToolsQuestion testId="settings-team-tools" heading={4} />
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
        // Never a way to turn on what can't run (AJ-E3013).
        disabled={busy || (!info.on && !info.available)}
        note={unavailableLine(info)}
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

/** A project's or an App's line follows its switch (the rc19 walk: an
 * unchecked project still said its sessions get them). */
const projectLine = (on: boolean) => (on ? "Its sessions get the agent tools." : "Its sessions don't get the agent tools.");
const appLine = (on: boolean) => (on ? "Sessions in this app get the agent tools." : "Sessions in this app don't get the agent tools.");

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
      {rows === null && !error && <p className="mt-2 text-xs text-ink-2">Loading…</p>}
      {rows !== null && (
        <div className="mt-2.5 flex flex-col gap-3">
          <Group title="Projects" empty="No projects yet.">
            {rows.projects.map((project) => (
              <SettingSwitch
                key={project.id}
                id={`agent-tools-project-${project.id}`}
                label={project.name}
                hint={projectLine(!info.off_projects.includes(project.id))}
                checked={!info.off_projects.includes(project.id)}
                disabled={busy}
                onChange={(on) => toggle("project", project.id, on)}
              />
            ))}
          </Group>
          <Group title="Apps" empty="No apps yet.">
            {rows.profiles.map((profile) => (
              <SettingSwitch
                key={profile.id}
                id={`agent-tools-profile-${profile.id}`}
                label={cliAppName(profile.provider, profile.label)}
                hint={appLine(!info.off_profiles.includes(profile.id))}
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
      {children.length === 0 ? <p className="text-xs text-ink-2">{empty}</p> : children}
    </div>
  );
}
